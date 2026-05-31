//! Vertical isolation guarantees (#676).
//!
//! Strict isolation invariant: a vertical's manifest may
//! reference assets only inside its own directory. Cross-
//! vertical references — `[role].system_prompt = "../other-
//! vertical/prompts/system.md"` — are rejected at load time.
//!
//! This is the **static** layer of vertical isolation. Runtime
//! layers (memory namespacing per session.vertical_id, learned
//! skill scoping, etc.) ship in follow-up PRs. This PR ensures
//! the manifest itself cannot declare cross-vertical leakage.
//!
//! Three checks per relative-path field in the manifest:
//!
//! 1. The path must NOT be absolute — verticals are
//!    location-portable; absolute paths break that.
//! 2. The path must NOT contain a `..` component — direct
//!    traversal escape.
//! 3. After file-system canonicalization (when the file
//!    exists), the resolved path must remain inside the
//!    vertical's canonicalized root — catches symlink
//!    traversal.
//!
//! Fields covered (every relative-path field the manifest
//! schema declares):
//!
//! - `[role].system_prompt`, `[role].persona`
//! - `[memory].schema`, `[memory].seed`
//! - `[skills].bundle`
//! - `[workflows].files[*]`
//! - `[evaluation].suites[*]`
//!
//! Returns the FIRST violation as `ArgoError::Schema` so the
//! load-time error message is precise. Fail-fast keeps the
//! security boundary as tight as the deny-list.
use crate::manifest::VerticalManifest;
use argo_types::{ArgoError, ArgoResult};
use std::path::{Component, Path};

/// Verify every relative-path field in the manifest stays
/// inside `root`. Called by [`crate::validate`] before the
/// existence checks so a path-traversal attempt fails loud
/// before any I/O.
pub fn check_isolation(manifest: &VerticalManifest, root: &Path) -> ArgoResult<()> {
    if let Some(role) = &manifest.role {
        check_path_within(&role.system_prompt, root, "[role].system_prompt")?;
        if let Some(persona) = &role.persona {
            check_path_within(persona, root, "[role].persona")?;
        }
    }
    if let Some(mem) = &manifest.memory {
        if let Some(schema) = &mem.schema {
            check_path_within(schema, root, "[memory].schema")?;
        }
        if let Some(seed) = &mem.seed {
            check_path_within(seed, root, "[memory].seed")?;
        }
    }
    if let Some(skills) = &manifest.skills {
        check_path_within(&skills.bundle, root, "[skills].bundle")?;
    }
    if let Some(wf) = &manifest.workflows {
        for f in &wf.files {
            check_path_within(f, root, "[workflows].files")?;
        }
    }
    if let Some(eval) = &manifest.evaluation {
        for s in &eval.suites {
            check_path_within(s, root, "[evaluation].suites")?;
        }
    }
    Ok(())
}

/// Reject paths absolute under EITHER the Unix or the Windows convention,
/// regardless of the host OS this check runs on.
///
/// [`Path::is_absolute`] is host-dependent: on Windows it returns `false`
/// for the Unix-absolute `/etc/passwd` (no drive prefix), and on Unix it
/// returns `false` for `C:\Windows`. A vertical manifest is portable across
/// hosts, so an asset path absolute under *either* convention must be
/// refused wherever the check runs — otherwise a manifest authored to
/// escape on Unix would pass validation on a Windows builder (and vice
/// versa). This is a strict superset of `Path::is_absolute` on both
/// platforms, so it replaces it outright (security boundary, #676).
fn is_absolute_any_os(rel: &str) -> bool {
    let bytes = rel.as_bytes();
    // Unix-absolute (`/etc/…`) OR Windows rooted / UNC (`\foo`,
    // `\\server\share`, `\\?\C:\…`): a leading `/` or `\`.
    if matches!(bytes.first(), Some(b'/') | Some(b'\\')) {
        return true;
    }
    // Windows drive-qualified (`C:\…`, `C:/…`) or drive-relative (`C:rel`):
    // an ASCII letter followed by `:`. Any drive prefix is non-portable.
    bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':'
}

fn check_path_within(rel: &str, root: &Path, label: &str) -> ArgoResult<()> {
    let p = Path::new(rel);
    if is_absolute_any_os(rel) {
        return Err(ArgoError::Schema(format!(
            "{label}: path '{rel}' is absolute; vertical asset paths must be relative to the vertical's root (security boundary, #676)"
        )));
    }
    for component in p.components() {
        if let Component::ParentDir = component {
            return Err(ArgoError::Schema(format!(
                "{label}: path '{rel}' contains `..` and would escape the vertical's root '{}' (cross-vertical reference forbidden, #676)",
                root.display()
            )));
        }
    }
    // Symlink traversal check — only meaningful when both the
    // root and the target file actually exist on disk.
    let abs = root.join(rel);
    if let (Ok(canon_root), Ok(canon_abs)) = (root.canonicalize(), abs.canonicalize()) {
        if !canon_abs.starts_with(&canon_root) {
            return Err(ArgoError::Schema(format!(
                "{label}: path '{rel}' resolves (via symlink) to '{}' which is outside the vertical's root '{}' (security boundary, #676)",
                canon_abs.display(),
                canon_root.display()
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "isolation_tests.rs"]
mod tests;
