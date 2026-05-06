//! Workspace bundle — deterministic tarball assembly per §23.1.
//!
//! A workspace bundle includes:
//! - `argo.toml`
//! - the four memory files (`MEMORY.md`, `USER.md`, `SOUL.md`, `AGENTS.md`)
//! - `skills/**`
//! - `cron/**`
//! - `secrets.example.toml`
//! - `personas/**`
//!
//! Determinism: files are added to the tar in **lexicographic
//! path order**, with mtime / uid / gid set to 0, so two pushes of
//! the same input produce a byte-identical tarball + digest.
use argo_types::{ArgoError, ArgoResult};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

/// Files / directories that count as part of a workspace bundle, in
/// the canonical order they appear in the tar.
pub const BUNDLE_PATHS: &[&str] = &[
    "argo.toml",
    "secrets.example.toml",
    "MEMORY.md",
    "USER.md",
    "SOUL.md",
    "AGENTS.md",
    "personas",
    "skills",
    "cron",
];

/// Manifest emitted alongside the tar (digest + entries).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BundleManifest {
    /// SHA-256 of the tar in hex.
    pub digest: String,
    /// `argo.toml` path → resolved name (e.g. `"my-workspace"`).
    pub workspace_name: String,
    /// Tag (e.g. `"0.3.0"` or `"latest"`).
    pub tag: String,
    /// Files included in the tar, in tar order.
    pub entries: Vec<BundleEntry>,
}

/// One file in the bundle.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BundleEntry {
    /// Workspace-relative path.
    pub path: String,
    /// Size in bytes.
    pub size: u64,
    /// SHA-256 of the file body in hex.
    pub digest: String,
}

/// Assemble a deterministic tar of `workspace_root` into `out_tar`.
///
/// Returns the manifest (with the tar's hex digest + per-entry
/// digests). Round-trippable: pushing → pulling → re-bundling
/// yields the same digest for the same workspace state.
pub async fn build_bundle(
    workspace_root: &Path,
    workspace_name: &str,
    tag: &str,
    out_tar: &Path,
) -> ArgoResult<BundleManifest> {
    let entries = collect_entries(workspace_root).await?;
    let tar_bytes = build_tar_bytes(workspace_root, &entries).await?;
    tokio::fs::write(out_tar, &tar_bytes)
        .await
        .map_err(|e| ArgoError::Provider(format!("write tar {}: {e}", out_tar.display())))?;
    let digest = sha256_hex(&tar_bytes);
    Ok(BundleManifest {
        digest,
        workspace_name: workspace_name.to_string(),
        tag: tag.to_string(),
        entries,
    })
}

/// Two manifests' digests match → byte-identical bundle (§23.3).
#[must_use]
pub fn manifests_match(a: &BundleManifest, b: &BundleManifest) -> bool {
    a.digest == b.digest
}

/// Build a vertical bundle (Stream `claude-marketplace-vertical-publish`).
///
/// Unlike [`build_bundle`] which walks the canonical workspace
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
    let mut entries = Vec::new();
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

/// Walk `workspace_root` for the canonical bundle paths and produce
/// per-entry digests + sizes in tar order.
async fn collect_entries(workspace_root: &Path) -> ArgoResult<Vec<BundleEntry>> {
    let mut all = Vec::new();
    for top in BUNDLE_PATHS {
        let p = workspace_root.join(top);
        if !p.exists() {
            continue;
        }
        if p.is_file() {
            all.push(read_entry(workspace_root, &p).await?);
        } else if p.is_dir() {
            let mut nested = Vec::new();
            walk_dir(workspace_root, &p, &mut nested).await?;
            nested.sort_by(|a, b| a.path.cmp(&b.path));
            all.extend(nested);
        }
    }
    Ok(all)
}

async fn read_entry(workspace_root: &Path, p: &Path) -> ArgoResult<BundleEntry> {
    let bytes = tokio::fs::read(p)
        .await
        .map_err(|e| ArgoError::Provider(format!("read {}: {e}", p.display())))?;
    let rel = p
        .strip_prefix(workspace_root)
        .map_err(|e| ArgoError::Provider(format!("strip_prefix: {e}")))?
        .to_string_lossy()
        .replace('\\', "/");
    Ok(BundleEntry {
        path: rel,
        size: bytes.len() as u64,
        digest: sha256_hex(&bytes),
    })
}

fn walk_dir<'a>(
    root: &'a Path,
    dir: &'a Path,
    out: &'a mut Vec<BundleEntry>,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = ArgoResult<()>> + Send + 'a>> {
    Box::pin(async move {
        let mut entries = tokio::fs::read_dir(dir)
            .await
            .map_err(|e| ArgoError::Provider(format!("read_dir {}: {e}", dir.display())))?;
        let mut child_dirs: Vec<PathBuf> = Vec::new();
        while let Some(e) = entries
            .next_entry()
            .await
            .map_err(|e| ArgoError::Provider(format!("next_entry: {e}")))?
        {
            let path = e.path();
            let ft = e
                .file_type()
                .await
                .map_err(|e| ArgoError::Provider(format!("file_type: {e}")))?;
            if ft.is_dir() {
                child_dirs.push(path);
            } else if ft.is_file() {
                out.push(read_entry(root, &path).await?);
            }
        }
        child_dirs.sort();
        for child in child_dirs {
            walk_dir(root, &child, out).await?;
        }
        Ok(())
    })
}

async fn build_tar_bytes(workspace_root: &Path, entries: &[BundleEntry]) -> ArgoResult<Vec<u8>> {
    let mut buf: Vec<u8> = Vec::new();
    {
        let mut builder = tar::Builder::new(&mut buf);
        builder.mode(tar::HeaderMode::Deterministic);
        for e in entries {
            let abs = workspace_root.join(&e.path);
            let body = tokio::fs::read(&abs)
                .await
                .map_err(|err| ArgoError::Provider(format!("read {}: {err}", abs.display())))?;
            let mut header = tar::Header::new_gnu();
            header
                .set_path(&e.path)
                .map_err(|err| ArgoError::Provider(format!("tar set_path {}: {err}", e.path)))?;
            header.set_size(body.len() as u64);
            header.set_mode(0o644);
            header.set_mtime(0);
            header.set_uid(0);
            header.set_gid(0);
            header.set_entry_type(tar::EntryType::Regular);
            header.set_cksum();
            builder
                .append(&header, body.as_slice())
                .map_err(|err| ArgoError::Provider(format!("tar append: {err}")))?;
        }
        builder
            .finish()
            .map_err(|err| ArgoError::Provider(format!("tar finish: {err}")))?;
    }
    Ok(buf)
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    hex::encode(h.finalize())
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

    fn fixture(dir: &Path) -> PathBuf {
        let ws = dir.join("ws");
        ws.to_path_buf()
    }

    async fn populate(ws: &Path) {
        write(&ws.join("argo.toml"), "[core]\nname=\"test\"\n").await;
        write(&ws.join("MEMORY.md"), "memory body").await;
        write(&ws.join("USER.md"), "user").await;
        write(&ws.join("SOUL.md"), "soul").await;
        write(&ws.join("AGENTS.md"), "agents").await;
        write(&ws.join("skills/alpha.md"), "skill alpha body").await;
        write(&ws.join("skills/beta.md"), "skill beta body").await;
        write(&ws.join("personas/main.md"), "persona main").await;
    }

    #[tokio::test]
    async fn bundle_round_trip_yields_stable_digest() {
        // §23.3: workspace round-trips through fork → modify → push →
        // pull on a fresh machine; result is byte-identical. We
        // simulate the byte-identical guarantee by re-bundling and
        // verifying the digest matches.
        let dir = tempfile::tempdir().unwrap();
        let ws = fixture(dir.path());
        populate(&ws).await;
        let tar1 = dir.path().join("a.tar");
        let tar2 = dir.path().join("b.tar");
        let m1 = build_bundle(&ws, "test-workspace", "0.1", &tar1)
            .await
            .unwrap();
        let m2 = build_bundle(&ws, "test-workspace", "0.1", &tar2)
            .await
            .unwrap();
        assert!(manifests_match(&m1, &m2));
        let bytes_a = tokio::fs::read(&tar1).await.unwrap();
        let bytes_b = tokio::fs::read(&tar2).await.unwrap();
        assert_eq!(bytes_a, bytes_b);
    }

    #[tokio::test]
    async fn bundle_picks_up_canonical_paths_in_order() {
        let dir = tempfile::tempdir().unwrap();
        let ws = fixture(dir.path());
        populate(&ws).await;
        let tar = dir.path().join("ws.tar");
        let m = build_bundle(&ws, "x", "0.1", &tar).await.unwrap();
        let names: Vec<&str> = m.entries.iter().map(|e| e.path.as_str()).collect();
        // argo.toml first, then MEMORY/USER/SOUL/AGENTS before
        // personas/skills.
        assert_eq!(names[0], "argo.toml");
        assert!(
            names.iter().position(|p| *p == "MEMORY.md").unwrap()
                < names.iter().position(|p| *p == "skills/alpha.md").unwrap()
        );
        // skills/* sorted within their group.
        let skills: Vec<&&str> = names.iter().filter(|p| p.starts_with("skills/")).collect();
        assert_eq!(skills, [&"skills/alpha.md", &"skills/beta.md"]);
    }

    #[tokio::test]
    async fn modifying_a_file_changes_the_digest() {
        let dir = tempfile::tempdir().unwrap();
        let ws = fixture(dir.path());
        populate(&ws).await;
        let tar1 = dir.path().join("a.tar");
        let m1 = build_bundle(&ws, "x", "0.1", &tar1).await.unwrap();
        write(&ws.join("MEMORY.md"), "edited body").await;
        let tar2 = dir.path().join("b.tar");
        let m2 = build_bundle(&ws, "x", "0.1", &tar2).await.unwrap();
        assert!(!manifests_match(&m1, &m2));
    }

    #[tokio::test]
    async fn bundle_skips_unknown_top_level_dirs() {
        let dir = tempfile::tempdir().unwrap();
        let ws = fixture(dir.path());
        populate(&ws).await;
        write(&ws.join("not_in_bundle/x.txt"), "ignored").await;
        let tar = dir.path().join("ws.tar");
        let m = build_bundle(&ws, "x", "0.1", &tar).await.unwrap();
        for e in &m.entries {
            assert!(!e.path.starts_with("not_in_bundle/"), "{}", e.path);
        }
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
        let vroot = dir.path().join("v");
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
