//! The 6 sandboxed filesystem ops the brief calls for. Every op
//! goes through `Sandbox::resolve` first (rejects traversal +
//! absolute paths) and then `assert_no_symlink_escape` so a
//! symlink planted between checks doesn't get followed.
//!
//! Auto-approval mapping (consumed by the AutoApprovalPolicy gate
//! at action-emission time): `read_file`, `list_dir`, `stat` are
//! read-only and may auto-approve under workspace policy;
//! `write_file`, `mkdir`, `remove` are mutating and stay
//! approval-gated by default.

use crate::error::{FsExecutorError, FsResult};
use crate::sandbox::Sandbox;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Lightweight directory entry returned by `list_dir`. We
/// deliberately don't surface the inode/permissions/modtime — those
/// are platform-specific and would leak info the audit chain
/// shouldn't need to canonicalise.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DirEntry {
    /// File or directory name (last segment only — no leading
    /// path).
    pub name: String,
    /// `true` for a directory, `false` for a regular file or
    /// symlink. Symlinks are reported as files.
    pub is_dir: bool,
    /// Size in bytes for files; `0` for directories.
    pub size: u64,
}

/// Stat result. Same fields as `DirEntry` plus `exists` so callers
/// can distinguish "missing" from "empty".
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileMeta {
    pub exists: bool,
    pub is_dir: bool,
    pub size: u64,
}

/// Tag used by the AutoApprovalPolicy gate to decide whether an op
/// is read-only (auto-approvable) or mutating (approval-gated).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FsOpKind {
    ReadFile,
    WriteFile,
    ListDir,
    Mkdir,
    Remove,
    Stat,
}

impl FsOpKind {
    /// Read-only ops are eligible for auto-approval. Mutating ops
    /// always flow through the human approval gate by default.
    #[must_use]
    pub fn is_read_only(self) -> bool {
        matches!(self, Self::ReadFile | Self::ListDir | Self::Stat)
    }
}

/// Read the whole file at `rel` into memory. Per-op size cap is
/// enforced after we stat the file — refusing reads of giant files
/// before they hit RAM.
pub fn read_file(sb: &Sandbox, rel: impl AsRef<std::path::Path>) -> FsResult<Vec<u8>> {
    let abs = sb.resolve(rel.as_ref())?;
    sb.assert_no_symlink_escape(&abs)?;
    let meta = std::fs::metadata(&abs)
        .map_err(|e| FsExecutorError::io("metadata", &abs, e))?;
    sb.preflight_op_size(meta.len())?;
    std::fs::read(&abs).map_err(|e| FsExecutorError::io("read", &abs, e))
}

/// Write `bytes` to `rel`, creating parent directories as needed.
/// Refused if either the per-op or the per-worker total cap would
/// be exceeded.
pub fn write_file(
    sb: &Sandbox,
    rel: impl AsRef<std::path::Path>,
    bytes: &[u8],
) -> FsResult<PathBuf> {
    let abs = sb.resolve(rel.as_ref())?;
    sb.assert_no_symlink_escape(&abs)?;
    sb.preflight_op_size(bytes.len() as u64)?;
    sb.preflight_total_size(bytes.len() as u64)?;
    if let Some(parent) = abs.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent)
                .map_err(|e| FsExecutorError::io("mkdir_parent", parent, e))?;
        }
    }
    std::fs::write(&abs, bytes).map_err(|e| FsExecutorError::io("write", &abs, e))?;
    Ok(abs)
}

/// List the immediate children of `rel`. Returns the entries sorted
/// alphabetically so the audit chain is deterministic.
pub fn list_dir(sb: &Sandbox, rel: impl AsRef<std::path::Path>) -> FsResult<Vec<DirEntry>> {
    let abs = sb.resolve(rel.as_ref())?;
    sb.assert_no_symlink_escape(&abs)?;
    let mut out: Vec<DirEntry> = Vec::new();
    let read = std::fs::read_dir(&abs)
        .map_err(|e| FsExecutorError::io("read_dir", &abs, e))?;
    for entry in read {
        let entry = entry.map_err(|e| FsExecutorError::io("read_dir_entry", &abs, e))?;
        let meta = entry
            .metadata()
            .map_err(|e| FsExecutorError::io("metadata", entry.path(), e))?;
        let is_dir = meta.is_dir();
        out.push(DirEntry {
            name: entry.file_name().to_string_lossy().into_owned(),
            is_dir,
            size: if is_dir { 0 } else { meta.len() },
        });
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

/// Create a directory (and any missing parents) at `rel`. No-op if
/// the directory already exists.
pub fn mkdir(sb: &Sandbox, rel: impl AsRef<std::path::Path>) -> FsResult<PathBuf> {
    let abs = sb.resolve(rel.as_ref())?;
    sb.assert_no_symlink_escape(&abs)?;
    std::fs::create_dir_all(&abs).map_err(|e| FsExecutorError::io("mkdir", &abs, e))?;
    Ok(abs)
}

/// Remove the single entry at `rel`. Refuses non-empty directories
/// — recursive deletion needs an explicit recursive flag, which is
/// a follow-up brief. Files (and empty dirs) are deleted.
pub fn remove(sb: &Sandbox, rel: impl AsRef<std::path::Path>) -> FsResult<()> {
    let abs = sb.resolve(rel.as_ref())?;
    sb.assert_no_symlink_escape(&abs)?;
    let meta = std::fs::symlink_metadata(&abs)
        .map_err(|e| FsExecutorError::io("symlink_metadata", &abs, e))?;
    if meta.is_dir() {
        let mut iter = std::fs::read_dir(&abs)
            .map_err(|e| FsExecutorError::io("read_dir", &abs, e))?;
        if iter.next().is_some() {
            return Err(FsExecutorError::NonEmptyDir(abs));
        }
        std::fs::remove_dir(&abs).map_err(|e| FsExecutorError::io("remove_dir", &abs, e))
    } else {
        std::fs::remove_file(&abs).map_err(|e| FsExecutorError::io("remove_file", &abs, e))
    }
}

/// Stat the entry at `rel`. Non-existent paths come back as
/// `FileMeta { exists: false, is_dir: false, size: 0 }` so callers
/// don't have to special-case `NotFound`.
pub fn stat(sb: &Sandbox, rel: impl AsRef<std::path::Path>) -> FsResult<FileMeta> {
    let abs = sb.resolve(rel.as_ref())?;
    sb.assert_no_symlink_escape(&abs)?;
    if !abs.exists() {
        return Ok(FileMeta {
            exists: false,
            is_dir: false,
            size: 0,
        });
    }
    let meta = std::fs::metadata(&abs)
        .map_err(|e| FsExecutorError::io("metadata", &abs, e))?;
    Ok(FileMeta {
        exists: true,
        is_dir: meta.is_dir(),
        size: if meta.is_dir() { 0 } else { meta.len() },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn sb() -> (tempfile::TempDir, Sandbox) {
        let dir = tempdir().unwrap();
        let sb = Sandbox::open(dir.path()).unwrap();
        (dir, sb)
    }

    #[test]
    fn write_then_read_round_trips() {
        let (_dir, sb) = sb();
        write_file(&sb, "hello.txt", b"hi there").unwrap();
        let body = read_file(&sb, "hello.txt").unwrap();
        assert_eq!(body, b"hi there");
    }

    #[test]
    fn write_creates_parent_directories() {
        let (_dir, sb) = sb();
        write_file(&sb, "nested/dir/note.md", b"ok").unwrap();
        let m = stat(&sb, "nested/dir/note.md").unwrap();
        assert!(m.exists);
        assert_eq!(m.size, 2);
    }

    #[test]
    fn read_rejects_path_traversal() {
        let (_dir, sb) = sb();
        let err = read_file(&sb, "../etc/passwd").unwrap_err();
        assert!(matches!(err, FsExecutorError::PathEscape(_)));
    }

    #[test]
    fn write_rejects_oversize_payload() {
        let dir = tempdir().unwrap();
        let sb = Sandbox::open(dir.path()).unwrap().with_caps(8, 1024);
        let err = write_file(&sb, "big.bin", b"123456789").unwrap_err();
        assert!(matches!(err, FsExecutorError::OpSizeCap { .. }));
    }

    #[test]
    fn write_rejects_when_total_cap_exceeded() {
        let dir = tempdir().unwrap();
        let sb = Sandbox::open(dir.path()).unwrap().with_caps(1024, 100);
        write_file(&sb, "a.bin", &[0u8; 80]).unwrap();
        let err = write_file(&sb, "b.bin", &[0u8; 50]).unwrap_err();
        assert!(matches!(err, FsExecutorError::TotalSizeCap { .. }));
    }

    #[test]
    fn list_dir_returns_sorted_entries() {
        let (_dir, sb) = sb();
        write_file(&sb, "b.txt", b"b").unwrap();
        write_file(&sb, "a.txt", b"a").unwrap();
        mkdir(&sb, "sub").unwrap();
        let entries = list_dir(&sb, ".").unwrap();
        let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, vec!["a.txt", "b.txt", "sub"]);
        assert!(entries.iter().find(|e| e.name == "sub").unwrap().is_dir);
    }

    #[test]
    fn mkdir_is_idempotent_for_existing_dirs() {
        let (_dir, sb) = sb();
        mkdir(&sb, "a/b/c").unwrap();
        mkdir(&sb, "a/b/c").unwrap();
        let m = stat(&sb, "a/b/c").unwrap();
        assert!(m.is_dir);
    }

    #[test]
    fn remove_deletes_files_and_empty_dirs() {
        let (_dir, sb) = sb();
        write_file(&sb, "gone.txt", b"x").unwrap();
        remove(&sb, "gone.txt").unwrap();
        let m = stat(&sb, "gone.txt").unwrap();
        assert!(!m.exists);
        mkdir(&sb, "emptydir").unwrap();
        remove(&sb, "emptydir").unwrap();
        assert!(!stat(&sb, "emptydir").unwrap().exists);
    }

    #[test]
    fn remove_refuses_non_empty_directory() {
        let (_dir, sb) = sb();
        write_file(&sb, "drawer/sock.txt", b"sock").unwrap();
        let err = remove(&sb, "drawer").unwrap_err();
        assert!(matches!(err, FsExecutorError::NonEmptyDir(_)));
    }

    #[test]
    fn stat_reports_missing_path_without_error() {
        let (_dir, sb) = sb();
        let m = stat(&sb, "ghost.txt").unwrap();
        assert!(!m.exists);
    }

    #[test]
    fn stat_reports_size_for_existing_file() {
        let (_dir, sb) = sb();
        write_file(&sb, "size.bin", &[0u8; 12]).unwrap();
        let m = stat(&sb, "size.bin").unwrap();
        assert!(m.exists);
        assert_eq!(m.size, 12);
        assert!(!m.is_dir);
    }

    #[test]
    fn fs_op_kind_read_only_classification_matches_brief() {
        assert!(FsOpKind::ReadFile.is_read_only());
        assert!(FsOpKind::ListDir.is_read_only());
        assert!(FsOpKind::Stat.is_read_only());
        assert!(!FsOpKind::WriteFile.is_read_only());
        assert!(!FsOpKind::Mkdir.is_read_only());
        assert!(!FsOpKind::Remove.is_read_only());
    }

    #[cfg(unix)]
    #[test]
    fn read_via_symlink_escaping_root_is_rejected() {
        use std::os::unix::fs::symlink;
        let outside = tempdir().unwrap();
        std::fs::write(outside.path().join("treasure.txt"), b"loot").unwrap();
        let inside = tempdir().unwrap();
        let sb = Sandbox::open(inside.path()).unwrap();
        symlink(
            outside.path().join("treasure.txt"),
            inside.path().join("link.txt"),
        )
        .unwrap();
        let err = read_file(&sb, "link.txt").unwrap_err();
        assert!(matches!(err, FsExecutorError::SymlinkEscape(_)));
    }
}
