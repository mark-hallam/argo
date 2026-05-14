//! `oci-distribution` backed `OciRegistry` implementation.
//!
//! Argo verticals are pushed as normal OCI images:
//! - config blob: [`BundleManifest`] JSON
//! - layers: one layer per bundled vertical file, with a media type
//!   derived from the file kind and a path annotation used to
//!   reconstruct the deterministic tar on pull
//!
//! A single legacy tar layer is still accepted on pull so older test
//! fixtures and artifacts remain readable.

use crate::bundle::{BundleEntry, BundleManifest};
use crate::oci_auth::ResolvedRegistryAuth;
use crate::registry::{OciReference, OciRegistry};
use argo_types::{ArgoError, ArgoResult};
use async_trait::async_trait;
use oci_distribution::client::{Client, ClientConfig, ClientProtocol, Config, ImageLayer};
use oci_distribution::secrets::RegistryAuth;
use oci_distribution::Reference;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap};
use std::io::{Cursor, Read};

/// OCI image manifest media type.
pub const MANIFEST_MEDIA_TYPE: &str = "application/vnd.oci.image.manifest.v1+json";
/// Argo vertical bundle manifest config media type.
pub const CONFIG_MEDIA_TYPE: &str = "application/vnd.skycot.vertical.manifest.v1+json";
/// Legacy single-tar layer media type accepted for backward compatibility.
pub const LAYER_MEDIA_TYPE: &str = "application/vnd.argo.bundle.tar.v1";
/// TOML vertical file layer media type.
pub const TOML_LAYER_MEDIA_TYPE: &str = "application/vnd.skycot.vertical.v1+toml";
/// Markdown vertical file layer media type.
pub const MARKDOWN_LAYER_MEDIA_TYPE: &str = "text/markdown; charset=utf-8";
/// JSON vertical file layer media type.
pub const JSON_LAYER_MEDIA_TYPE: &str = "application/json";
/// Fallback vertical file layer media type.
pub const OCTET_LAYER_MEDIA_TYPE: &str = "application/octet-stream";
/// OCI descriptor annotation carrying the vertical-relative path.
pub const LAYER_PATH_ANNOTATION: &str = "org.opencontainers.image.title";

/// HTTP-backed registry. `base_url` controls whether the underlying
/// OCI client speaks HTTP or HTTPS; the registry host still comes
/// from the [`OciReference`].
#[derive(Debug, Clone)]
pub struct OciDistributionRegistry {
    base_url: String,
    auth: ResolvedRegistryAuth,
}

impl OciDistributionRegistry {
    /// Build a registry pointing at `base_url`. `auth_token` is the
    /// highest-precedence token path used by older callers.
    #[must_use]
    pub fn new(base_url: &str, auth_token: Option<String>) -> Self {
        let auth = auth_token
            .map(ResolvedRegistryAuth::token)
            .unwrap_or_else(ResolvedRegistryAuth::anonymous);
        Self::with_resolved_auth(base_url, auth)
    }

    /// Build a registry with a fully resolved auth fallback result.
    #[must_use]
    pub fn with_resolved_auth(base_url: &str, auth: ResolvedRegistryAuth) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            auth,
        }
    }

    /// Configure HTTP Basic auth for bearer-challenge registries.
    #[must_use]
    pub fn with_basic_auth(mut self, user: impl Into<String>, password: impl Into<String>) -> Self {
        self.auth = ResolvedRegistryAuth::basic(user, password);
        self
    }

    fn client(&self) -> Client {
        let protocol = if self.base_url.starts_with("http://") {
            ClientProtocol::Http
        } else {
            ClientProtocol::Https
        };
        Client::new(ClientConfig {
            protocol,
            ..Default::default()
        })
    }

    fn registry_auth(&self) -> RegistryAuth {
        if let Some((user, password)) = self.auth.basic.as_ref() {
            return RegistryAuth::Basic(user.clone(), password.clone());
        }
        if let Some(token) = self.auth.token.as_ref() {
            return RegistryAuth::Basic("argo".into(), token.clone());
        }
        RegistryAuth::Anonymous
    }

    fn reference(reference: &OciReference) -> Reference {
        Reference::with_tag(
            reference.host.clone(),
            reference.repo.clone(),
            reference.tag.clone(),
        )
    }
}

/// OCI image-manifest descriptor. Kept public for callers/tests that
/// inspect manifest wire shape.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OciDescriptor {
    /// Registered media type.
    #[serde(rename = "mediaType")]
    pub media_type: String,
    /// `sha256:<hex>` blob address.
    pub digest: String,
    /// Blob size in bytes.
    pub size: u64,
}

/// Minimal OCI image manifest shape used by older tests.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OciImageManifest {
    /// OCI schema version.
    #[serde(rename = "schemaVersion")]
    pub schema_version: u32,
    /// Manifest media type.
    #[serde(rename = "mediaType", skip_serializing_if = "Option::is_none")]
    pub media_type: Option<String>,
    /// Config blob descriptor.
    pub config: OciDescriptor,
    /// Layer descriptors.
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
        let image = Self::reference(reference);
        let layers = layers_from_tar(tar_bytes, manifest)?;
        let config_bytes = serde_json::to_vec(manifest)
            .map_err(|e| ArgoError::Provider(format!("serialize bundle manifest: {e}")))?;
        let config = Config::new(config_bytes, CONFIG_MEDIA_TYPE.to_string(), None);
        self.client()
            .push(&image, &layers, config, &self.registry_auth(), None)
            .await
            .map_err(|e| ArgoError::Provider(format!("oci push: {e}")))?;
        Ok(())
    }

    async fn pull(&self, reference: &OciReference) -> ArgoResult<(Vec<u8>, BundleManifest)> {
        let image = Self::reference(reference);
        let data = self
            .client()
            .pull(&image, &self.registry_auth(), accepted_layer_media_types())
            .await
            .map_err(|e| ArgoError::Provider(format!("oci pull manifest: {e}")))?;
        let manifest: BundleManifest = serde_json::from_slice(&data.config.data)
            .map_err(|e| ArgoError::Provider(format!("parse bundle manifest: {e}")))?;
        let tar_bytes = tar_from_layers(data.layers, &manifest)?;
        Ok((tar_bytes, manifest))
    }
}

fn accepted_layer_media_types() -> Vec<&'static str> {
    vec![
        TOML_LAYER_MEDIA_TYPE,
        MARKDOWN_LAYER_MEDIA_TYPE,
        JSON_LAYER_MEDIA_TYPE,
        OCTET_LAYER_MEDIA_TYPE,
        LAYER_MEDIA_TYPE,
    ]
}

fn layers_from_tar(tar_bytes: &[u8], manifest: &BundleManifest) -> ArgoResult<Vec<ImageLayer>> {
    let files = match tar_entries(tar_bytes) {
        Ok(files) => files,
        Err(_) => return Ok(vec![legacy_tar_layer(tar_bytes.to_vec())]),
    };
    let mut layers = Vec::with_capacity(manifest.entries.len());
    for entry in &manifest.entries {
        let body = files
            .get(&entry.path)
            .ok_or_else(|| ArgoError::Provider(format!("vertical tar missing {}", entry.path)))?;
        verify_entry(entry, body)?;
        let mut annotations = HashMap::new();
        annotations.insert(LAYER_PATH_ANNOTATION.to_string(), entry.path.clone());
        layers.push(ImageLayer::new(
            body.clone(),
            media_type_for_path(&entry.path).to_string(),
            Some(annotations),
        ));
    }
    Ok(layers)
}

fn legacy_tar_layer(tar_bytes: Vec<u8>) -> ImageLayer {
    ImageLayer::new(tar_bytes, LAYER_MEDIA_TYPE.to_string(), None)
}

fn tar_entries(tar_bytes: &[u8]) -> ArgoResult<BTreeMap<String, Vec<u8>>> {
    let mut archive = tar::Archive::new(Cursor::new(tar_bytes));
    let mut files = BTreeMap::new();
    let entries = archive
        .entries()
        .map_err(|e| ArgoError::Io(format!("read tar entries: {e}")))?;
    for entry in entries {
        let mut entry = entry.map_err(|e| ArgoError::Provider(format!("tar entry: {e}")))?;
        if !entry.header().entry_type().is_file() {
            continue;
        }
        let path = entry
            .path()
            .map_err(|e| ArgoError::Provider(format!("tar entry path: {e}")))?
            .to_string_lossy()
            .replace('\\', "/");
        let mut body = Vec::new();
        entry
            .read_to_end(&mut body)
            .map_err(|e| ArgoError::Io(format!("read tar body: {e}")))?;
        files.insert(path, body);
    }
    Ok(files)
}

fn tar_from_layers(layers: Vec<ImageLayer>, manifest: &BundleManifest) -> ArgoResult<Vec<u8>> {
    if layers.len() == 1 && layers[0].media_type == LAYER_MEDIA_TYPE {
        return Ok(layers[0].data.clone());
    }
    let mut by_path = BTreeMap::new();
    for layer in layers {
        let path = layer
            .annotations
            .as_ref()
            .and_then(|a| a.get(LAYER_PATH_ANNOTATION))
            .ok_or_else(|| ArgoError::Provider("oci layer missing path annotation".into()))?;
        by_path.insert(path.clone(), layer.data);
    }
    build_tar_from_entries(&by_path, &manifest.entries)
}

fn build_tar_from_entries(
    files: &BTreeMap<String, Vec<u8>>,
    entries: &[BundleEntry],
) -> ArgoResult<Vec<u8>> {
    let mut buf = Vec::new();
    {
        let mut builder = tar::Builder::new(&mut buf);
        builder.mode(tar::HeaderMode::Deterministic);
        for entry in entries {
            let body = files
                .get(&entry.path)
                .ok_or_else(|| ArgoError::Provider(format!("oci layers missing {}", entry.path)))?;
            verify_entry(entry, body)?;
            let mut header = tar::Header::new_gnu();
            header
                .set_path(&entry.path)
                .map_err(|e| ArgoError::Provider(format!("tar set_path {}: {e}", entry.path)))?;
            header.set_size(body.len() as u64);
            header.set_mode(0o644);
            header.set_mtime(0);
            header.set_uid(0);
            header.set_gid(0);
            header.set_entry_type(tar::EntryType::Regular);
            header.set_cksum();
            builder
                .append(&header, body.as_slice())
                .map_err(|e| ArgoError::Provider(format!("tar append: {e}")))?;
        }
        builder
            .finish()
            .map_err(|e| ArgoError::Provider(format!("tar finish: {e}")))?;
    }
    Ok(buf)
}

fn verify_entry(entry: &BundleEntry, body: &[u8]) -> ArgoResult<()> {
    if entry.size != body.len() as u64 {
        return Err(ArgoError::Provider(format!(
            "entry {} size mismatch: manifest={} actual={}",
            entry.path,
            entry.size,
            body.len()
        )));
    }
    let digest = sha256_hex(body);
    if entry.digest != digest && entry.digest != format!("sha256:{digest}") {
        return Err(ArgoError::Provider(format!(
            "entry {} digest mismatch",
            entry.path
        )));
    }
    Ok(())
}

fn media_type_for_path(path: &str) -> &'static str {
    if path.ends_with(".toml") {
        TOML_LAYER_MEDIA_TYPE
    } else if path.ends_with(".md") || path.ends_with(".markdown") {
        MARKDOWN_LAYER_MEDIA_TYPE
    } else if path.ends_with(".json") || path.ends_with(".jsonl") {
        JSON_LAYER_MEDIA_TYPE
    } else {
        OCTET_LAYER_MEDIA_TYPE
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    hex::encode(h.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(path: &str, body: &[u8]) -> BundleEntry {
        BundleEntry {
            path: path.into(),
            size: body.len() as u64,
            digest: sha256_hex(body),
        }
    }

    #[test]
    fn vertical_toml_layer_uses_skycot_media_type() {
        let mut files = BTreeMap::new();
        files.insert("vertical.toml".to_string(), b"[vertical]\n".to_vec());
        let entries = vec![entry("vertical.toml", b"[vertical]\n")];
        let tar = build_tar_from_entries(&files, &entries).unwrap();
        let manifest = BundleManifest {
            digest: sha256_hex(&tar),
            workspace_name: "v".into(),
            tag: "v1".into(),
            entries,
        };
        let layers = layers_from_tar(&tar, &manifest).unwrap();
        assert_eq!(layers[0].media_type, TOML_LAYER_MEDIA_TYPE);
        assert_eq!(
            layers[0]
                .annotations
                .as_ref()
                .unwrap()
                .get(LAYER_PATH_ANNOTATION)
                .map(String::as_str),
            Some("vertical.toml")
        );
    }

    #[test]
    fn typed_layers_rebuild_original_tar_digest() {
        let mut files = BTreeMap::new();
        files.insert("vertical.toml".to_string(), b"[vertical]\n".to_vec());
        files.insert("skills/post.md".to_string(), b"post".to_vec());
        let entries = vec![
            entry("skills/post.md", b"post"),
            entry("vertical.toml", b"[vertical]\n"),
        ];
        let tar = build_tar_from_entries(&files, &entries).unwrap();
        let manifest = BundleManifest {
            digest: sha256_hex(&tar),
            workspace_name: "v".into(),
            tag: "v1".into(),
            entries,
        };
        let layers = layers_from_tar(&tar, &manifest).unwrap();
        let rebuilt = tar_from_layers(layers, &manifest).unwrap();
        assert_eq!(sha256_hex(&rebuilt), manifest.digest);
    }
}
