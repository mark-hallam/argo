//! `argo vertical install <source>` and `argo vertical uninstall
//! <id>` (#661 — Phase 8 first slice).
//!
//! These are the simplest possible lifecycle ops: copy a
//! validated source dir into the discovery dir, or delete a
//! previously-installed dir. Tarball + OCI flows land later;
//! this module sticks to local directory operations so it ships
//! independently revertable with no new transitive deps.
//!
//! Scope:
//!
//! - **install**: validate `<source>/vertical.toml` via
//!   `argo_vertical::parse` + `validate`, then copy the whole
//!   tree to `<target>/<vertical-id>/`. Refuse to overwrite
//!   unless `--force`. Re-validate post-copy as a sanity check.
//! - **uninstall**: refuse without `--yes` (the confirmation is
//!   the safety boundary). With `--yes`, remove the dir from
//!   `<target>/<id>/`.
//!
//! Both commands respect `$ARGO_VERTICALS_DIR` via
//! `argo_vertical::default_dir()` so the operator's discovery
//! policy is the single source of truth.
use argo_types::{ArgoError, ArgoResult};
use std::path::{Path, PathBuf};

/// Top-level entry for `argo vertical install`.
pub async fn install(
    source: &str,
    target: Option<&str>,
    force: bool,
    json: bool,
) -> ArgoResult<()> {
    let src = PathBuf::from(source);
    if !src.is_dir() {
        return Err(ArgoError::Provider(format!(
            "source '{}' is not a directory (tarball + OCI flows ship in a follow-up PR)",
            src.display()
        )));
    }
    let manifest_path = src.join("vertical.toml");
    let manifest_src = std::fs::read_to_string(&manifest_path).map_err(|e| {
        ArgoError::Io(format!(
            "read {}: {e} — does the source dir contain `vertical.toml`?",
            manifest_path.display()
        ))
    })?;
    let manifest = argo_vertical::parse(&manifest_src)?;
    argo_vertical::validate(&manifest, &src)?;

    let target_root = match target {
        Some(t) => PathBuf::from(t),
        None => argo_vertical::default_dir(),
    };
    let dest = target_root.join(&manifest.vertical.id);
    if dest.exists() {
        if !force {
            return Err(ArgoError::Provider(format!(
                "vertical '{}' is already installed at {} — pass --force to replace",
                manifest.vertical.id,
                dest.display()
            )));
        }
        std::fs::remove_dir_all(&dest).map_err(|e| {
            ArgoError::Io(format!("remove {}: {e}", dest.display()))
        })?;
    }
    std::fs::create_dir_all(&target_root)
        .map_err(|e| ArgoError::Io(format!("mkdir {}: {e}", target_root.display())))?;
    copy_dir_recursive(&src, &dest)?;

    // Re-validate against the destination so the cross-ref
    // checks (file references in `[role].system_prompt`,
    // `[skills].bundle`, etc.) don't silently rot.
    let dest_manifest_src = std::fs::read_to_string(dest.join("vertical.toml"))
        .map_err(|e| ArgoError::Io(format!("re-read manifest after copy: {e}")))?;
    let dest_manifest = argo_vertical::parse(&dest_manifest_src)?;
    argo_vertical::validate(&dest_manifest, &dest)?;

    if json {
        println!(
            "{}",
            serde_json::to_string(&serde_json::json!({
                "ok": true,
                "id": manifest.vertical.id,
                "name": manifest.vertical.name,
                "version": manifest.vertical.version,
                "root": dest.display().to_string(),
            }))
            .unwrap_or_default()
        );
    } else {
        println!(
            "installed vertical '{}' v{} into {}",
            manifest.vertical.id,
            manifest.vertical.version,
            dest.display()
        );
        println!("  → run `argo vertical show {}` to inspect", manifest.vertical.id);
    }
    Ok(())
}

/// Top-level entry for `argo vertical uninstall`.
pub async fn uninstall(
    id: &str,
    target: Option<&str>,
    yes: bool,
    json: bool,
) -> ArgoResult<()> {
    let target_root = match target {
        Some(t) => PathBuf::from(t),
        None => argo_vertical::default_dir(),
    };
    let dest = target_root.join(id);
    if !dest.exists() {
        return Err(ArgoError::Provider(format!(
            "no vertical with id '{id}' installed at {}",
            target_root.display()
        )));
    }
    if !yes {
        // Print plan + exit non-zero so scripts can't silently
        // proceed. Same UX as `git clean -n`.
        if json {
            println!(
                "{}",
                serde_json::to_string(&serde_json::json!({
                    "ok": false,
                    "id": id,
                    "would_remove": dest.display().to_string(),
                    "hint": "pass --yes to actually remove",
                }))
                .unwrap_or_default()
            );
        } else {
            println!(
                "would remove {} (pass --yes to actually delete)",
                dest.display()
            );
        }
        return Err(ArgoError::Provider(
            "uninstall requires --yes (safety guard)".into(),
        ));
    }
    std::fs::remove_dir_all(&dest)
        .map_err(|e| ArgoError::Io(format!("remove {}: {e}", dest.display())))?;
    if json {
        println!(
            "{}",
            serde_json::to_string(&serde_json::json!({
                "ok": true,
                "id": id,
                "removed_path": dest.display().to_string(),
            }))
            .unwrap_or_default()
        );
    } else {
        println!("uninstalled vertical '{id}' from {}", dest.display());
    }
    Ok(())
}

/// Copy `src` into `dst` recursively. `dst` must NOT exist
/// (caller's responsibility — install() handles the --force
/// remove path).
fn copy_dir_recursive(src: &Path, dst: &Path) -> ArgoResult<()> {
    std::fs::create_dir_all(dst)
        .map_err(|e| ArgoError::Io(format!("mkdir {}: {e}", dst.display())))?;
    for entry in
        std::fs::read_dir(src).map_err(|e| ArgoError::Io(format!("read_dir {}: {e}", src.display())))?
    {
        let entry =
            entry.map_err(|e| ArgoError::Io(format!("read_dir entry under {}: {e}", src.display())))?;
        let ft = entry.file_type().map_err(|e| {
            ArgoError::Io(format!("file_type {}: {e}", entry.path().display()))
        })?;
        let from = entry.path();
        let to = dst.join(entry.file_name());
        if ft.is_dir() {
            copy_dir_recursive(&from, &to)?;
        } else if ft.is_file() {
            std::fs::copy(&from, &to)
                .map_err(|e| ArgoError::Io(format!("copy {} → {}: {e}", from.display(), to.display())))?;
        }
        // Symlinks + special files: skipped for now. Verticals
        // are pure source bundles; if we ever need symlinks the
        // restriction can be relaxed with a feature flag.
    }
    Ok(())
}

#[cfg(test)]
#[path = "vertical_install_tests.rs"]
mod tests;
