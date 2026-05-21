//! OCI registry catalog scan for marketplace skill discovery.
//!
//! Walks `<registry>/v2/_catalog` + per-repo `tags/list` + manifest
//! annotations, then hydrates a [`MarketplaceSkill`] per repo whose
//! path starts with the configured namespace. v1 is anonymous-pull
//! only; auth + cosign signature verification land via
//! `mission-skill-publish-verified-ingest`.
//!
//! Catalog pagination follows OCI `Link: ...; rel="next"` headers up
//! to [`DEFAULT_MAX_CATALOG_ITEMS`]. Multi-tag handling defaults to the
//! manifest with the newest `org.opencontainers.image.created` value.

use crate::marketplace::{MarketplaceError, MarketplaceSkill, MarketplaceSource};
use crate::TrustGrade;
use chrono::{DateTime, FixedOffset};
use reqwest::header;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::Path;

/// Default maximum number of catalog repositories read from one
/// registry scan.
pub const DEFAULT_MAX_CATALOG_ITEMS: usize = 1000;

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
    /// Maximum `_catalog` repositories to read per registry.
    #[serde(default = "default_max_catalog_items")]
    pub max_catalog_items: usize,
}

/// A single registry config entry.
#[derive(Debug, Clone, Deserialize)]
pub struct RegistryConfig {
    /// Base registry URL, no trailing slash, e.g. `https://skycot.cr`.
    pub url: String,
    /// Namespace prefix to filter `_catalog` results by, e.g. `skills`.
    pub namespace: String,
    /// Policy for selecting one manifest when a repo has multiple tags.
    #[serde(default)]
    pub tag_policy: MarketplaceTagPolicy,
}

/// Multi-tag selection policy for one OCI marketplace repository.
///
/// ```
/// use argo_skill::marketplace::oci::MarketplaceTagPolicy;
/// assert!(matches!(MarketplaceTagPolicy::default(), MarketplaceTagPolicy::TimestampNewest));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum MarketplaceTagPolicy {
    /// Select the tag whose manifest has the newest OCI created timestamp.
    #[default]
    TimestampNewest,
    /// Preserve the legacy deterministic behavior of choosing lexicographic last.
    AlphabeticalLast,
    /// Select a named tag exactly.
    Explicit(String),
}

const fn default_max_catalog_items() -> usize {
    DEFAULT_MAX_CATALOG_ITEMS
}

impl<'de> Deserialize<'de> for MarketplaceTagPolicy {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = String::deserialize(deserializer)?;
        Ok(match raw.as_str() {
            "timestamp_newest" => Self::TimestampNewest,
            "alphabetical_last" => Self::AlphabeticalLast,
            explicit => Self::Explicit(explicit.to_string()),
        })
    }
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
        toml::from_str(&text)
            .map_err(|e| MarketplaceError::Config(format!("parse {}: {e}", path.display())))
    }

    /// Parse a TOML body into a config; used by tests.
    pub fn parse_toml(body: &str) -> Result<Self, MarketplaceError> {
        toml::from_str(body).map_err(|e| MarketplaceError::Config(format!("parse: {e}")))
    }
}

/// Walks an OCI Distribution-spec registry catalog and turns every
/// matching repo manifest into a [`MarketplaceSkill`].
///
/// v1 limitations: anonymous-pull only (no auth header), and surfaces
/// one selected tag per repo according to [`MarketplaceTagPolicy`].
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
    max_catalog_items: usize,
    tag_policy: MarketplaceTagPolicy,
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
            max_catalog_items: DEFAULT_MAX_CATALOG_ITEMS,
            tag_policy: MarketplaceTagPolicy::default(),
            client: reqwest::Client::new(),
        }
    }

    /// Create a scanner with an explicit catalog item cap. A zero cap
    /// is normalized to the default so accidental config omissions do
    /// not hide all registry entries.
    #[must_use]
    pub fn with_max_catalog_items(
        registry_url: String,
        namespace: String,
        max_catalog_items: usize,
    ) -> Self {
        let mut scan = Self::new(registry_url, namespace);
        if max_catalog_items > 0 {
            scan.max_catalog_items = max_catalog_items;
        }
        scan
    }

    /// Create a scanner with an explicit multi-tag selection policy.
    #[must_use]
    pub fn with_tag_policy(
        registry_url: String,
        namespace: String,
        max_catalog_items: usize,
        tag_policy: MarketplaceTagPolicy,
    ) -> Self {
        let mut scan = Self::with_max_catalog_items(registry_url, namespace, max_catalog_items);
        scan.tag_policy = tag_policy;
        scan
    }

    async fn fetch_repo(&self, repo: &str) -> Result<Option<MarketplaceSkill>, MarketplaceError> {
        let tags_url = format!("{}/v2/{}/tags/list", self.registry_url, repo);
        let tags: TagListResponse = http_json(&self.client, &tags_url, None).await?;
        let Some((tag, manifest)) = self.select_tag(repo, &tags.tags).await? else {
            return Ok(None);
        };

        Ok(Some(skill_from_annotations(
            repo,
            &tag,
            &manifest.annotations,
        )))
    }

    async fn select_tag(
        &self,
        repo: &str,
        tags: &[String],
    ) -> Result<Option<(String, ManifestResponse)>, MarketplaceError> {
        match &self.tag_policy {
            MarketplaceTagPolicy::AlphabeticalLast => {
                let Some(tag) = tags.iter().max() else {
                    return Ok(None);
                };
                fetch_manifest(&self.client, &self.registry_url, repo, tag).await
            }
            MarketplaceTagPolicy::Explicit(tag) => {
                if tags.iter().any(|candidate| candidate == tag) {
                    fetch_manifest(&self.client, &self.registry_url, repo, tag).await
                } else {
                    Ok(None)
                }
            }
            MarketplaceTagPolicy::TimestampNewest => {
                let mut newest: Option<(String, ManifestResponse, Option<DateTime<FixedOffset>>)> =
                    None;
                for tag in tags {
                    let Some((tag, manifest)) =
                        fetch_manifest(&self.client, &self.registry_url, repo, tag).await?
                    else {
                        continue;
                    };
                    let created = manifest_created_at(&manifest);
                    let replace = match newest.as_ref() {
                        Some((_, _, prior)) => created > *prior,
                        None => true,
                    };
                    if replace {
                        newest = Some((tag, manifest, created));
                    }
                }
                Ok(newest.map(|(tag, manifest, _)| (tag, manifest)))
            }
        }
    }
}

#[async_trait::async_trait]
impl MarketplaceSource for OciRegistryScan {
    async fn list_all(&self) -> Result<Vec<MarketplaceSkill>, MarketplaceError> {
        let catalog_url = format!("{}/v2/_catalog?n=100", self.registry_url);
        let repositories =
            fetch_all_pages(&self.client, &catalog_url, self.max_catalog_items).await?;
        let prefix = format!("{}/", self.namespace);
        let mut out = Vec::new();
        for repo in repositories {
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

async fn fetch_all_pages(
    client: &reqwest::Client,
    first_url: &str,
    max_items: usize,
) -> Result<Vec<String>, MarketplaceError> {
    let cap = max_items.max(1);
    let mut next = Some(first_url.to_string());
    let mut repositories = Vec::new();
    while let Some(url) = next {
        if repositories.len() >= cap {
            tracing::warn!(
                target: "argo_skill::marketplace::oci",
                max_catalog_items = cap,
                "OCI catalog pagination cap reached; truncating registry results"
            );
            break;
        }
        let (catalog, link_next) = http_catalog(client, &url).await?;
        for repo in catalog.repositories {
            if repositories.len() >= cap {
                tracing::warn!(
                    target: "argo_skill::marketplace::oci",
                    max_catalog_items = cap,
                    "OCI catalog pagination cap reached; truncating registry results"
                );
                break;
            }
            repositories.push(repo);
        }
        next = link_next.and_then(|link| resolve_next_url(&url, &link));
    }
    Ok(repositories)
}

async fn http_catalog(
    client: &reqwest::Client,
    url: &str,
) -> Result<(CatalogResponse, Option<String>), MarketplaceError> {
    let resp = client
        .get(url)
        .send()
        .await
        .map_err(|e| MarketplaceError::Network {
            url: url.to_string(),
            reason: e.to_string(),
        })?
        .error_for_status()
        .map_err(|e| MarketplaceError::Network {
            url: url.to_string(),
            reason: e.to_string(),
        })?;
    let next = resp
        .headers()
        .get(header::LINK)
        .and_then(|value| value.to_str().ok())
        .and_then(parse_next_link);
    let catalog = resp
        .json::<CatalogResponse>()
        .await
        .map_err(|e| MarketplaceError::Network {
            url: url.to_string(),
            reason: format!("parse: {e}"),
        })?;
    Ok((catalog, next))
}

fn parse_next_link(header: &str) -> Option<String> {
    for link in header.split(',') {
        let mut parts = link.split(';').map(str::trim);
        let Some(target) = parts.next() else {
            continue;
        };
        let is_next = parts.any(|part| part.eq_ignore_ascii_case(r#"rel="next""#));
        if is_next {
            return target
                .strip_prefix('<')
                .and_then(|s| s.strip_suffix('>'))
                .map(str::to_string);
        }
    }
    None
}

fn resolve_next_url(current_url: &str, link: &str) -> Option<String> {
    if link.starts_with("http://") || link.starts_with("https://") {
        return Some(link.to_string());
    }
    reqwest::Url::parse(current_url)
        .ok()
        .and_then(|base| base.join(link).ok())
        .map(|url| url.to_string())
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

async fn fetch_manifest(
    client: &reqwest::Client,
    registry_url: &str,
    repo: &str,
    tag: &str,
) -> Result<Option<(String, ManifestResponse)>, MarketplaceError> {
    let manifest_url = format!("{registry_url}/v2/{repo}/manifests/{tag}");
    match http_json::<ManifestResponse>(
        client,
        &manifest_url,
        Some("application/vnd.oci.image.manifest.v1+json"),
    )
    .await
    {
        Ok(manifest) => Ok(Some((tag.to_string(), manifest))),
        Err(MarketplaceError::Network { reason, .. }) if reason.contains("404") => Ok(None),
        Err(e) => Err(e),
    }
}

fn manifest_created_at(manifest: &ManifestResponse) -> Option<DateTime<FixedOffset>> {
    manifest
        .annotations
        .get("org.opencontainers.image.created")
        .and_then(|value| DateTime::parse_from_rfc3339(value).ok())
}

#[cfg(test)]
#[path = "marketplace_oci_tests.rs"]
mod marketplace_oci_tests;

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
    let tenant = annotations
        .get(TENANT_ANNOTATION)
        .cloned()
        .unwrap_or_else(|| {
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
