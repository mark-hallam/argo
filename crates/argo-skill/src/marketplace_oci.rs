//! OCI registry catalog scan for marketplace skill discovery.
//!
//! Walks `<registry>/v2/_catalog` + per-repo `tags/list` + manifest
//! annotations, then hydrates a [`MarketplaceSkill`] per repo whose
//! path starts with the configured namespace. v1 is anonymous-pull
//! only; auth + cosign signature verification land via
//! `mission-skill-publish-verified-ingest`.
//!
//! Pagination beyond the first `_catalog` page is a TODO — `?n=100`
//! is the only hint v1 sends. Multi-tag handling is also v1-minimal:
//! the most-recent tag in `tags/list` wins.

use crate::marketplace::{MarketplaceError, MarketplaceSkill, MarketplaceSource};
use crate::TrustGrade;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::Path;

/// OCI annotation key carrying a Skycot trust grade label.
pub const TRUST_GRADE_ANNOTATION: &str = "org.skycot.trust_grade";

/// OCI annotation key carrying the publisher tenant id.
pub const TENANT_ANNOTATION: &str = "org.skycot.tenant";

/// Top-level marketplace config in `~/.argo/marketplace.toml`.
///
/// ```
/// use argo_skill::marketplace::oci::MarketplaceConfig;
/// let body = "[[registry]]\nurl = \"https://r.example\"\nnamespace = \"skills\"\n";
/// let cfg = MarketplaceConfig::parse_toml(body).unwrap();
/// assert_eq!(cfg.registries.len(), 1);
/// assert_eq!(cfg.registries[0].namespace, "skills");
/// ```
#[derive(Debug, Default, Clone, Deserialize)]
pub struct MarketplaceConfig {
    /// One entry per `[[registry]]` block.
    #[serde(default, rename = "registry")]
    pub registries: Vec<RegistryConfig>,
}

/// A single registry config entry.
#[derive(Debug, Clone, Deserialize)]
pub struct RegistryConfig {
    /// Base registry URL, no trailing slash, e.g. `https://skycot.cr`.
    pub url: String,
    /// Namespace prefix to filter `_catalog` results by, e.g. `skills`.
    pub namespace: String,
}

impl MarketplaceConfig {
    /// Load from `path` if it exists, else return an empty config.
    /// Empty config is intentionally a successful path so absent
    /// `~/.argo/marketplace.toml` keeps the static-only behavior.
    pub fn load_or_default(path: &Path) -> Result<Self, MarketplaceError> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let text = std::fs::read_to_string(path)
            .map_err(|e| MarketplaceError::Config(format!("read {}: {e}", path.display())))?;
        Self::parse_toml(&text)
    }

    /// Parse a TOML body into a config; used by tests and
    /// [`load_or_default`].
    pub fn parse_toml(body: &str) -> Result<Self, MarketplaceError> {
        toml::from_str(body).map_err(|e| MarketplaceError::Config(format!("parse: {e}")))
    }
}

/// Walks an OCI Distribution-spec registry catalog and turns every
/// matching repo manifest into a [`MarketplaceSkill`].
///
/// v1 limitations: anonymous-pull only (no auth header), reads only
/// the first page of `_catalog` (no `Link: rel=next` follow), and
/// surfaces only the most-recent tag per repo.
///
/// ```no_run
/// use argo_skill::marketplace::{MarketplaceSource, oci::OciRegistryScan};
/// # async fn _doc() {
/// let scan = OciRegistryScan::new("https://skycot.cr".into(), "skills".into());
/// let _ = scan.list_all().await;
/// # }
/// ```
pub struct OciRegistryScan {
    registry_url: String,
    namespace: String,
    client: reqwest::Client,
}

impl OciRegistryScan {
    /// Create a scanner against `registry_url`, filtering to repos
    /// under `<namespace>/*`.
    #[must_use]
    pub fn new(registry_url: String, namespace: String) -> Self {
        Self {
            registry_url: registry_url.trim_end_matches('/').to_string(),
            namespace,
            client: reqwest::Client::new(),
        }
    }

    async fn fetch_repo(&self, repo: &str) -> Result<Option<MarketplaceSkill>, MarketplaceError> {
        let tags_url = format!("{}/v2/{}/tags/list", self.registry_url, repo);
        let tags: TagListResponse = http_json(&self.client, &tags_url, None).await?;
        let Some(tag) = tags.tags.last().cloned() else {
            return Ok(None);
        };

        let manifest_url = format!("{}/v2/{}/manifests/{}", self.registry_url, repo, tag);
        let manifest: ManifestResponse = http_json(
            &self.client,
            &manifest_url,
            Some("application/vnd.oci.image.manifest.v1+json"),
        )
        .await?;

        Ok(Some(skill_from_annotations(
            repo,
            &tag,
            &manifest.annotations,
        )))
    }
}

#[async_trait::async_trait]
impl MarketplaceSource for OciRegistryScan {
    async fn list_all(&self) -> Result<Vec<MarketplaceSkill>, MarketplaceError> {
        let catalog_url = format!("{}/v2/_catalog?n=100", self.registry_url);
        let catalog: CatalogResponse = http_json(&self.client, &catalog_url, None).await?;
        let prefix = format!("{}/", self.namespace);
        let mut out = Vec::new();
        for repo in catalog.repositories {
            if !repo.starts_with(&prefix) {
                continue;
            }
            if let Some(skill) = self.fetch_repo(&repo).await? {
                out.push(skill);
            }
        }
        Ok(out)
    }
}

async fn http_json<T: for<'de> Deserialize<'de>>(
    client: &reqwest::Client,
    url: &str,
    accept: Option<&str>,
) -> Result<T, MarketplaceError> {
    let mut req = client.get(url);
    if let Some(value) = accept {
        req = req.header(reqwest::header::ACCEPT, value);
    }
    let resp = req.send().await.map_err(|e| MarketplaceError::Network {
        url: url.to_string(),
        reason: e.to_string(),
    })?;
    let resp = resp
        .error_for_status()
        .map_err(|e| MarketplaceError::Network {
            url: url.to_string(),
            reason: e.to_string(),
        })?;
    resp.json::<T>()
        .await
        .map_err(|e| MarketplaceError::Network {
            url: url.to_string(),
            reason: format!("parse: {e}"),
        })
}

#[derive(Deserialize)]
struct CatalogResponse {
    #[serde(default)]
    repositories: Vec<String>,
}

#[derive(Deserialize)]
struct TagListResponse {
    #[serde(default)]
    tags: Vec<String>,
}

#[derive(Deserialize)]
struct ManifestResponse {
    #[serde(default)]
    annotations: BTreeMap<String, String>,
}

fn skill_from_annotations(
    repo: &str,
    tag: &str,
    annotations: &BTreeMap<String, String>,
) -> MarketplaceSkill {
    let name = annotations
        .get("org.opencontainers.image.title")
        .cloned()
        .unwrap_or_else(|| repo.rsplit('/').next().unwrap_or(repo).to_string());
    let description = annotations
        .get("org.opencontainers.image.description")
        .cloned()
        .unwrap_or_default();
    let version = annotations
        .get("org.opencontainers.image.version")
        .cloned()
        .unwrap_or_else(|| tag.to_string());
    let last_published_at = annotations
        .get("org.opencontainers.image.created")
        .cloned()
        .unwrap_or_default();
    let trust_grade = annotations
        .get(TRUST_GRADE_ANNOTATION)
        .and_then(|s| crate::marketplace::parse_trust_grade(s).ok())
        .unwrap_or(TrustGrade::PublishedCommunity);
    let tenant = annotations.get(TENANT_ANNOTATION).cloned().unwrap_or_else(|| {
        repo.split('/')
            .next()
            .map(str::to_string)
            .unwrap_or_default()
    });
    MarketplaceSkill {
        name,
        tenant,
        description,
        version,
        trust_grade,
        last_published_at,
        oci_ref: format!("{repo}:{tag}"),
    }
}
