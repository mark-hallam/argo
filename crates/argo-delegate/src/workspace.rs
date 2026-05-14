//! COW workspace fork — copies the parent workspace into a child
//! sandbox dir, then merges changes back when the child completes.
//!
//! Phase 5 implementation: eager copy on fork (simple, predictable).
//! Larger workspaces would benefit from lazy COW (only copy on first
//! write); that's a §16.2 follow-up.
use argo_types::{ArgoError, ArgoResult};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// One forked workspace under a temp dir.
#[derive(Debug)]
pub struct WorkspaceFork {
    pub(crate) parent_root: PathBuf,
    pub(crate) child_root: PathBuf,
    /// SHA-256 of every file that existed in the parent at fork time.
    /// Used to detect whether the parent changed during the
    /// delegation; if so, merging back is unsafe.
    pub(crate) parent_baseline: HashMap<PathBuf, [u8; 32]>,
}

/// Result of merging a child workspace back into the parent (§16.4
/// "two siblings editing different files merge cleanly; same file
/// produces a conflict").
#[derive(Debug, Clone, Default)]
pub struct MergeReport {
    /// Files the child added or modified that we wrote into the parent.
    pub applied: Vec<PathBuf>,
    /// Files where the parent ALSO changed during the delegation —
    /// we don't apply; the caller decides what to do.
    pub conflicts: Vec<PathBuf>,
    /// Files the child deleted; we delete in the parent too.
    pub deleted: Vec<PathBuf>,
}

impl WorkspaceFork {
    /// Eagerly copy `parent_root` into a sibling temp dir. The child
    /// agent operates on the returned `child_root`.
    pub async fn fork(parent_root: &Path) -> ArgoResult<Self> {
        let child_root = make_sibling_dir(parent_root)?;
        let parent_baseline = copy_dir_recursive_with_hashes(parent_root, &child_root).await?;
        Ok(Self {
            parent_root: parent_root.to_path_buf(),
            child_root,
            parent_baseline,
        })
    }

    /// Path the child agent should treat as its workspace root.
    #[must_use]
    pub fn child_root(&self) -> &Path {
        &self.child_root
    }

    /// Merge the child's changes back into the parent.
    ///
    /// For each file that exists in the child:
    /// - if absent from `parent_baseline` → added by child → apply.
    /// - if `parent_baseline` matches the parent today AND child
    ///   changed → apply.
    /// - if `parent_baseline` no longer matches the parent → both
    ///   changed → conflict.
    /// - otherwise → unchanged → no-op.
    ///
    /// For each file in `parent_baseline` missing from the child →
    /// child deleted it → delete from parent (if parent still has the
    /// baseline content; otherwise conflict).
    pub async fn merge_back(&self) -> ArgoResult<MergeReport> {
        let mut report = MergeReport::default();
        let child_files = list_files(&self.child_root).await?;
        for rel in &child_files {
            let parent_path = self.parent_root.join(rel);
            let child_path = self.child_root.join(rel);
            let child_hash = hash_file(&child_path).await?;
            let baseline = self.parent_baseline.get(rel).copied();
            match baseline {
                None => {
                    // Child added a new file. If parent already has
                    // it, that's a conflict (parent + child both
                    // created during delegation).
                    if parent_path.exists() {
                        report.conflicts.push(rel.clone());
                    } else {
                        copy_file(&child_path, &parent_path).await?;
                        report.applied.push(rel.clone());
                    }
                }
                Some(baseline_hash) => {
                    if child_hash == baseline_hash {
                        // Child didn't touch the file; nothing to do.
                        continue;
                    }
                    let parent_hash = hash_file(&parent_path).await?;
                    if parent_hash == baseline_hash {
                        // Parent unchanged → safe to apply child.
                        copy_file(&child_path, &parent_path).await?;
                        report.applied.push(rel.clone());
                    } else {
                        // Both changed since fork → conflict.
                        report.conflicts.push(rel.clone());
                    }
                }
            }
        }
        let baseline_set: std::collections::HashSet<&PathBuf> =
            self.parent_baseline.keys().collect();
        let child_set: std::collections::HashSet<&PathBuf> = child_files.iter().collect();
        for rel in baseline_set.difference(&child_set) {
            let parent_path = self.parent_root.join(rel);
            let baseline_hash = self.parent_baseline.get(*rel).copied().unwrap_or([0u8; 32]);
            if !parent_path.exists() {
                continue;
            }
            let parent_hash = hash_file(&parent_path).await?;
            if parent_hash == baseline_hash {
                tokio::fs::remove_file(&parent_path).await.map_err(|e| {
                    ArgoError::Provider(format!(
                        "delete during merge {}: {e}",
                        parent_path.display()
                    ))
                })?;
                report.deleted.push((*rel).clone());
            } else {
                report.conflicts.push((*rel).clone());
            }
        }
        Ok(report)
    }

    /// Drop the child's temp dir.
    pub async fn cleanup(self) -> ArgoResult<()> {
        if self.child_root.exists() {
            tokio::fs::remove_dir_all(&self.child_root)
                .await
                .map_err(|e| {
                    ArgoError::Provider(format!("cleanup {}: {e}", self.child_root.display()))
                })?;
        }
        Ok(())
    }
}

fn make_sibling_dir(parent_root: &Path) -> ArgoResult<PathBuf> {
    let suffix = format!(
        "argo-delegate-{}",
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
    );
    let parent_dir = parent_root
        .parent()
        .ok_or_else(|| ArgoError::Schema("workspace has no parent dir".into()))?;
    let p = parent_dir.join(suffix);
    std::fs::create_dir_all(&p).map_err(ArgoError::from)?;
    Ok(p)
}

async fn copy_dir_recursive_with_hashes(
    src: &Path,
    dst: &Path,
) -> ArgoResult<HashMap<PathBuf, [u8; 32]>> {
    let mut hashes = HashMap::new();
    let files = list_files(src).await?;
    for rel in files {
        let src_path = src.join(&rel);
        let dst_path = dst.join(&rel);
        if let Some(parent) = dst_path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| ArgoError::Io(format!("mkdir {}: {e}", parent.display())))?;
        }
        copy_file(&src_path, &dst_path).await?;
        hashes.insert(rel, hash_file(&src_path).await?);
    }
    Ok(hashes)
}

async fn copy_file(src: &Path, dst: &Path) -> ArgoResult<()> {
    if let Some(parent) = dst.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| ArgoError::Io(format!("mkdir {}: {e}", parent.display())))?;
    }
    tokio::fs::copy(src, dst).await.map_err(|e| {
        ArgoError::Io(format!("copy {} → {}: {e}", src.display(), dst.display()))
    })?;
    Ok(())
}

async fn hash_file(p: &Path) -> ArgoResult<[u8; 32]> {
    let bytes = tokio::fs::read(p)
        .await
        .map_err(|e| ArgoError::Io(format!("read {}: {e}", p.display())))?;
    let mut h = Sha256::new();
    h.update(&bytes);
    let out = h.finalize();
    let mut buf = [0u8; 32];
    buf.copy_from_slice(&out);
    Ok(buf)
}

async fn list_files(root: &Path) -> ArgoResult<Vec<PathBuf>> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let mut entries = tokio::fs::read_dir(&dir)
            .await
            .map_err(|e| ArgoError::Io(format!("read_dir {}: {e}", dir.display())))?;
        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|e| ArgoError::Provider(format!("next_entry: {e}")))?
        {
            let path = entry.path();
            let ft = entry
                .file_type()
                .await
                .map_err(|e| ArgoError::Provider(format!("file_type: {e}")))?;
            if ft.is_dir() {
                stack.push(path);
            } else if ft.is_file() {
                let rel = path
                    .strip_prefix(root)
                    .map_err(|e| ArgoError::Provider(format!("strip_prefix: {e}")))?
                    .to_path_buf();
                out.push(rel);
            }
        }
    }
    out.sort();
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn write(p: &Path, body: &str) {
        if let Some(d) = p.parent() {
            tokio::fs::create_dir_all(d).await.unwrap();
        }
        tokio::fs::write(p, body).await.unwrap();
    }

    #[tokio::test]
    async fn fork_eagerly_copies_files() {
        let dir = tempfile::tempdir().unwrap();
        let parent = dir.path().join("parent");
        write(&parent.join("a.txt"), "v1").await;
        write(&parent.join("nested/b.txt"), "v1").await;
        let fork = WorkspaceFork::fork(&parent).await.unwrap();
        assert_eq!(
            tokio::fs::read_to_string(fork.child_root().join("a.txt"))
                .await
                .unwrap(),
            "v1"
        );
        assert_eq!(
            tokio::fs::read_to_string(fork.child_root().join("nested/b.txt"))
                .await
                .unwrap(),
            "v1"
        );
        fork.cleanup().await.unwrap();
    }

    #[tokio::test]
    async fn merge_applies_child_only_changes() {
        let dir = tempfile::tempdir().unwrap();
        let parent = dir.path().join("parent");
        write(&parent.join("a.txt"), "v1").await;
        let fork = WorkspaceFork::fork(&parent).await.unwrap();
        write(&fork.child_root().join("a.txt"), "v2").await;
        write(&fork.child_root().join("new.txt"), "child").await;
        let report = fork.merge_back().await.unwrap();
        assert!(report.conflicts.is_empty());
        assert!(report.applied.contains(&PathBuf::from("a.txt")));
        assert!(report.applied.contains(&PathBuf::from("new.txt")));
        assert_eq!(
            tokio::fs::read_to_string(parent.join("a.txt"))
                .await
                .unwrap(),
            "v2"
        );
        assert_eq!(
            tokio::fs::read_to_string(parent.join("new.txt"))
                .await
                .unwrap(),
            "child"
        );
    }

    #[tokio::test]
    async fn merge_flags_conflict_when_both_changed() {
        let dir = tempfile::tempdir().unwrap();
        let parent = dir.path().join("parent");
        write(&parent.join("a.txt"), "v1").await;
        let fork = WorkspaceFork::fork(&parent).await.unwrap();
        // Both sides change the same file with different content.
        write(&fork.child_root().join("a.txt"), "child-edit").await;
        write(&parent.join("a.txt"), "parent-edit").await;
        let report = fork.merge_back().await.unwrap();
        assert!(report.conflicts.contains(&PathBuf::from("a.txt")));
        // Parent's edit was preserved; child's was not applied.
        assert_eq!(
            tokio::fs::read_to_string(parent.join("a.txt"))
                .await
                .unwrap(),
            "parent-edit"
        );
    }

    #[tokio::test]
    async fn merge_handles_child_deletion() {
        let dir = tempfile::tempdir().unwrap();
        let parent = dir.path().join("parent");
        write(&parent.join("a.txt"), "v1").await;
        write(&parent.join("b.txt"), "stays").await;
        let fork = WorkspaceFork::fork(&parent).await.unwrap();
        tokio::fs::remove_file(fork.child_root().join("a.txt"))
            .await
            .unwrap();
        let report = fork.merge_back().await.unwrap();
        assert!(report.deleted.contains(&PathBuf::from("a.txt")));
        assert!(!parent.join("a.txt").exists());
        assert!(parent.join("b.txt").exists());
    }

    #[tokio::test]
    async fn merge_no_op_when_child_unchanged() {
        let dir = tempfile::tempdir().unwrap();
        let parent = dir.path().join("parent");
        write(&parent.join("a.txt"), "v1").await;
        let fork = WorkspaceFork::fork(&parent).await.unwrap();
        let report = fork.merge_back().await.unwrap();
        assert!(report.applied.is_empty());
        assert!(report.conflicts.is_empty());
        assert!(report.deleted.is_empty());
    }

    #[tokio::test]
    async fn sibling_subagents_editing_different_files_merge_cleanly() {
        // §16.4 acceptance: simulate two sibling forks each editing
        // disjoint files; merging both back should produce zero
        // conflicts.
        let dir = tempfile::tempdir().unwrap();
        let parent = dir.path().join("parent");
        write(&parent.join("a.txt"), "v1").await;
        write(&parent.join("b.txt"), "v1").await;
        let fork_a = WorkspaceFork::fork(&parent).await.unwrap();
        let fork_b = WorkspaceFork::fork(&parent).await.unwrap();
        write(&fork_a.child_root().join("a.txt"), "edit-a").await;
        write(&fork_b.child_root().join("b.txt"), "edit-b").await;
        let r1 = fork_a.merge_back().await.unwrap();
        let r2 = fork_b.merge_back().await.unwrap();
        assert!(r1.conflicts.is_empty());
        assert!(r2.conflicts.is_empty());
        assert_eq!(
            tokio::fs::read_to_string(parent.join("a.txt"))
                .await
                .unwrap(),
            "edit-a"
        );
        assert_eq!(
            tokio::fs::read_to_string(parent.join("b.txt"))
                .await
                .unwrap(),
            "edit-b"
        );
    }

    #[tokio::test]
    async fn sibling_subagents_editing_same_file_produce_conflict() {
        // §16.4 acceptance: two siblings editing the same file → the
        // second merge sees the first's apply as a conflicting edit
        // against its baseline.
        let dir = tempfile::tempdir().unwrap();
        let parent = dir.path().join("parent");
        write(&parent.join("a.txt"), "v1").await;
        let fork_a = WorkspaceFork::fork(&parent).await.unwrap();
        let fork_b = WorkspaceFork::fork(&parent).await.unwrap();
        write(&fork_a.child_root().join("a.txt"), "edit-a").await;
        write(&fork_b.child_root().join("a.txt"), "edit-b").await;
        let r1 = fork_a.merge_back().await.unwrap();
        assert!(r1.conflicts.is_empty()); // first merge is clean
        let r2 = fork_b.merge_back().await.unwrap();
        assert!(r2.conflicts.contains(&PathBuf::from("a.txt")));
    }
}
