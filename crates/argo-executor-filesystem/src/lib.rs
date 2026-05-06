//! V1.2 Filesystem ExecutorKind — sandboxed file ops for Argo
//! workers.
//!
//! The mission says "any task a human can do on a PC/Mac";
//! filesystem ops are a foundational PC/Mac capability that the
//! ComputerUse executor (browser only) doesn't cover.
//!
//! Each worker gets a private root prefix
//! (`worker.fs_root = /tmp/argo-fs/<worker_id>/`) and the executor
//! refuses any op whose path escapes that root via:
//!   * absolute paths (`/etc/passwd`)
//!   * parent traversal (`../escape`)
//!   * symlinks whose target sits outside the root
//!
//! Two size caps protect runaway workers:
//!   * per-op cap (default 16 MiB) — the size of a single
//!     `write_file` payload, env override `ARGO_FS_OP_SIZE_CAP`
//!   * per-worker total cap (default 1 GiB) — measured by walking
//!     the on-disk tree at op time, env override
//!     `ARGO_FS_TOTAL_SIZE_CAP`
//!
//! Ops are split into read-only (auto-approvable) and mutating
//! (approval-gated) — see `FsOpKind::is_read_only`. The Argo
//! AutoApprovalPolicy gate consumes that classifier when deciding
//! whether to clear an action without operator sign-off.
//!
//! Quick start:
//! ```no_run
//! use argo_executor_filesystem::{Sandbox, ops};
//! let sb = Sandbox::open("/tmp/argo-fs/worker-123").unwrap();
//! ops::write_file(&sb, "notes/today.md", b"hello world").unwrap();
//! let body = ops::read_file(&sb, "notes/today.md").unwrap();
//! assert_eq!(body, b"hello world");
//! ```

pub mod error;
pub mod executor;
pub mod ops;
pub mod sandbox;

pub use error::{FsExecutorError, FsResult};
pub use executor::{FsAction, FsConfig, FsExecutor, FsOpOutcome, DEFAULT_FS_ROOT_PREFIX};
pub use ops::{DirEntry, FileMeta, FsOpKind};
pub use sandbox::{
    Sandbox, DEFAULT_OP_SIZE_CAP, DEFAULT_TOTAL_SIZE_CAP, ENV_OP_SIZE_CAP, ENV_TOTAL_SIZE_CAP,
};
