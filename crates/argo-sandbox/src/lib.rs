//! `argo-sandbox` — sandboxed command execution (§15).
//!
//! [`Sandbox`] is the trait every backend implements. Phase 5 ships:
//!
//! - [`LocalSandbox`] — `bash -c` in the current OS namespace. No
//!   isolation; suitable for trusted workspaces (the Phase-1
//!   walking-skeleton case).
//! - [`DockerSandbox`] — Phase-5 stub returning a clear
//!   `ArgoError::Provider` so the agent loop can route around it via
//!   fallback. Real Docker backend lands with the §15 conformance
//!   test suite.
//!
//! ```
//! # async fn doc() -> argo_types::ArgoResult<()> {
//! # let dir = tempfile::tempdir().unwrap();
//! use argo_sandbox::{LocalSandbox, RunRequest, Sandbox};
//! let sandbox = LocalSandbox::new();
//! let r = sandbox.run(&RunRequest::new("echo ok", dir.path())).await?;
//! assert_eq!(r.exit_code, 0);
//! # Ok(()) }
//! ```
#![warn(missing_docs)]
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

mod docker;
mod local;
mod sandbox;

pub use docker::DockerSandbox;
pub use local::LocalSandbox;
pub use sandbox::{
    truncate_stream, RunRequest, RunResult, Sandbox, DEFAULT_TIMEOUT, MAX_OUTPUT_BYTES,
};
