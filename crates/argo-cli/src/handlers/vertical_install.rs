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

/// Top-level entry for `argo vertical install`. Source may be
/// either a directory or a `.tar` archive (#675); auto-detected
/// by extension.
pub async fn install(
    source: &str,
    target: Option<&str>,
    force: bool,
    json: bool,
) -> ArgoResult<()> {
    let src = PathBuf::from(source);
    // Tarball path: extract into a temp dir + recurse with the
    // extracted dir as the source. This keeps the dir-source
    // path the canonical implementation.
    if src.extension().and_then(|e| e.to_str()) == Some("tar") && src.is_file() {
        let tmp = tempfile::tempdir().map_err(|e| {
            ArgoError::Io(format!("create tempdir for tarball extract: {e}"))
        })?;
        extract_tar_into(&src, tmp.path())?;
        let extracted = pick_single_subdir(tmp.path())?;
        let extracted_str = extracted.to_string_lossy().into_owned();
        return Box::pin(install(&extracted_str, target, force, json)).await;
    }
    if !src.is_dir() {
        return Err(ArgoError::Provider(format!(
            "source '{}' is neither a directory nor a `.tar` file",
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

/// Pack an installed vertical into a single `.tar` archive.
/// Inverse of the tarball-source install path; ships with the
/// vertical's id stamped as the top-level directory inside the
/// archive so an `install` round-trip recovers the source name.
pub async fn pack(
    id: &str,
    out: Option<&str>,
    target: Option<&str>,
    json: bool,
) -> ArgoResult<()> {
    let target_root = match target {
        Some(t) => PathBuf::from(t),
        None => argo_vertical::default_dir(),
    };
    let src_dir = target_root.join(id);
    if !src_dir.is_dir() {
        return Err(ArgoError::Provider(format!(
            "no vertical with id '{id}' installed at {}",
            target_root.display()
        )));
    }
    // Validate before packing — refuse to ship a broken
    // vertical as an artifact (operator could inadvertently
    // distribute a half-edited manifest).
    let manifest_src = std::fs::read_to_string(src_dir.join("vertical.toml"))
        .map_err(|e| ArgoError::Io(format!("read manifest: {e}")))?;
    let manifest = argo_vertical::parse(&manifest_src)?;
    argo_vertical::validate(&manifest, &src_dir)?;

    let out_path = match out {
        Some(p) => PathBuf::from(p),
        None => PathBuf::from(format!("{id}.tar")),
    };
    write_tar_from_dir(&src_dir, id, &out_path)?;
    let bytes = std::fs::metadata(&out_path)
        .map(|m| m.len())
        .unwrap_or(0);

    if json {
        println!(
            "{}",
            serde_json::to_string(&serde_json::json!({
                "ok": true,
                "id": id,
                "path": out_path.display().to_string(),
                "bytes": bytes,
            }))
            .unwrap_or_default()
        );
    } else {
        println!(
            "packed vertical '{id}' → {} ({bytes} bytes)",
            out_path.display()
        );
        println!("  → ship + restore with `argo vertical install {}`", out_path.display());
    }
    Ok(())
}

fn write_tar_from_dir(src_dir: &Path, id: &str, out_path: &Path) -> ArgoResult<()> {
    let file = std::fs::File::create(out_path).map_err(|e| {
        ArgoError::Io(format!("create {}: {e}", out_path.display()))
    })?;
    let mut builder = tar::Builder::new(file);
    builder
        .append_dir_all(id, src_dir)
        .map_err(|e| ArgoError::Io(format!("tar append: {e}")))?;
    builder
        .finish()
        .map_err(|e| ArgoError::Io(format!("tar finish: {e}")))?;
    Ok(())
}

fn extract_tar_into(src: &Path, into: &Path) -> ArgoResult<()> {
    let file = std::fs::File::open(src)
        .map_err(|e| ArgoError::Io(format!("open {}: {e}", src.display())))?;
    let mut archive = tar::Archive::new(file);
    archive
        .unpack(into)
        .map_err(|e| ArgoError::Io(format!("tar unpack into {}: {e}", into.display())))?;
    Ok(())
}

/// Tarballs produced by `pack` always carry a single top-level
/// dir (the vertical's id). Pick that subdir from the extract
/// dest so the caller installs the right tree.
fn pick_single_subdir(into: &Path) -> ArgoResult<PathBuf> {
    let mut entries: Vec<PathBuf> = Vec::new();
    for entry in std::fs::read_dir(into)
        .map_err(|e| ArgoError::Io(format!("read_dir {}: {e}", into.display())))?
    {
        let entry =
            entry.map_err(|e| ArgoError::Io(format!("read_dir entry under {}: {e}", into.display())))?;
        if entry
            .file_type()
            .map(|t| t.is_dir())
            .unwrap_or(false)
        {
            entries.push(entry.path());
        }
    }
    if entries.len() == 1 {
        // SAFETY: just checked len == 1 — `next()` always yields
        // Some — but `unwrap_used` is denied at the crate root, so
        // use a let-else to lint cleanly without a runtime panic
        // path.
        let Some(only) = entries.into_iter().next() else {
            return Err(ArgoError::Provider(
                "pick_single_subdir: len==1 but iterator empty (unreachable)".into(),
            ));
        };
        return Ok(only);
    }
    if entries.is_empty() {
        // Tarball was packed flat (no top-level dir); install
        // from the extract root directly.
        return Ok(into.to_path_buf());
    }
    Err(ArgoError::Provider(format!(
        "tarball at {} has multiple top-level directories ({}); expected exactly one (vertical id)",
        into.display(),
        entries.len()
    )))
}

#[cfg(test)]
#[path = "vertical_install_tests.rs"]
mod tests;
