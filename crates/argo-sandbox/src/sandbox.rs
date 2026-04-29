//! `Sandbox` trait — single seam for command execution. Phase 5
//! ships:
//!
//! - [`LocalSandbox`] — runs the command in the current OS namespace
//!   under `bash -c`. Suitable for fully-trusted workspaces (the
//!   Phase-1 walking-skeleton case).
//! - [`DockerSandbox`] — Phase-1 stub returning
//!   `ArgoError::Provider("not yet wired")`. The real Docker backend
//!   lands with the §15 conformance test follow-up.
//!
//! The trait is intentionally narrow — `run` returns stdout / stderr /
//! exit code with output capped per stream so a runaway command can't
//! OOM the loop.
use argo_types::{ArgoError, ArgoResult};
use async_trait::async_trait;
use std::path::PathBuf;
use std::time::Duration;

/// Generous default per-command wall-clock timeout.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(60);

/// Cap per stream — generous enough for normal output, hard enough to
/// stop a runaway command from filling memory.
pub const MAX_OUTPUT_BYTES: usize = 64 * 1024;

/// One run request.
#[derive(Debug, Clone)]
pub struct RunRequest {
    /// Shell command line (executed via `bash -c`).
    pub command: String,
    /// Working directory for the command.
    pub working_dir: PathBuf,
    /// Environment variables to inject (in addition to the parent
    /// process's, unless `inherit_env = false`).
    pub env: Vec<(String, String)>,
    /// If `false`, the child sees only the explicitly-passed `env`.
    pub inherit_env: bool,
    /// Wall-clock timeout. `None` → use [`DEFAULT_TIMEOUT`].
    pub timeout: Option<Duration>,
}

impl RunRequest {
    /// Convenience constructor for `bash -c` in `cwd` with default
    /// timeout + inherited env.
    #[must_use]
    pub fn new(command: impl Into<String>, working_dir: impl Into<PathBuf>) -> Self {
        Self {
            command: command.into(),
            working_dir: working_dir.into(),
            env: Vec::new(),
            inherit_env: true,
            timeout: None,
        }
    }
}

/// One run result.
#[derive(Debug, Clone)]
pub struct RunResult {
    /// Stdout (truncated to [`MAX_OUTPUT_BYTES`]).
    pub stdout: String,
    /// Stderr (truncated to [`MAX_OUTPUT_BYTES`]).
    pub stderr: String,
    /// Exit code (or `-1` if the process was killed by signal).
    pub exit_code: i32,
    /// Whether the timeout fired (process was killed).
    pub timed_out: bool,
    /// Wall-clock duration of the run.
    pub duration: Duration,
}

/// Sandbox abstraction.
///
/// ```
/// # use argo_sandbox::Sandbox;
/// fn assert_object_safe(_s: &dyn Sandbox) {}
/// ```
#[async_trait]
pub trait Sandbox: Send + Sync + std::fmt::Debug {
    /// Stable id (e.g. `"local"`, `"docker"`).
    fn id(&self) -> &str;
    /// Execute `req` and return `RunResult`. Errors are reserved for
    /// transport-level failures (couldn't spawn, couldn't read output);
    /// non-zero exit codes are normal results.
    async fn run(&self, req: &RunRequest) -> ArgoResult<RunResult>;
}

/// Truncate raw bytes to `cap` UTF-8-safely with a `[truncated …]`
/// trailer.
#[must_use]
pub fn truncate_stream(bytes: &[u8], cap: usize) -> String {
    if bytes.len() <= cap {
        return String::from_utf8_lossy(bytes).into_owned();
    }
    let head = String::from_utf8_lossy(&bytes[..cap]).into_owned();
    format!(
        "{head}\n... [truncated {} bytes from this stream]",
        bytes.len() - cap
    )
}

/// Map an `i/o` failure into [`ArgoError::Provider`] with a stable prefix.
pub(crate) fn io_err(label: &str, e: std::io::Error) -> ArgoError {
    ArgoError::Provider(format!("{label}: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_stream_keeps_short_buffers_intact() {
        assert_eq!(truncate_stream(b"hi", MAX_OUTPUT_BYTES), "hi");
    }

    #[test]
    fn truncate_stream_caps_long_buffers() {
        let big = vec![b'x'; MAX_OUTPUT_BYTES + 17];
        let s = truncate_stream(&big, MAX_OUTPUT_BYTES);
        assert!(s.contains("[truncated 17"));
    }

    #[test]
    fn run_request_new_uses_defaults() {
        let r = RunRequest::new("echo hi", "/tmp");
        assert_eq!(r.command, "echo hi");
        assert_eq!(r.working_dir, PathBuf::from("/tmp"));
        assert!(r.inherit_env);
        assert!(r.timeout.is_none());
    }
}
