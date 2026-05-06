//! Errors raised by the sandboxed filesystem executor. Each variant
//! is a distinct refusal — callers (and the audit ledger) can tell
//! a traversal-rejection apart from a quota-rejection without
//! sniffing strings.
use argo_types::ArgoError;
use std::path::PathBuf;
use thiserror::Error;

/// Why the executor refused (or aborted) a filesystem op.
#[derive(Debug, Error)]
pub enum FsExecutorError {
    /// The supplied path tries to escape the sandbox root via `..`,
    /// an absolute path, or a Windows-style drive prefix. The path
    /// is rejected before any I/O happens.
    #[error("path escapes sandbox root: {0}")]
    PathEscape(String),

    /// The path resolves through a symlink whose target sits outside
    /// the sandbox root. Symlinks are not followed past the root.
    #[error("symlink target outside sandbox: {0}")]
    SymlinkEscape(String),

    /// The op would exceed the per-op size cap (default 16 MiB).
    /// Carries the cap and the would-be size for the audit log.
    #[error("op size {size} bytes exceeds per-op cap {cap} bytes")]
    OpSizeCap { size: u64, cap: u64 },

    /// The op would exceed the per-worker total-size cap (default
    /// 1 GiB). Computed from the real on-disk usage walk before the
    /// op, so concurrent writers may still race past — caller
    /// serialises if strict accounting is required.
    #[error("worker total {used} bytes + op {delta} bytes exceeds cap {cap} bytes")]
    TotalSizeCap { used: u64, delta: u64, cap: u64 },

    /// Underlying I/O failure (open, read, write, mkdir, …) carried
    /// through verbatim so operators see the OS message.
    #[error("io {op} {path:?}: {source}")]
    Io {
        op: &'static str,
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// `remove` only handles a single non-recursive entry. Recursive
    /// deletion is high-risk and will be a separate approval-gated
    /// op in a follow-up brief.
    #[error("remove of non-empty directory refused: {0}")]
    NonEmptyDir(PathBuf),
}

impl FsExecutorError {
    /// Wrap a `std::io::Error` with the failing op name + path so
    /// the audit ledger can group failures by op.
    pub fn io(op: &'static str, path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Self::Io {
            op,
            path: path.into(),
            source,
        }
    }
}

/// Handy result alias used throughout this crate.
pub type FsResult<T> = Result<T, FsExecutorError>;

impl From<FsExecutorError> for ArgoError {
    fn from(err: FsExecutorError) -> Self {
        ArgoError::Provider(err.to_string())
    }
}
