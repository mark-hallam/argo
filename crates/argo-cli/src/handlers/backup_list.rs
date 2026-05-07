//! `argo backup list`.

use argo_types::{ArgoError, ArgoResult};
use serde::Serialize;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
struct BackupListItem {
    path: String,
    bytes: u64,
}

pub async fn run(json: bool) -> ArgoResult<()> {
    let workspace = std::env::current_dir()
        .map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))?;
    let backups = list_backups(&workspace)?;
    if json {
        println!(
            "{}",
            serde_json::to_string(&serde_json::json!({"ok": true, "backups": backups}))
                .map_err(|e| ArgoError::Provider(format!("backup list JSON: {e}")))?
        );
    } else if backups.is_empty() {
        println!("(no backups)");
    } else {
        for backup in backups {
            println!("{}\t{} bytes", backup.path, backup.bytes);
        }
    }
    Ok(())
}

fn list_backups(workspace: &Path) -> ArgoResult<Vec<BackupListItem>> {
    let backups_dir = workspace.join(".argo").join("backups");
    if !backups_dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut backups = Vec::new();
    for entry in fs::read_dir(backups_dir)? {
        let entry = entry?;
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
            continue;
        };
        if !name.ends_with(".tar.gz") {
            continue;
        }
        let metadata = entry.metadata()?;
        if metadata.is_file() {
            backups.push(BackupListItem {
                path: path.display().to_string(),
                bytes: metadata.len(),
            });
        }
    }
    backups.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(backups)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_backups_returns_sorted_tar_gz_files() {
        let temp = tempfile::tempdir().unwrap();
        let backups = temp.path().join(".argo").join("backups");
        fs::create_dir_all(&backups).unwrap();
        fs::write(backups.join("b.tar.gz"), "b").unwrap();
        fs::write(backups.join("a.tar.gz"), "a").unwrap();
        fs::write(backups.join("note.txt"), "skip").unwrap();

        let listed = list_backups(temp.path()).unwrap();
        let paths: Vec<_> = listed
            .iter()
            .map(|backup| Path::new(&backup.path).file_name().unwrap().to_str().unwrap())
            .collect();

        assert_eq!(paths, vec!["a.tar.gz", "b.tar.gz"]);
    }
}
