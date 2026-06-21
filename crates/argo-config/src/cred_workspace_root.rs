//! Canonical credential-vault workspace-root resolver.
//!
//! Split out of [`crate::cred`] to keep that module under the repo's
//! 500-line ceiling. Both the credential READ path (MCP connector spawn
//! → `load_tenant`) and the WRITE path (gateway / CLI `set_tenant`)
//! resolve their workspace through [`workspace_root`] so a credential
//! written in production lands at the SAME durable path it is read from.

use std::env;
use std::path::PathBuf;

/// Resolve the workspace root that hosts the credential vault
/// (`<root>/.argo/credentials.toml`).
///
/// Precedence (first non-empty wins):
/// 1. `$ARGO_WORKSPACE` — the durable-volume mount in production. In the
///    AWS deploy this is `/data`, the only path snapshotted by the
///    EFS/S3 backup, so credentials written under it survive a task
///    restart or redeploy.
/// 2. `$ARGO_HOME` — the same env knob the config loader honours
///    ([`crate::loader`] reads it via `ARGO_HOME`), so config and
///    credentials resolve to the same base when only `ARGO_HOME` is set.
/// 3. [`std::env::current_dir`] — local-dev default (no env set), with a
///    final fallback to `.` if `current_dir` errors (matching the
///    loader's `current_dir().unwrap_or_else(|_| ".")` pattern).
///
/// # Examples
/// ```
/// // With no env set, the vault base is the current working directory.
/// std::env::remove_var("ARGO_WORKSPACE");
/// std::env::remove_var("ARGO_HOME");
/// let root = argo_config::cred::workspace_root();
/// assert_eq!(root, std::env::current_dir().unwrap());
/// ```
#[must_use]
pub fn workspace_root() -> PathBuf {
    if let Some(ws) = env_non_empty("ARGO_WORKSPACE") {
        return PathBuf::from(ws);
    }
    if let Some(home) = env_non_empty("ARGO_HOME") {
        return PathBuf::from(home);
    }
    env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

/// Read an env var, returning `Some` only when it is set AND non-empty.
fn env_non_empty(key: &str) -> Option<String> {
    env::var(key).ok().filter(|s| !s.is_empty())
}

#[cfg(test)]
#[path = "cred_workspace_root_tests.rs"]
mod cred_workspace_root_tests;
