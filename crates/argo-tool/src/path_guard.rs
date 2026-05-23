//! Runtime path-confinement for tool args (#677).
//!
//! Enforces that any path argument the agent / workflow runner
//! passes to an in-core file tool resolves inside the
//! workspace root. Without this layer, an agent could ask
//! `file_read` for `args = { path = "../../etc/passwd" }` and
//! the tool would happily try to open it — the workspace
//! boundary would only exist by convention.
//!
//! This is the runtime companion to the manifest-level
//! isolation enforced in #676. Together they ensure:
//!
//! - Static (manifest): a vertical's manifest cannot declare
//!   path references that escape the vertical's own root
//!   directory.
//! - Runtime (this PR): tool dispatches cannot escape the
//!   workspace root, regardless of whether the call came
//!   from the agent loop, a workflow step, or a direct CLI
//!   invocation.
//!
//! Enforcement (canonicalize-then-prefix-check, supersedes the
//! original lexical-only check from #677; see
//! `mission-path-guard-canonicalize-symlinks` and PR #2117 F3):
//!
//! 1. The `workspace_root` must exist and is canonicalized
//!    (resolving any symlinks in the root itself). A non-existent
//!    workspace is refused — production callers always pass
//!    `ctx.workspace_root`, which is set from the agent's actual
//!    workspace.
//! 2. The requested path is joined against the canonical
//!    workspace. Any `..` component in the joined path is refused
//!    early as defense-in-depth (catches the obvious / accidental
//!    traversal even before we touch the filesystem).
//! 3. The deepest existing ancestor of the joined path is
//!    canonicalized (so any symlink in the chain is resolved),
//!    and any non-existent leaf components are re-attached
//!    lexically. This lets `file_write` create a new file at
//!    `dir/sub/new.txt` without erroring on the not-yet-present
//!    tail.
//! 4. Dangling symlinks (symlink whose target does not exist)
//!    are refused: the in-workspace symlink-of-symlink chain
//!    must resolve to something real, otherwise we have no way
//!    to verify the final destination is inside the workspace.
//! 5. The canonicalized result must `starts_with` the canonical
//!    workspace root. An in-workspace symlink pointing outside
//!    the workspace fails this check.

use std::ffi::OsString;
use std::path::{Component, Path, PathBuf};

/// Confine `requested` (relative or absolute) to `workspace_root`.
/// Returns the canonicalized absolute path on success, or an
/// error string describing the violation.
///
/// See the module-level docs for the full enforcement policy.
pub fn confine_path(workspace_root: &Path, requested: &Path) -> Result<PathBuf, String> {
    let canonical_root = workspace_root.canonicalize().map_err(|e| {
        format!(
            "workspace root '{}' cannot be canonicalized: {e}; refusing (vertical isolation, #677)",
            workspace_root.display()
        )
    })?;

    // If requested is absolute, `join` discards the workspace_root
    // and just returns the requested path — that's documented Rust
    // behaviour and exactly what we want (the absolute path must
    // stand on its own merits and still canonicalize inside the
    // workspace).
    let joined = canonical_root.join(requested);

    for component in joined.components() {
        if matches!(component, Component::ParentDir) {
            return Err(format!(
                "path '{}' contains `..` after resolving against workspace; refusing (vertical isolation, #677)",
                requested.display()
            ));
        }
    }

    let canonical = canonicalize_deepest_existing(&joined)?;

    if !canonical.starts_with(&canonical_root) {
        return Err(format!(
            "path '{}' resolves to '{}' which is outside the workspace root '{}'; refusing (vertical isolation, #677)",
            requested.display(),
            canonical.display(),
            canonical_root.display(),
        ));
    }

    Ok(canonical)
}

/// Walk `joined` from leaf to root looking for the deepest
/// ancestor that canonicalizes successfully, then re-attach the
/// non-existent tail components in their original order.
///
/// A `NotFound` at any ancestor that exists as a symlink (per
/// `symlink_metadata`) is a dangling-symlink — refused. A
/// `NotFound` at a path that does not exist at all is treated as
/// "writing a new file" and the walk continues up.
fn canonicalize_deepest_existing(joined: &Path) -> Result<PathBuf, String> {
    let mut suffix: Vec<OsString> = Vec::new();
    let mut current = joined.to_path_buf();

    loop {
        match current.canonicalize() {
            Ok(canonical) => {
                let mut result = canonical;
                for part in suffix.iter().rev() {
                    result.push(part);
                }
                return Ok(result);
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                if current.symlink_metadata().is_ok() {
                    return Err(format!(
                        "path '{}' includes dangling symlink at '{}'; refusing (vertical isolation, #677)",
                        joined.display(),
                        current.display()
                    ));
                }
                let Some(leaf) = current.file_name().map(|s| s.to_owned()) else {
                    return Err(format!(
                        "no canonicalizable ancestor for '{}'; refusing (vertical isolation, #677)",
                        joined.display()
                    ));
                };
                suffix.push(leaf);
                let Some(parent) = current.parent().map(|p| p.to_path_buf()) else {
                    return Err(format!(
                        "walked above filesystem root for '{}'; refusing (vertical isolation, #677)",
                        joined.display()
                    ));
                };
                current = parent;
            }
            Err(e) => {
                return Err(format!(
                    "canonicalize failed for ancestor of '{}': {e}; refusing (vertical isolation, #677)",
                    joined.display()
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn ws() -> TempDir {
        tempfile::tempdir().unwrap()
    }

    /// Tempdirs on macOS live under `/tmp` which is a symlink to
    /// `/private/tmp`. `confine_path` returns canonicalized paths,
    /// so the test must compare against the canonical form.
    fn canonical(p: &Path) -> PathBuf {
        p.canonicalize().unwrap()
    }

    #[test]
    fn relative_path_inside_workspace_passes() {
        let ws = ws();
        let resolved = confine_path(ws.path(), Path::new("hello.txt")).unwrap();
        assert_eq!(resolved, canonical(ws.path()).join("hello.txt"));
    }

    #[test]
    fn relative_path_in_subdir_passes() {
        let ws = ws();
        let resolved = confine_path(ws.path(), Path::new("a/b/c.txt")).unwrap();
        assert_eq!(resolved, canonical(ws.path()).join("a/b/c.txt"));
    }

    #[test]
    fn relative_parent_traversal_rejected() {
        let ws = ws();
        let err = confine_path(ws.path(), Path::new("../escape.txt")).unwrap_err();
        assert!(err.contains("`..`"), "got: {err}");
    }

    #[test]
    fn nested_parent_traversal_rejected() {
        let ws = ws();
        let err = confine_path(ws.path(), Path::new("a/../../escape.txt")).unwrap_err();
        assert!(err.contains("`..`"), "got: {err}");
    }

    #[test]
    fn absolute_path_inside_workspace_passes() {
        let ws = ws();
        let target = ws.path().join("foo.txt");
        let resolved = confine_path(ws.path(), &target).unwrap();
        assert_eq!(resolved, canonical(ws.path()).join("foo.txt"));
    }

    #[test]
    fn absolute_path_outside_workspace_rejected() {
        let ws = ws();
        let err = confine_path(ws.path(), Path::new("/etc/passwd")).unwrap_err();
        assert!(err.contains("outside the workspace root"), "got: {err}");
    }

    #[test]
    fn absolute_root_path_rejected() {
        let ws = ws();
        let err = confine_path(ws.path(), Path::new("/")).unwrap_err();
        assert!(err.contains("outside the workspace root"), "got: {err}");
    }

    #[test]
    fn empty_path_resolves_to_workspace_root() {
        let ws = ws();
        let resolved = confine_path(ws.path(), Path::new("")).unwrap();
        assert_eq!(resolved, canonical(ws.path()));
    }

    #[test]
    fn nonexistent_workspace_root_rejected() {
        let err = confine_path(
            Path::new("/tmp/argo-does-not-exist-9b1c"),
            Path::new("foo.txt"),
        )
        .unwrap_err();
        assert!(err.contains("cannot be canonicalized"), "got: {err}");
    }

    // ---- Symlink-aware coverage (the canonicalize-then-check upgrade) ----

    #[cfg(unix)]
    #[test]
    fn in_workspace_symlink_to_outside_workspace_rejected() {
        let ws = ws();
        let outside = tempfile::tempdir().unwrap();
        let outside_file = outside.path().join("secret.txt");
        std::fs::write(&outside_file, "secret").unwrap();
        let link = ws.path().join("escape_link");
        std::os::unix::fs::symlink(&outside_file, &link).unwrap();

        let err = confine_path(ws.path(), Path::new("escape_link")).unwrap_err();
        assert!(
            err.contains("outside the workspace root"),
            "expected outside-workspace rejection, got: {err}"
        );
    }

    #[cfg(unix)]
    #[test]
    fn in_workspace_symlink_to_in_workspace_target_allowed() {
        let ws = ws();
        let target = ws.path().join("real.txt");
        std::fs::write(&target, "ok").unwrap();
        let link = ws.path().join("link_to_real");
        std::os::unix::fs::symlink(&target, &link).unwrap();

        let resolved = confine_path(ws.path(), Path::new("link_to_real")).unwrap();
        assert_eq!(resolved, canonical(ws.path()).join("real.txt"));
    }

    #[cfg(unix)]
    #[test]
    fn symlink_chain_inside_workspace_followed_and_allowed() {
        let ws = ws();
        let real = ws.path().join("real.txt");
        std::fs::write(&real, "ok").unwrap();
        let mid = ws.path().join("mid");
        std::os::unix::fs::symlink(&real, &mid).unwrap();
        let outer = ws.path().join("outer");
        std::os::unix::fs::symlink(&mid, &outer).unwrap();

        let resolved = confine_path(ws.path(), Path::new("outer")).unwrap();
        assert_eq!(resolved, canonical(ws.path()).join("real.txt"));
    }

    #[cfg(unix)]
    #[test]
    fn symlink_chain_escapes_workspace_rejected() {
        let ws = ws();
        let outside = tempfile::tempdir().unwrap();
        let outside_file = outside.path().join("secret.txt");
        std::fs::write(&outside_file, "secret").unwrap();
        let mid = ws.path().join("mid");
        std::os::unix::fs::symlink(&outside_file, &mid).unwrap();
        let outer = ws.path().join("outer");
        std::os::unix::fs::symlink(&mid, &outer).unwrap();

        let err = confine_path(ws.path(), Path::new("outer")).unwrap_err();
        assert!(
            err.contains("outside the workspace root"),
            "expected outside-workspace rejection through symlink chain, got: {err}"
        );
    }

    /// Writing a new file at `subdir/new.txt` where `subdir` exists
    /// but `new.txt` does not — `file_write` depends on this case.
    #[test]
    fn nonexistent_leaf_under_existing_dir_allowed() {
        let ws = ws();
        let sub = ws.path().join("subdir");
        std::fs::create_dir(&sub).unwrap();

        let resolved = confine_path(ws.path(), Path::new("subdir/new.txt")).unwrap();
        assert_eq!(resolved, canonical(ws.path()).join("subdir/new.txt"));
    }

    /// Writing a new file deep under multiple non-existent directories
    /// — `file_write { create_dirs: true }` is the typical caller.
    #[test]
    fn nonexistent_deep_path_under_workspace_allowed() {
        let ws = ws();
        let resolved =
            confine_path(ws.path(), Path::new("a/b/c/new.txt")).unwrap();
        assert_eq!(resolved, canonical(ws.path()).join("a/b/c/new.txt"));
    }

    /// Dangling symlink — the link itself exists (via
    /// `symlink_metadata`) but its target does not. Reject, because
    /// the eventual `open()` will follow the link and we cannot
    /// verify the final destination is inside the workspace.
    #[cfg(unix)]
    #[test]
    fn dangling_symlink_rejected() {
        let ws = ws();
        let link = ws.path().join("dangling");
        std::os::unix::fs::symlink("/tmp/this-does-not-exist-cf91", &link).unwrap();

        let err = confine_path(ws.path(), Path::new("dangling")).unwrap_err();
        assert!(
            err.contains("dangling symlink"),
            "expected dangling-symlink rejection, got: {err}"
        );
    }
}
