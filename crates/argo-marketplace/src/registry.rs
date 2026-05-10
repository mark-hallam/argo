//! OCI registry seam — Phase-6.5 stub.
//!
//! `oci-distribution` is heavyweight; the actual push/pull lands
//! with the §23.3 integration-test PR (against test accounts on
//! GHCR / Docker Hub / Artifact Registry). For Phase-6.5 we ship
//! the trait + a clean stub so the CLI surface compiles and we
//! can wire the rest of the marketplace flow end-to-end.
use crate::bundle::BundleManifest;
use argo_types::{ArgoError, ArgoResult};
use async_trait::async_trait;

/// Registry reference — `<host>/<repo>:<tag>`.
///
/// ```
/// use argo_marketplace::OciReference;
/// let r = OciReference::parse("ghcr.io/argo-agents/demo:0.3.0").unwrap();
/// assert_eq!(r.host, "ghcr.io");
/// assert_eq!(r.repo, "argo-agents/demo");
/// assert_eq!(r.tag, "0.3.0");
/// assert_eq!(r.to_canonical(), "ghcr.io/argo-agents/demo:0.3.0");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OciReference {
    /// Host (e.g. `"ghcr.io"`).
    pub host: String,
    /// Repo (`<org>/<workspace-name>`).
    pub repo: String,
    /// Tag (`"0.3.0"` / `"latest"`).
    pub tag: String,
}

impl OciReference {
    /// Parse `host/repo:tag`. Defaults tag to `"latest"` when absent.
    ///
    /// ```
    /// use argo_marketplace::OciReference;
    /// assert_eq!(OciReference::parse("ghcr.io/x/y").unwrap().tag, "latest");
    /// assert!(OciReference::parse("missing-slash").is_err());
    /// ```
    pub fn parse(s: &str) -> ArgoResult<Self> {
        let (left, tag) = match s.rsplit_once(':') {
            // Avoid splitting on the host's `:port`.
            Some((l, t)) if !t.contains('/') => (l.to_string(), t.to_string()),
            _ => (s.to_string(), "latest".to_string()),
        };
        let (host, repo) = left
            .split_once('/')
            .ok_or_else(|| ArgoError::Schema(format!("OCI reference missing repo: {s}")))?;
        if host.is_empty() || repo.is_empty() {
            return Err(ArgoError::Schema(format!("invalid OCI reference: {s}")));
        }
        Ok(Self {
            host: host.to_string(),
            repo: repo.to_string(),
            tag,
        })
    }

    /// Render as `host/repo:tag`.
    #[must_use]
    pub fn to_canonical(&self) -> String {
        format!("{}/{}:{}", self.host, self.repo, self.tag)
    }
}

/// Registry abstraction.
#[async_trait]
pub trait OciRegistry: Send + Sync + std::fmt::Debug {
    /// Push a tarball + manifest to `reference`.
    async fn push(
        &self,
        reference: &OciReference,
        tar_bytes: &[u8],
        manifest: &BundleManifest,
    ) -> ArgoResult<()>;
    /// Pull a tarball + manifest from `reference`.
    async fn pull(&self, reference: &OciReference) -> ArgoResult<(Vec<u8>, BundleManifest)>;
}

/// Phase-6.5 stub.
#[derive(Debug, Default, Clone, Copy)]
pub struct StubRegistry;

#[async_trait]
impl OciRegistry for StubRegistry {
    async fn push(
        &self,
        reference: &OciReference,
        _tar_bytes: &[u8],
        _manifest: &BundleManifest,
    ) -> ArgoResult<()> {
        Err(ArgoError::Provider(format!(
            "OCI registry not yet wired (push to {}); §23.3 follow-up uses oci-distribution",
            reference.to_canonical()
        )))
    }
    async fn pull(&self, reference: &OciReference) -> ArgoResult<(Vec<u8>, BundleManifest)> {
        Err(ArgoError::Provider(format!(
            "OCI registry not yet wired (pull from {}); §23.3 follow-up uses oci-distribution",
            reference.to_canonical()
        )))
    }
}

/// In-memory `OciRegistry` for tests + the §23.3 integration suite.
/// Stores `(canonical reference → (tar bytes, manifest))` in a
/// process-local map so push/pull round-trip without a network /
/// real OCI registry. Callers that want a real registry will slot
/// in `oci-distribution` (or the eventual workspace `--registry-url`
/// flag) here.
///
/// ```
/// use argo_marketplace::{MemoryRegistry, OciReference, OciRegistry};
/// # async fn demo() -> argo_types::ArgoResult<()> {
/// let registry = MemoryRegistry::new();
/// assert!(registry.is_empty().await);
/// let reference = OciReference::parse("ghcr.io/x/y:1")?;
/// let manifest = argo_marketplace::BundleManifest {
///     digest: "abc".into(),
///     workspace_name: "y".into(),
///     tag: "1".into(),
///     entries: vec![],
/// };
/// registry.push(&reference, b"tar bytes", &manifest).await?;
/// assert_eq!(registry.len().await, 1);
/// let (bytes, _) = registry.pull(&reference).await?;
/// assert_eq!(bytes, b"tar bytes");
/// # Ok(()) }
/// ```
#[derive(Debug, Default)]
pub struct MemoryRegistry {
    inner: tokio::sync::Mutex<std::collections::HashMap<String, (Vec<u8>, BundleManifest)>>,
}

impl MemoryRegistry {
    /// Empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Number of refs currently stored — useful for tests asserting
    /// tear-down semantics.
    pub async fn len(&self) -> usize {
        self.inner.lock().await.len()
    }

    /// Whether the registry holds zero refs.
    pub async fn is_empty(&self) -> bool {
        self.len().await == 0
    }
}

#[async_trait]
impl OciRegistry for MemoryRegistry {
    async fn push(
        &self,
        reference: &OciReference,
        tar_bytes: &[u8],
        manifest: &BundleManifest,
    ) -> ArgoResult<()> {
        self.inner
            .lock()
            .await
            .insert(reference.to_canonical(), (tar_bytes.to_vec(), manifest.clone()));
        Ok(())
    }

    async fn pull(&self, reference: &OciReference) -> ArgoResult<(Vec<u8>, BundleManifest)> {
        let key = reference.to_canonical();
        let map = self.inner.lock().await;
        map.get(&key)
            .cloned()
            .ok_or_else(|| ArgoError::Provider(format!("no such reference: {key}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_canonical_reference() {
        let r = OciReference::parse("ghcr.io/argo-agents/test:0.3.0").unwrap();
        assert_eq!(r.host, "ghcr.io");
        assert_eq!(r.repo, "argo-agents/test");
        assert_eq!(r.tag, "0.3.0");
    }

    #[test]
    fn defaults_tag_to_latest_when_absent() {
        let r = OciReference::parse("ghcr.io/argo-agents/test").unwrap();
        assert_eq!(r.tag, "latest");
    }

    #[test]
    fn rejects_invalid_reference() {
        assert!(OciReference::parse("no-slash").is_err());
        assert!(OciReference::parse("/empty-host").is_err());
    }

    #[test]
    fn host_with_port_is_handled() {
        let r = OciReference::parse("registry.local:5000/org/x:v1").unwrap();
        assert_eq!(r.host, "registry.local:5000");
        assert_eq!(r.repo, "org/x");
        assert_eq!(r.tag, "v1");
    }

    #[tokio::test]
    async fn stub_returns_clear_provider_error_on_push() {
        let reg = StubRegistry;
        let reference = OciReference::parse("ghcr.io/x/y:1").unwrap();
        let manifest = BundleManifest {
            digest: "x".into(),
            workspace_name: "y".into(),
            tag: "1".into(),
            entries: vec![],
        };
        match reg.push(&reference, b"", &manifest).await {
            Err(ArgoError::Provider(msg)) => assert!(msg.contains("oci-distribution")),
            other => panic!("expected Provider error, got {other:?}"),
        }
    }
}
