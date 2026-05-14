//! `argo restore`.

use super::backup_verify::{verify_backup, ArchiveFile};
use argo_types::{ArgoError, ArgoResult};
use serde::Serialize;
use std::fs;
use std::path::{Component, Path, PathBuf};

#[derive(Debug, Serialize)]
struct RestoreOut {
    path: String,
    workspace: String,
    dry_run: bool,
    files: Vec<String>,
}

pub async fn run(
    path: &Path,
    into: Option<&Path>,
    dry_run: bool,
    json: bool,
) -> ArgoResult<()> {
    let target = restore_target(into)?;
    let inspection = verify_backup(path)?;
    let file_names: Vec<String> = inspection
        .files
        .iter()
        .map(|file| file.path.clone())
        .collect();
    if !dry_run {
        ensure_fresh_workspace(&target)?;
        fs::create_dir_all(&target)?;
        for file in &inspection.files {
            restore_file(&target, file)?;
        }
    }
    let out = RestoreOut {
        path: path.display().to_string(),
        workspace: target.display().to_string(),
        dry_run,
        files: file_names,
    };
    if json {
        println!(
            "{}",
            serde_json::to_string(&serde_json::json!({"ok": true, "restore": out}))
                .map_err(|e| ArgoError::Provider(format!("restore JSON: {e}")))?
        );
    } else if dry_run {
        for file in &out.files {
            println!("{file}");
        }
    } else {
        println!("restored backup into {}", out.workspace);
    }
    Ok(())
}

fn restore_target(into: Option<&Path>) -> ArgoResult<PathBuf> {
    match into {
        Some(path) if path.is_absolute() => Ok(path.to_path_buf()),
        Some(path) => {
            let cwd = std::env::current_dir()
                .map_err(|e| ArgoError::Io(format!("current_dir: {e}")))?;
            Ok(cwd.join(path))
        }
        None => std::env::current_dir()
            .map_err(|e| ArgoError::Io(format!("current_dir: {e}"))),
    }
}

fn ensure_fresh_workspace(path: &Path) -> ArgoResult<()> {
    if !workspace_is_empty(path)? {
        return Err(ArgoError::Provider(format!(
            "restore target is not empty: {}",
            path.display()
        )));
    }
    Ok(())
}

fn workspace_is_empty(path: &Path) -> ArgoResult<bool> {
    if !path.exists() {
        return Ok(true);
    }
    if !path.is_dir() {
        return Ok(false);
    }
    Ok(fs::read_dir(path)?.next().is_none())
}

fn restore_file(workspace: &Path, file: &ArchiveFile) -> ArgoResult<()> {
    let destination = safe_restore_path(workspace, &file.path)?;
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(destination, &file.bytes)?;
    Ok(())
}

fn safe_restore_path(workspace: &Path, archive_path: &str) -> ArgoResult<PathBuf> {
    let rel = Path::new(archive_path);
    let mut out = workspace.to_path_buf();
    let mut components = rel.components();
    match components.next() {
        Some(Component::Normal(root)) if root == ".argo" => out.push(root),
        _ => {
            return Err(ArgoError::Provider(format!(
                "restore entry must be under .argo/: {archive_path}"
            )));
        }
    }
    for component in components {
        match component {
            Component::Normal(value) => out.push(value),
            _ => {
                return Err(ArgoError::Provider(format!(
                    "unsafe restore entry path: {archive_path}"
                )));
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workspace_is_empty_rejects_non_empty_directory() {
        let temp = tempfile::tempdir().unwrap();
        fs::write(temp.path().join("marker"), "x").unwrap();

        assert!(!workspace_is_empty(temp.path()).unwrap());
    }

    #[test]
    fn safe_restore_path_rejects_absolute_paths() {
        let temp = tempfile::tempdir().unwrap();
        let err = safe_restore_path(temp.path(), "/.argo/state.db")
            .unwrap_err()
            .to_string();

        assert!(err.contains("restore entry must be under .argo"));
    }
}
