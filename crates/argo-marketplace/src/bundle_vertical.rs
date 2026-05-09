//! Vertical-bundle assembly + unpack — sibling of `bundle.rs`.
//!
//! Split out of `bundle.rs` (claude-marketplace-bundle-split) to
//! keep that file under the 500-LOC HARD RULE 6 ceiling. Pure
//! refactor; no behaviour change. Public API is unchanged —
//! `build_vertical_bundle` and `unpack_vertical_bundle` are still
//! re-exported from the crate root.
//!
//! Verticals (Stream `claude-marketplace-vertical-publish`) pack
//! every file under the vertical root, unlike the workspace
//! bundle which walks a fixed `BUNDLE_PATHS` allowlist.
use crate::bundle::{build_tar_bytes, sha256_hex, walk_dir, BundleEntry, BundleManifest};
use argo_types::{ArgoError, ArgoResult};
use std::path::Path;

/// Build a vertical bundle (Stream `claude-marketplace-vertical-publish`).
///
/// Unlike [`crate::build_bundle`] which walks the canonical workspace
/// `BUNDLE_PATHS` set, this packs EVERY file under the
/// `vertical_root` (recursively). Verticals can ship arbitrary
/// `prompts/`, `memory/`, `eval/`, `skills/`, `worker.toml`,
/// `vertical.toml` files — `argo vertical publish` should mirror
/// that on-disk layout exactly.
///
/// Round-trippable + deterministic (entries sorted by path,
/// `Header::set_mtime(0)`, etc.) so the digest stays stable
/// across runs of the same vertical state.
pub async fn build_vertical_bundle(
    vertical_root: &Path,
    vertical_id: &str,
    tag: &str,
    out_tar: &Path,
) -> ArgoResult<BundleManifest> {
    if !vertical_root.exists() {
        return Err(ArgoError::Provider(format!(
            "vertical_root '{}' does not exist",
            vertical_root.display()
        )));
    }
    let mut entries: Vec<BundleEntry> = Vec::new();
    walk_dir(vertical_root, vertical_root, &mut entries).await?;
    entries.sort_by(|a, b| a.path.cmp(&b.path));
    let tar_bytes = build_tar_bytes(vertical_root, &entries).await?;
    tokio::fs::write(out_tar, &tar_bytes)
        .await
        .map_err(|e| ArgoError::Provider(format!("write tar {}: {e}", out_tar.display())))?;
    let digest = sha256_hex(&tar_bytes);
    Ok(BundleManifest {
        digest,
        workspace_name: vertical_id.to_string(),
        tag: tag.to_string(),
        entries,
    })
}

/// Unpack a previously-published vertical bundle into
/// `target_root`. Verifies the tar's sha256 against the
/// supplied manifest digest BEFORE writing any file — refuses
/// to extract a tampered bundle.
///
/// `target_root` is the directory that will become the new
/// vertical's root (e.g. `verticals/<id>/`). Caller is
/// responsible for refusing to overwrite an existing vertical;
/// this function happily writes whatever's in the tar.
pub async fn unpack_vertical_bundle(
    tar_bytes: &[u8],
    expected: &BundleManifest,
    target_root: &Path,
) -> ArgoResult<()> {
    let actual = sha256_hex(tar_bytes);
    if actual != expected.digest {
        return Err(ArgoError::Provider(format!(
            "vertical bundle digest mismatch: manifest declares {} but tar hashes to {} \
             — refusing to extract tampered bundle",
            expected.digest, actual
        )));
    }
    tokio::fs::create_dir_all(target_root).await.map_err(|e| {
        ArgoError::Provider(format!(
            "create target_root {}: {e}",
            target_root.display()
        ))
    })?;
    let mut archive = tar::Archive::new(std::io::Cursor::new(tar_bytes));
    for entry in archive.entries().map_err(|e| {
        ArgoError::Provider(format!("read tar entries: {e}"))
    })? {
        let mut entry =
            entry.map_err(|e| ArgoError::Provider(format!("tar entry: {e}")))?;
        let rel = entry
            .path()
            .map_err(|e| ArgoError::Provider(format!("tar entry path: {e}")))?
            .into_owned();
        let abs = target_root.join(&rel);
        if let Some(parent) = abs.parent() {
            tokio::fs::create_dir_all(parent).await.map_err(|e| {
                ArgoError::Provider(format!(
                    "create parent {}: {e}",
                    parent.display()
                ))
            })?;
        }
        let mut body = Vec::new();
        std::io::Read::read_to_end(&mut entry, &mut body)
            .map_err(|e| ArgoError::Provider(format!("read tar body: {e}")))?;
        tokio::fs::write(&abs, &body)
            .await
            .map_err(|e| ArgoError::Provider(format!("write {}: {e}", abs.display())))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    async fn write(p: &Path, body: &str) {
        if let Some(d) = p.parent() {
            tokio::fs::create_dir_all(d).await.unwrap();
        }
        tokio::fs::write(p, body).await.unwrap();
    }

    async fn populate_vertical(root: &Path) {
        write(
            &root.join("vertical.toml"),
            "[meta]\nid=\"selling-skates\"\nversion=\"0.1\"\n",
        )
        .await;
        write(&root.join("worker.toml"), "[worker]\nname=\"seller\"\n").await;
        write(&root.join("prompts/system.md"), "you are a seller").await;
        write(&root.join("prompts/safety.md"), "do not share PII").await;
        write(&root.join("memory/seed.md"), "initial price points").await;
        write(&root.join("skills/post.md"), "post a listing").await;
        write(&root.join("eval/golden_path.json"), "{\"steps\":[]}").await;
    }

    #[tokio::test]
    async fn vertical_bundle_round_trip_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let vroot: PathBuf = dir.path().join("v");
        populate_vertical(&vroot).await;
        let tar = dir.path().join("v.tar");
        let manifest = build_vertical_bundle(&vroot, "selling-skates", "0.1", &tar)
            .await
            .unwrap();
        let target = dir.path().join("restored");
        let bytes = tokio::fs::read(&tar).await.unwrap();
        unpack_vertical_bundle(&bytes, &manifest, &target)
            .await
            .unwrap();
        for entry in &manifest.entries {
            let restored = target.join(&entry.path);
            assert!(restored.exists(), "missing {}", entry.path);
        }
        assert!(target.join("vertical.toml").exists());
        assert!(target.join("prompts/system.md").exists());
        assert!(target.join("eval/golden_path.json").exists());
    }

    #[tokio::test]
    async fn vertical_bundle_packs_arbitrary_paths() {
        // Unlike the workspace bundle, vertical bundle walks every
        // file under the root — not the BUNDLE_PATHS allowlist.
        let dir = tempfile::tempdir().unwrap();
        let vroot = dir.path().join("v");
        populate_vertical(&vroot).await;
        write(&vroot.join("custom_dir/extra.toml"), "x=1").await;
        let tar = dir.path().join("v.tar");
        let manifest = build_vertical_bundle(&vroot, "v", "0.1", &tar).await.unwrap();
        assert!(manifest
            .entries
            .iter()
            .any(|e| e.path == "custom_dir/extra.toml"));
    }

    #[tokio::test]
    async fn vertical_bundle_rejects_tampered_tar() {
        let dir = tempfile::tempdir().unwrap();
        let vroot = dir.path().join("v");
        populate_vertical(&vroot).await;
        let tar = dir.path().join("v.tar");
        let manifest = build_vertical_bundle(&vroot, "v", "0.1", &tar).await.unwrap();
        let bytes = tokio::fs::read(&tar).await.unwrap();
        let mut tampered = bytes.clone();
        // Flip a byte mid-tar — not header (would invalid format) —
        // somewhere in a body region.
        let mid = tampered.len() / 2;
        tampered[mid] ^= 0xFF;
        let target = dir.path().join("restored");
        let err = unpack_vertical_bundle(&tampered, &manifest, &target)
            .await
            .unwrap_err();
        let msg = format!("{err}");
        assert!(
            msg.contains("digest mismatch"),
            "expected digest mismatch err, got: {msg}"
        );
        assert!(
            !target.join("vertical.toml").exists(),
            "must not write any file when digest fails"
        );
    }

    #[tokio::test]
    async fn vertical_bundle_is_deterministic() {
        let dir = tempfile::tempdir().unwrap();
        let vroot = dir.path().join("v");
        populate_vertical(&vroot).await;
        let a = dir.path().join("a.tar");
        let b = dir.path().join("b.tar");
        let m1 = build_vertical_bundle(&vroot, "v", "0.1", &a).await.unwrap();
        let m2 = build_vertical_bundle(&vroot, "v", "0.1", &b).await.unwrap();
        assert_eq!(m1.digest, m2.digest);
    }

    #[tokio::test]
    async fn vertical_bundle_missing_root_errs() {
        let dir = tempfile::tempdir().unwrap();
        let vroot = dir.path().join("does-not-exist");
        let tar = dir.path().join("v.tar");
        let err = build_vertical_bundle(&vroot, "v", "0.1", &tar)
            .await
            .unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("does not exist"), "{msg}");
    }
}
