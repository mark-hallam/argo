//! Versioning helpers for preserving old vertical directories.
use crate::{parse, VerticalManifest};
use argo_types::{ArgoError, ArgoResult};
use std::path::{Path, PathBuf};

/// Return the first free `<id>-v<n>` sibling directory.
#[must_use]
pub fn next_version_dir(parent: &Path, id: &str) -> PathBuf {
    for n in 1.. {
        let candidate = parent.join(format!("{id}-v{n}"));
        if !candidate.exists() {
            return candidate;
        }
    }
    unreachable!("unbounded integer iterator always yields")
}

/// Move `root` to a versioned sibling and update the archived manifest id.
pub fn archive_existing_vertical(root: &Path, id: &str) -> ArgoResult<PathBuf> {
    let parent = root.parent().ok_or_else(|| {
        ArgoError::Schema(format!("vertical root '{}' has no parent", root.display()))
    })?;
    let archive = next_version_dir(parent, id);
    std::fs::rename(root, &archive).map_err(|e| {
        ArgoError::Io(format!(
            "archive vertical {} -> {}: {e}",
            root.display(),
            archive.display()
        ))
    })?;
    rewrite_archived_manifest(
        &archive,
        archive.file_name().and_then(|s| s.to_str()).unwrap_or(id),
    )?;
    Ok(archive)
}

fn rewrite_archived_manifest(root: &Path, archived_id: &str) -> ArgoResult<()> {
    let manifest_path = root.join("vertical.toml");
    let src = std::fs::read_to_string(&manifest_path)
        .map_err(|e| ArgoError::Io(format!("read {}: {e}", manifest_path.display())))?;
    let mut manifest: VerticalManifest = parse(&src)?;
    manifest.vertical.id = archived_id.to_string();
    let body = toml::to_string_pretty(&manifest)
        .map_err(|e| ArgoError::Schema(format!("serialise archived vertical: {e}")))?;
    std::fs::write(&manifest_path, body)
        .map_err(|e| ArgoError::Io(format!("write {}: {e}", manifest_path.display())))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn next_version_dir_skips_existing_siblings() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("demo-v1")).unwrap();
        assert_eq!(
            next_version_dir(dir.path(), "demo"),
            dir.path().join("demo-v2")
        );
    }

    #[test]
    fn archive_rewrites_manifest_id() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("demo");
        std::fs::create_dir(&root).unwrap();
        std::fs::write(
            root.join("vertical.toml"),
            "schema_version = \"1\"\n\n[vertical]\nid = \"demo\"\nname = \"Demo\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        let archived = archive_existing_vertical(&root, "demo").unwrap();
        assert_eq!(archived.file_name().unwrap(), "demo-v1");
        let src = std::fs::read_to_string(archived.join("vertical.toml")).unwrap();
        assert!(src.contains("id = \"demo-v1\""));
    }
}
