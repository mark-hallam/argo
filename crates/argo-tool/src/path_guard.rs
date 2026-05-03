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
//! Two enforcement rules:
//!
//! 1. After joining the requested path against the workspace
//!    root, no `..` component may remain in the resolved path.
//! 2. If the requested path is absolute, it must lexically
//!    start with the workspace root.
//!
//! Lexical-only — does not follow symlinks. A symlink inside
//! the workspace pointing outside still escapes. Follow-up
//! PR (#678+) can add canonicalization for the symlink case;
//! the lexical check catches the obvious / accidental
//! traversal attempts that are the most common mistake.
use std::path::{Component, Path, PathBuf};

/// Confine `requested` (relative or absolute) to
/// `workspace_root`. Returns the resolved absolute path on
/// success, or an error string describing the violation.
pub fn confine_path(workspace_root: &Path, requested: &Path) -> Result<PathBuf, String> {
    let joined = workspace_root.join(requested);
    for component in joined.components() {
        if matches!(component, Component::ParentDir) {
            return Err(format!(
                "path '{}' contains `..` after resolving against workspace; refusing (vertical isolation, #677)",
                requested.display()
            ));
        }
    }
    if requested.is_absolute() && !joined.starts_with(workspace_root) {
        return Err(format!(
            "absolute path '{}' is outside the workspace root '{}'; refusing (vertical isolation, #677)",
            requested.display(),
            workspace_root.display()
        ));
    }
    Ok(joined)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ws() -> PathBuf {
        PathBuf::from("/tmp/argo-ws")
    }

    #[test]
    fn relative_path_inside_workspace_passes() {
        let resolved = confine_path(&ws(), Path::new("hello.txt")).unwrap();
        assert_eq!(resolved, PathBuf::from("/tmp/argo-ws/hello.txt"));
    }

    #[test]
    fn relative_path_in_subdir_passes() {
        let resolved = confine_path(&ws(), Path::new("a/b/c.txt")).unwrap();
        assert_eq!(resolved, PathBuf::from("/tmp/argo-ws/a/b/c.txt"));
    }

    #[test]
    fn relative_parent_traversal_rejected() {
        let err = confine_path(&ws(), Path::new("../escape.txt")).unwrap_err();
        assert!(err.contains("`..`"), "got: {err}");
    }

    #[test]
    fn nested_parent_traversal_rejected() {
        // `a/../../escape.txt` after join becomes `/tmp/argo-ws/a/../../escape.txt`
        // — the resolved path STILL contains `..` lexically,
        // even though it'd resolve to `/tmp/escape.txt` after
        // canonicalization. The lexical check fires.
        let err = confine_path(&ws(), Path::new("a/../../escape.txt")).unwrap_err();
        assert!(err.contains("`..`"), "got: {err}");
    }

    #[test]
    fn absolute_path_inside_workspace_passes() {
        // /tmp/argo-ws/foo.txt is inside the workspace.
        let resolved =
            confine_path(&ws(), Path::new("/tmp/argo-ws/foo.txt")).unwrap();
        assert_eq!(resolved, PathBuf::from("/tmp/argo-ws/foo.txt"));
    }

    #[test]
    fn absolute_path_outside_workspace_rejected() {
        let err = confine_path(&ws(), Path::new("/etc/passwd")).unwrap_err();
        assert!(err.contains("outside the workspace root"), "got: {err}");
    }

    #[test]
    fn absolute_root_path_rejected() {
        let err = confine_path(&ws(), Path::new("/")).unwrap_err();
        assert!(err.contains("outside the workspace root"), "got: {err}");
    }

    #[test]
    fn empty_path_resolves_to_workspace_root() {
        // Edge case — caller probably has a bug if they pass
        // empty, but it's not a security violation.
        let resolved = confine_path(&ws(), Path::new("")).unwrap();
        assert_eq!(resolved, ws());
    }
}
