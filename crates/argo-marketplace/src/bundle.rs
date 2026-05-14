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
//!
//! Vertical-bundle assembly (`build_vertical_bundle`,
//! `unpack_vertical_bundle`) lives in the sibling
//! `bundle_vertical` module — both share the helpers below
//! (`walk_dir`, `build_tar_bytes`, `sha256_hex`).
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
///
/// Round-trips through serde so `argo workspace push` / `pull` can
/// persist or transmit it as JSON next to the tarball:
///
/// ```
/// use argo_marketplace::{BundleEntry, BundleManifest};
/// let m = BundleManifest {
///     digest: "deadbeef".into(),
///     workspace_name: "demo".into(),
///     tag: "0.1.0".into(),
///     entries: vec![BundleEntry { path: "argo.toml".into(), size: 12, digest: "abc".into() }],
/// };
/// let json = serde_json::to_string(&m).unwrap();
/// let back: BundleManifest = serde_json::from_str(&json).unwrap();
/// assert_eq!(back.entries[0].path, "argo.toml");
/// ```
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
        .map_err(|e| ArgoError::Io(format!("write tar {}: {e}", out_tar.display())))?;
    let digest = sha256_hex(&tar_bytes);
    Ok(BundleManifest {
        digest,
        workspace_name: workspace_name.to_string(),
        tag: tag.to_string(),
        entries,
    })
}

/// Two manifests' digests match → byte-identical bundle (§23.3).
///
/// ```
/// use argo_marketplace::{manifests_match, BundleManifest};
/// let m1 = BundleManifest { digest: "abc".into(), workspace_name: "x".into(), tag: "1".into(), entries: vec![] };
/// let mut m2 = m1.clone();
/// assert!(manifests_match(&m1, &m2));
/// m2.digest = "different".into();
/// assert!(!manifests_match(&m1, &m2));
/// ```
#[must_use]
pub fn manifests_match(a: &BundleManifest, b: &BundleManifest) -> bool {
    a.digest == b.digest
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
        .map_err(|e| ArgoError::Io(format!("read {}: {e}", p.display())))?;
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

pub(crate) fn walk_dir<'a>(
    root: &'a Path,
    dir: &'a Path,
    out: &'a mut Vec<BundleEntry>,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = ArgoResult<()>> + Send + 'a>> {
    Box::pin(async move {
        let mut entries = tokio::fs::read_dir(dir)
            .await
            .map_err(|e| ArgoError::Io(format!("read_dir {}: {e}", dir.display())))?;
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

pub(crate) async fn build_tar_bytes(
    workspace_root: &Path,
    entries: &[BundleEntry],
) -> ArgoResult<Vec<u8>> {
    let mut buf: Vec<u8> = Vec::new();
    {
        let mut builder = tar::Builder::new(&mut buf);
        builder.mode(tar::HeaderMode::Deterministic);
        for e in entries {
            let abs = workspace_root.join(&e.path);
            let body = tokio::fs::read(&abs)
                .await
                .map_err(|err| ArgoError::Io(format!("read {}: {err}", abs.display())))?;
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

pub(crate) fn sha256_hex(bytes: &[u8]) -> String {
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
}
