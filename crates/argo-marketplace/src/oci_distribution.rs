//! HTTP-backed `OciRegistry` against the OCI Distribution Spec
//! v1 (a.k.a. Docker Registry API v2). Round-trips an Argo
//! workspace bundle against any registry that speaks the v2
//! protocol — GHCR, Docker Hub, Artifact Registry, Harbor,
//! self-hosted distribution, etc.
//!
//! Wire shape
//! ----------
//! Push:
//!   1. POST `/v2/<repo>/blobs/uploads/`         → `202` + `Location`
//!   2. PUT  `<location>?digest=sha256:<config>` → `201`  (config blob = `BundleManifest` JSON)
//!   3. POST `/v2/<repo>/blobs/uploads/`         → `202` + `Location`
//!   4. PUT  `<location>?digest=sha256:<layer>`  → `201`  (layer blob = tar bytes)
//!   5. PUT  `/v2/<repo>/manifests/<tag>`        → `201`  (OCI image manifest)
//!
//! Pull:
//!   1. GET  `/v2/<repo>/manifests/<tag>`        → 200 + manifest
//!   2. GET  `/v2/<repo>/blobs/<config_digest>`  → 200 + `BundleManifest` JSON
//!   3. GET  `/v2/<repo>/blobs/<layer_digest>`   → 200 + tar bytes
//!
//! Auth
//! ----
//! No-auth and `Bearer <token>` are supported via the optional
//! `auth_token` field on the constructor; full OAuth2 token-
//! exchange (the GHCR / Docker Hub challenge flow) is left to the
//! caller — it's a few `reqwest` lines away and would otherwise
//! pull `oauth2` into the crate.
//!
//! Examples
//! ```no_run
//! # use argo_marketplace::{BundleManifest, OciDistributionRegistry, OciReference, OciRegistry};
//! # async fn doit() -> Result<(), Box<dyn std::error::Error>> {
//! let reg = OciDistributionRegistry::new("https://ghcr.io", None);
//! let reference = OciReference::parse("ghcr.io/argo-agents/x:0.1")?;
//! let (tar_bytes, manifest) = reg.pull(&reference).await?;
//! # let _ = (tar_bytes, manifest);
//! # Ok(())
//! # }
//! ```
use crate::bundle::BundleManifest;
use crate::registry::{OciReference, OciRegistry};
use argo_types::{ArgoError, ArgoResult};
use async_trait::async_trait;
use sha2::{Digest, Sha256};

/// OCI image manifest media type used for the manifest PUT.
pub const MANIFEST_MEDIA_TYPE: &str = "application/vnd.oci.image.manifest.v1+json";
/// Custom media type for the bundle config blob (Argo bundle metadata).
pub const CONFIG_MEDIA_TYPE: &str = "application/vnd.argo.bundle.config.v1+json";
/// Custom media type for the bundle layer blob (the workspace tar).
pub const LAYER_MEDIA_TYPE: &str = "application/vnd.argo.bundle.tar.v1";

/// HTTP-backed registry. `base_url` is the registry root
/// (e.g. `https://ghcr.io`). `auth_token`, when set, is sent as
/// `Authorization: Bearer <token>` on every request.
#[derive(Debug, Clone)]
pub struct OciDistributionRegistry {
    base_url: String,
    auth_token: Option<String>,
    client: reqwest::Client,
}

impl OciDistributionRegistry {
    /// Build a registry pointing at `base_url`. Pass `None` for
    /// `auth_token` for anonymous registries (or local mocks).
    #[must_use]
    pub fn new(base_url: &str, auth_token: Option<String>) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            auth_token,
            client: reqwest::Client::new(),
        }
    }

    fn auth(&self, req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        match &self.auth_token {
            Some(t) => req.bearer_auth(t),
            None => req,
        }
    }

    /// Hash bytes under SHA-256 and return the canonical
    /// `sha256:<hex>` digest string OCI registries use as blob
    /// addresses.
    fn digest_of(bytes: &[u8]) -> String {
        let mut h = Sha256::new();
        h.update(bytes);
        format!("sha256:{}", hex::encode(h.finalize()))
    }

    /// Two-step blob upload: `POST /v2/<repo>/blobs/uploads/` to
    /// reserve a session, then `PUT <location>?digest=…` to push
    /// the bytes. Returns the digest string.
    async fn upload_blob(
        &self,
        reference: &OciReference,
        bytes: Vec<u8>,
    ) -> ArgoResult<String> {
        let digest = Self::digest_of(&bytes);
        let init_url = format!("{}/v2/{}/blobs/uploads/", self.base_url, reference.repo);
        let init = self
            .auth(self.client.post(&init_url))
            .send()
            .await
            .map_err(|e| ArgoError::Provider(format!("oci blob init: {e}")))?;
        if !matches!(init.status().as_u16(), 200..=299) {
            return Err(ArgoError::Provider(format!(
                "oci blob init {}: {}",
                init.status(),
                init.text().await.unwrap_or_default()
            )));
        }
        let location = init
            .headers()
            .get(reqwest::header::LOCATION)
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| ArgoError::Provider("oci blob init: no Location header".into()))?
            .to_string();
        let upload_url = if location.starts_with("http://") || location.starts_with("https://") {
            location
        } else {
            // Spec allows a relative Location.
            format!("{}{}", self.base_url, location)
        };
        let put_url = if upload_url.contains('?') {
            format!("{upload_url}&digest={digest}")
        } else {
            format!("{upload_url}?digest={digest}")
        };
        let put = self
            .auth(self.client.put(&put_url))
            .header(
                reqwest::header::CONTENT_TYPE,
                "application/octet-stream",
            )
            .body(bytes)
            .send()
            .await
            .map_err(|e| ArgoError::Provider(format!("oci blob put: {e}")))?;
        if !matches!(put.status().as_u16(), 200..=299) {
            return Err(ArgoError::Provider(format!(
                "oci blob put {}: {}",
                put.status(),
                put.text().await.unwrap_or_default()
            )));
        }
        Ok(digest)
    }

    async fn fetch_blob(&self, reference: &OciReference, digest: &str) -> ArgoResult<Vec<u8>> {
        let url = format!(
            "{}/v2/{}/blobs/{}",
            self.base_url, reference.repo, digest
        );
        let resp = self
            .auth(self.client.get(&url))
            .send()
            .await
            .map_err(|e| ArgoError::Provider(format!("oci blob get: {e}")))?;
        if !matches!(resp.status().as_u16(), 200..=299) {
            return Err(ArgoError::Provider(format!(
                "oci blob get {}: {}",
                resp.status(),
                resp.text().await.unwrap_or_default()
            )));
        }
        resp.bytes()
            .await
            .map(|b| b.to_vec())
            .map_err(|e| ArgoError::Provider(format!("oci blob body: {e}")))
    }
}

/// OCI image-manifest descriptor — the JSON nodes inside
/// `config` + `layers`.
///
/// Public so tests + the impl can build it; clients don't need
/// to construct one directly.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OciDescriptor {
    /// Registered media type (we use `CONFIG_MEDIA_TYPE` for the
    /// config and `LAYER_MEDIA_TYPE` for layers).
    #[serde(rename = "mediaType")]
    pub media_type: String,
    /// `sha256:<hex>` blob address.
    pub digest: String,
    /// Blob size in bytes.
    pub size: u64,
}

/// OCI image manifest body. `schemaVersion` is always 2; the
/// optional `mediaType` is included for round-trip compatibility
/// with strict registries.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OciImageManifest {
    /// OCI schema version (always 2).
    #[serde(rename = "schemaVersion")]
    pub schema_version: u32,
    /// Manifest media type — mirrors the Content-Type the
    /// manifest was PUT with.
    #[serde(rename = "mediaType", skip_serializing_if = "Option::is_none")]
    pub media_type: Option<String>,
    /// Reference to the bundle config blob.
    pub config: OciDescriptor,
    /// Reference to the bundle layer blob (always exactly one
    /// layer for Argo bundles — the workspace tar).
    pub layers: Vec<OciDescriptor>,
}

#[async_trait]
impl OciRegistry for OciDistributionRegistry {
    async fn push(
        &self,
        reference: &OciReference,
        tar_bytes: &[u8],
        manifest: &BundleManifest,
    ) -> ArgoResult<()> {
        let config_bytes = serde_json::to_vec(manifest)
            .map_err(|e| ArgoError::Provider(format!("serialize bundle manifest: {e}")))?;
        let config_digest = self
            .upload_blob(reference, config_bytes.clone())
            .await?;
        let layer_digest = self.upload_blob(reference, tar_bytes.to_vec()).await?;
        let oci_manifest = OciImageManifest {
            schema_version: 2,
            media_type: Some(MANIFEST_MEDIA_TYPE.to_string()),
            config: OciDescriptor {
                media_type: CONFIG_MEDIA_TYPE.to_string(),
                digest: config_digest,
                size: config_bytes.len() as u64,
            },
            layers: vec![OciDescriptor {
                media_type: LAYER_MEDIA_TYPE.to_string(),
                digest: layer_digest,
                size: tar_bytes.len() as u64,
            }],
        };
        let manifest_json = serde_json::to_vec(&oci_manifest).map_err(|e| {
            ArgoError::Provider(format!("serialize oci manifest: {e}"))
        })?;
        let url = format!(
            "{}/v2/{}/manifests/{}",
            self.base_url, reference.repo, reference.tag
        );
        let resp = self
            .auth(self.client.put(&url))
            .header(reqwest::header::CONTENT_TYPE, MANIFEST_MEDIA_TYPE)
            .body(manifest_json)
            .send()
            .await
            .map_err(|e| ArgoError::Provider(format!("oci manifest put: {e}")))?;
        if !matches!(resp.status().as_u16(), 200..=299) {
            return Err(ArgoError::Provider(format!(
                "oci manifest put {}: {}",
                resp.status(),
                resp.text().await.unwrap_or_default()
            )));
        }
        Ok(())
    }

    async fn pull(&self, reference: &OciReference) -> ArgoResult<(Vec<u8>, BundleManifest)> {
        let url = format!(
            "{}/v2/{}/manifests/{}",
            self.base_url, reference.repo, reference.tag
        );
        let resp = self
            .auth(self.client.get(&url))
            .header(reqwest::header::ACCEPT, MANIFEST_MEDIA_TYPE)
            .send()
            .await
            .map_err(|e| ArgoError::Provider(format!("oci manifest get: {e}")))?;
        if !matches!(resp.status().as_u16(), 200..=299) {
            return Err(ArgoError::Provider(format!(
                "oci manifest get {}: {}",
                resp.status(),
                resp.text().await.unwrap_or_default()
            )));
        }
        let oci: OciImageManifest = resp
            .json()
            .await
            .map_err(|e| ArgoError::Provider(format!("oci manifest parse: {e}")))?;
        let layer = oci
            .layers
            .first()
            .ok_or_else(|| ArgoError::Provider("oci manifest has no layers".into()))?;
        let config_bytes = self.fetch_blob(reference, &oci.config.digest).await?;
        let bundle_manifest: BundleManifest = serde_json::from_slice(&config_bytes)
            .map_err(|e| ArgoError::Provider(format!("parse bundle manifest: {e}")))?;
        let tar_bytes = self.fetch_blob(reference, &layer.digest).await?;
        Ok((tar_bytes, bundle_manifest))
    }
}
