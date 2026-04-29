//! `DockerSandbox` — Phase-5 stub.
//!
//! The real Docker backend lands with the §15 conformance suite that
//! the Phase-1 walking skeleton doesn't need. Stub returns
//! `ArgoError::Provider("docker sandbox not yet wired …")` so the
//! agent loop can route around it cleanly via fallback.
use crate::sandbox::{RunRequest, RunResult, Sandbox};
use argo_types::{ArgoError, ArgoResult};
use async_trait::async_trait;

/// Docker-backed sandbox — stubbed for Phase 5.
#[derive(Debug, Clone)]
pub struct DockerSandbox {
    /// Image the sandbox would run under (e.g. `"python:3.12"`).
    pub image: String,
}

impl DockerSandbox {
    /// New stub bound to `image`.
    #[must_use]
    pub fn new(image: impl Into<String>) -> Self {
        Self {
            image: image.into(),
        }
    }
}

#[async_trait]
impl Sandbox for DockerSandbox {
    fn id(&self) -> &str {
        "docker"
    }
    async fn run(&self, _req: &RunRequest) -> ArgoResult<RunResult> {
        Err(ArgoError::Provider(format!(
            "docker sandbox not yet wired (image={}); §15 follow-up",
            self.image
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn stub_returns_clear_provider_error() {
        let s = DockerSandbox::new("alpine:3");
        let dir = tempfile::tempdir().unwrap();
        let r = s.run(&RunRequest::new("echo hi", dir.path())).await;
        match r {
            Err(ArgoError::Provider(msg)) => assert!(msg.contains("alpine:3")),
            other => panic!("expected Provider error, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn id_is_docker() {
        assert_eq!(DockerSandbox::new("x").id(), "docker");
    }
}
