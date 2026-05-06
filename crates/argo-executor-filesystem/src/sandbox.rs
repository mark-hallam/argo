//! Per-worker sandbox guard. Resolves a caller-supplied relative
//! path against the worker's `fs_root` and rejects:
//!   * absolute paths
//!   * paths that climb above the root via `..`
//!   * paths whose final component (or any intermediate symlink)
//!     points outside the root
//!
//! The guard ALSO carries the per-op + per-worker size caps. The
//! ops layer (`crate::ops`) calls `sandbox.preflight_op_size(n)`
//! before any write/mkdir so quota refusals come back as typed
//! errors rather than half-finished writes.

use crate::error::{FsExecutorError, FsResult};
use std::path::{Component, Path, PathBuf};

/// Default per-op size cap (16 MiB). Override via env or with
/// `Sandbox::with_caps`. The cap covers the *single* op (a
/// `write_file` payload, or a directory walk total for `list_dir`).
pub const DEFAULT_OP_SIZE_CAP: u64 = 16 * 1024 * 1024;

/// Default per-worker total-size cap (1 GiB). Approximation: walked
/// from the real on-disk tree at op time, so concurrent writers may
/// still race past. Conservative for v1.
pub const DEFAULT_TOTAL_SIZE_CAP: u64 = 1024 * 1024 * 1024;

/// Env var consulted at sandbox-construction time to bump the
/// per-op size cap above the default. Anything unparseable falls
/// back to the default with no warning (the caller controls the
/// env, not random input).
pub const ENV_OP_SIZE_CAP: &str = "ARGO_FS_OP_SIZE_CAP";

/// Env var for the per-worker total cap.
pub const ENV_TOTAL_SIZE_CAP: &str = "ARGO_FS_TOTAL_SIZE_CAP";

/// Sandbox guard. Construct one per worker (and re-use for every
/// op) — it caches a canonicalised `root` so each `resolve()` is a
/// cheap string check.
#[derive(Debug, Clone)]
pub struct Sandbox {
    root: PathBuf,
    op_size_cap: u64,
    total_size_cap: u64,
}

impl Sandbox {
    /// Build a sandbox rooted at `fs_root`. The directory is created
    /// if missing (workers don't have a setup step today). Caps
    /// pick up env overrides if set, otherwise the module defaults.
    pub fn open(fs_root: impl AsRef<Path>) -> FsResult<Self> {
        let root = fs_root.as_ref();
        if !root.exists() {
            std::fs::create_dir_all(root)
                .map_err(|e| FsExecutorError::io("mkdir_root", root, e))?;
        }
        let canonical = root
            .canonicalize()
            .map_err(|e| FsExecutorError::io("canonicalize_root", root, e))?;
        let op_size_cap = parse_env_size(ENV_OP_SIZE_CAP).unwrap_or(DEFAULT_OP_SIZE_CAP);
        let total_size_cap =
            parse_env_size(ENV_TOTAL_SIZE_CAP).unwrap_or(DEFAULT_TOTAL_SIZE_CAP);
        Ok(Self {
            root: canonical,
            op_size_cap,
            total_size_cap,
        })
    }

    /// Build a sandbox with explicit caps — used by tests + by
    /// callers who want strict, env-independent quotas.
    #[must_use]
    pub fn with_caps(mut self, op_size_cap: u64, total_size_cap: u64) -> Self {
        self.op_size_cap = op_size_cap;
        self.total_size_cap = total_size_cap;
        self
    }

    /// Borrow the canonicalised root.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Per-op size cap (bytes).
    #[must_use]
    pub fn op_size_cap(&self) -> u64 {
        self.op_size_cap
    }

    /// Per-worker total-size cap (bytes).
    #[must_use]
    pub fn total_size_cap(&self) -> u64 {
        self.total_size_cap
    }

    /// Resolve `rel` against the sandbox root, rejecting any path
    /// that escapes. The returned `PathBuf` is the absolute,
    /// root-rebased path callers should use for their I/O.
    ///
    /// Semantics:
    ///   * `Component::Prefix` (Windows drive) → rejected
    ///   * `Component::RootDir` (absolute) → rejected
    ///   * `Component::ParentDir` (`..`) → rejected
    ///   * `Component::CurDir` (`.`) + `Component::Normal(...)` → accepted
    ///
    /// We do NOT canonicalise the result — that would resolve
    /// symlinks ahead of the I/O attempt. Symlink-traversal rejection
    /// happens at op time via `assert_no_symlink_escape`.
    pub fn resolve(&self, rel: impl AsRef<Path>) -> FsResult<PathBuf> {
        let rel = rel.as_ref();
        let mut buf = self.root.clone();
        for comp in rel.components() {
            match comp {
                Component::Prefix(_) | Component::RootDir => {
                    return Err(FsExecutorError::PathEscape(rel.display().to_string()));
                }
                Component::ParentDir => {
                    return Err(FsExecutorError::PathEscape(rel.display().to_string()));
                }
                Component::CurDir => {}
                Component::Normal(seg) => buf.push(seg),
            }
        }
        Ok(buf)
    }

    /// After a successful `resolve`, check that the on-disk path
    /// (if it exists) doesn't symlink to somewhere outside the
    /// sandbox. Called from each op so callers don't have to remember.
    /// Non-existent paths pass through (they're write-targets).
    pub fn assert_no_symlink_escape(&self, abs: &Path) -> FsResult<()> {
        if !abs.exists() {
            return Ok(());
        }
        let real = abs
            .canonicalize()
            .map_err(|e| FsExecutorError::io("canonicalize", abs, e))?;
        if real.starts_with(&self.root) {
            Ok(())
        } else {
            Err(FsExecutorError::SymlinkEscape(real.display().to_string()))
        }
    }

    /// Refuse the op if its byte count would breach the per-op cap.
    pub fn preflight_op_size(&self, size: u64) -> FsResult<()> {
        if size > self.op_size_cap {
            Err(FsExecutorError::OpSizeCap {
                size,
                cap: self.op_size_cap,
            })
        } else {
            Ok(())
        }
    }

    /// Walk the sandbox, sum up file sizes, and refuse if `delta`
    /// would push us past the per-worker cap. `delta` is the size
    /// the upcoming write would add (so callers should pass `0` for
    /// ops that don't grow disk usage).
    pub fn preflight_total_size(&self, delta: u64) -> FsResult<()> {
        let used = walk_total(&self.root)?;
        if used.saturating_add(delta) > self.total_size_cap {
            Err(FsExecutorError::TotalSizeCap {
                used,
                delta,
                cap: self.total_size_cap,
            })
        } else {
            Ok(())
        }
    }
}

fn parse_env_size(key: &str) -> Option<u64> {
    std::env::var(key).ok().and_then(|v| v.parse::<u64>().ok())
}

/// Sum the size of every regular file under `dir`. Symlinks are
/// counted by their *link* size (their target may live outside the
/// sandbox, which we explicitly don't measure).
fn walk_total(dir: &Path) -> FsResult<u64> {
    let mut total: u64 = 0;
    let mut stack = vec![dir.to_path_buf()];
    while let Some(cur) = stack.pop() {
        let read = std::fs::read_dir(&cur)
            .map_err(|e| FsExecutorError::io("read_dir", &cur, e))?;
        for entry in read {
            let entry = entry.map_err(|e| FsExecutorError::io("read_dir_entry", &cur, e))?;
            let meta = entry
                .metadata()
                .map_err(|e| FsExecutorError::io("metadata", entry.path(), e))?;
            if meta.is_dir() {
                stack.push(entry.path());
            } else {
                total = total.saturating_add(meta.len());
            }
        }
    }
    Ok(total)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn resolve_accepts_clean_relative_path() {
        let dir = tempdir().unwrap();
        let sb = Sandbox::open(dir.path()).unwrap();
        let p = sb.resolve("notes/today.md").unwrap();
        assert!(p.starts_with(sb.root()));
        assert!(p.ends_with("notes/today.md"));
    }

    #[test]
    fn resolve_rejects_parent_traversal() {
        let dir = tempdir().unwrap();
        let sb = Sandbox::open(dir.path()).unwrap();
        let err = sb.resolve("../escape.txt").unwrap_err();
        assert!(matches!(err, FsExecutorError::PathEscape(_)));
    }

    #[test]
    fn resolve_rejects_absolute_path() {
        let dir = tempdir().unwrap();
        let sb = Sandbox::open(dir.path()).unwrap();
        let err = sb.resolve("/etc/passwd").unwrap_err();
        assert!(matches!(err, FsExecutorError::PathEscape(_)));
    }

    #[test]
    fn resolve_accepts_dot_segments() {
        let dir = tempdir().unwrap();
        let sb = Sandbox::open(dir.path()).unwrap();
        let p = sb.resolve("./notes/./today.md").unwrap();
        assert!(p.ends_with("notes/today.md"));
    }

    #[cfg(unix)]
    #[test]
    fn symlink_escape_is_caught_by_assert() {
        use std::os::unix::fs::symlink;
        let outside = tempdir().unwrap();
        let inside = tempdir().unwrap();
        let sb = Sandbox::open(inside.path()).unwrap();
        let outside_target = outside.path().join("treasure.txt");
        std::fs::write(&outside_target, b"hi").unwrap();
        let link = inside.path().join("link.txt");
        symlink(&outside_target, &link).unwrap();
        let err = sb.assert_no_symlink_escape(&link).unwrap_err();
        assert!(matches!(err, FsExecutorError::SymlinkEscape(_)));
    }

    #[test]
    fn preflight_op_size_refuses_oversize() {
        let dir = tempdir().unwrap();
        let sb = Sandbox::open(dir.path()).unwrap().with_caps(8, 1024);
        assert!(sb.preflight_op_size(8).is_ok());
        let err = sb.preflight_op_size(9).unwrap_err();
        assert!(matches!(err, FsExecutorError::OpSizeCap { size: 9, cap: 8 }));
    }

    #[test]
    fn preflight_total_size_refuses_when_used_plus_delta_exceeds_cap() {
        let dir = tempdir().unwrap();
        let sb = Sandbox::open(dir.path()).unwrap().with_caps(1024, 100);
        std::fs::write(dir.path().join("a.bin"), [0u8; 80]).unwrap();
        // 80 used + 50 delta > 100 cap → reject
        let err = sb.preflight_total_size(50).unwrap_err();
        assert!(matches!(err, FsExecutorError::TotalSizeCap { .. }));
        // 80 used + 10 delta = 90 ≤ 100 → ok
        assert!(sb.preflight_total_size(10).is_ok());
    }

    #[test]
    fn open_creates_root_when_missing() {
        let parent = tempdir().unwrap();
        let root = parent.path().join("workers/123");
        assert!(!root.exists());
        let sb = Sandbox::open(&root).unwrap();
        assert!(sb.root().exists());
    }
}
