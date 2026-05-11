//! Sandbox enforcement for CSV output paths.
//!
//! Resolves a caller-supplied `path` beneath the operator-configured
//! `output_root` (default `~/.argo/exports/csv/`). Path-traversal
//! attempts (`..`, absolute paths pointing outside the root,
//! symlinks targeting outside the root) raise
//! [`CsvSandboxError::SandboxViolation`].
//!
//! Implementation note: this delegates to
//! `argo_executor_filesystem::Sandbox` for the heavy lifting (root
//! canonicalisation + symlink-escape detection) and adds two
//! conveniences on top:
//!   * tilde expansion on `output_root` (so the default
//!     `~/.argo/exports/csv/` actually works without the caller
//!     pre-expanding).
//!   * acceptance of an *absolute* path whose canonical prefix is the
//!     output root (the underlying Sandbox primitive only accepts
//!     relative paths; this layer strips the root prefix first).

use argo_executor_filesystem::{FsExecutorError, Sandbox};
use argo_types::ArgoError;
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Default operator-side output root. Mirrors the brief and the
/// `[document_csv].output_root` default in argo-config.
pub const DEFAULT_OUTPUT_ROOT: &str = "~/.argo/exports/csv/";

/// Sandbox-violation surfaced by [`resolve_sandboxed_path`]. A typed
/// enum so the audit ledger can tell traversal apart from a generic
/// IO refusal without sniffing strings.
#[derive(Debug, Error)]
pub enum CsvSandboxError {
    /// The supplied `path` escapes `output_root` via `..`, an
    /// absolute path outside the root, or a symlink whose target
    /// sits outside the root.
    #[error("sandbox violation: path {0} escapes output_root")]
    SandboxViolation(String),
    /// Underlying filesystem error while resolving the root or
    /// canonicalising the candidate path.
    #[error("sandbox io: {0}")]
    Io(String),
}

impl From<CsvSandboxError> for ArgoError {
    fn from(err: CsvSandboxError) -> Self {
        match err {
            // Sandbox violations are caller-input failures (the
            // manifest declared a bad path) — Schema is the closest
            // canonical variant.
            CsvSandboxError::SandboxViolation(_) => ArgoError::Schema(err.to_string()),
            CsvSandboxError::Io(_) => ArgoError::Io(err.to_string()),
        }
    }
}

/// Resolve `path` against `output_root`. Returns the absolute
/// resolved path (suitable for `std::fs::write`). The output root is
/// created if missing.
///
/// # Errors
/// Returns [`CsvSandboxError::SandboxViolation`] when `path` escapes
/// the root via `..`, an absolute path whose canonical form is
/// outside the root, or an existing symlink whose target points
/// outside.
pub fn resolve_sandboxed_path(
    output_root: &Path,
    path: &Path,
) -> Result<PathBuf, CsvSandboxError> {
    let root = expand_tilde(output_root);
    let sb = Sandbox::open(&root).map_err(fs_err_to_sandbox_io)?;
    let relative = strip_root_prefix(sb.root(), path)?;
    let resolved = sb.resolve(&relative).map_err(fs_err_to_sandbox)?;
    if let Some(parent) = resolved.parent() {
        sb.assert_no_symlink_escape(parent).map_err(fs_err_to_sandbox)?;
    }
    sb.assert_no_symlink_escape(&resolved).map_err(fs_err_to_sandbox)?;
    Ok(resolved)
}

/// Expand a leading `~/` against `$HOME` (or accept the value
/// verbatim if no tilde / no $HOME). Mirrors the behaviour of the
/// `argo-config` tilde helper so operators see consistent paths.
fn expand_tilde(p: &Path) -> PathBuf {
    let s = p.to_string_lossy();
    if let Some(rest) = s.strip_prefix("~/") {
        if let Some(home) = std::env::var_os("HOME") {
            return PathBuf::from(home).join(rest);
        }
    } else if s == "~" {
        if let Some(home) = std::env::var_os("HOME") {
            return PathBuf::from(home);
        }
    }
    p.to_path_buf()
}

/// If `path` is absolute, strip a leading match of `root` and return
/// the relative tail; otherwise pass `path` through verbatim. An
/// absolute `path` outside `root` is a sandbox violation here — the
/// underlying `Sandbox::resolve` would reject all absolute paths
/// indiscriminately, so we pre-classify.
fn strip_root_prefix(root: &Path, path: &Path) -> Result<PathBuf, CsvSandboxError> {
    if path.is_absolute() {
        // Canonical-prefix check (root is already canonical via
        // `Sandbox::open`).
        let candidate_canon = if path.exists() {
            path.canonicalize()
                .map_err(|e| CsvSandboxError::Io(format!("canonicalize {}: {e}", path.display())))?
        } else {
            // Walk up to the deepest existing parent + canonicalise
            // it; rebuild the tail.
            canonicalise_lossy(path)?
        };
        if let Ok(rel) = candidate_canon.strip_prefix(root) {
            return Ok(rel.to_path_buf());
        }
        return Err(CsvSandboxError::SandboxViolation(path.display().to_string()));
    }
    Ok(path.to_path_buf())
}

/// Canonicalise the deepest existing prefix of `path`, then append
/// the not-yet-existent tail. Lets us check sandbox membership for
/// write-targets that don't exist yet.
fn canonicalise_lossy(path: &Path) -> Result<PathBuf, CsvSandboxError> {
    let mut deepest = path;
    while !deepest.exists() {
        match deepest.parent() {
            Some(p) if !p.as_os_str().is_empty() => deepest = p,
            _ => return Ok(path.to_path_buf()),
        }
    }
    let canon = deepest
        .canonicalize()
        .map_err(|e| CsvSandboxError::Io(format!("canonicalize {}: {e}", deepest.display())))?;
    if deepest == path {
        return Ok(canon);
    }
    let tail = path
        .strip_prefix(deepest)
        .map_err(|_| CsvSandboxError::Io(format!("strip prefix {}", path.display())))?;
    Ok(canon.join(tail))
}

fn fs_err_to_sandbox(err: FsExecutorError) -> CsvSandboxError {
    match err {
        FsExecutorError::PathEscape(p) | FsExecutorError::SymlinkEscape(p) => {
            CsvSandboxError::SandboxViolation(p)
        }
        other => CsvSandboxError::Io(other.to_string()),
    }
}

fn fs_err_to_sandbox_io(err: FsExecutorError) -> CsvSandboxError {
    CsvSandboxError::Io(err.to_string())
}
