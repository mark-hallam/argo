//! §23.3 — workspace bundle round-trip through an OCI-shaped
//! registry. Uses `MemoryRegistry` instead of a live OCI host so
//! the test runs hermetically; the same trait + bytes will plug
//! into a real registry once `oci-distribution` lands.
//!
//! Flow:
//!   1. Materialize a fixture workspace under a temp dir.
//!   2. `build_bundle` to a tar + manifest.
//!   3. Push to `MemoryRegistry`.
//!   4. Pull back; assert byte-for-byte equality on the tar and
//!      structural equality on the manifest (digest, workspace
//!      name, entries).
//!   5. Build a second bundle from the same workspace and verify
//!      `manifests_match` is true (deterministic across runs).
use argo_marketplace::{
    build_bundle, manifests_match, MemoryRegistry, OciReference, OciRegistry,
};
use std::fs;
use std::path::Path;

fn write_fixture(root: &Path) {
    fs::create_dir_all(root).unwrap();
    fs::write(
        root.join("argo.toml"),
        "# argo.toml\n[core]\nname = \"fixture\"\n",
    )
    .unwrap();
    for f in [
        "MEMORY.md",
        "USER.md",
        "SOUL.md",
        "AGENTS.md",
    ] {
        fs::write(root.join(f), format!("# {f}\nbody for fixture\n")).unwrap();
    }
    for d in ["skills", "personas", "cron", "rubrics"] {
        fs::create_dir_all(root.join(d)).unwrap();
    }
    fs::write(
        root.join("skills").join("hello.md"),
        "---\nname: hello\nversion: 0.1\n---\nhello world\n",
    )
    .unwrap();
}

#[tokio::test]
async fn workspace_bundle_round_trips_through_memory_registry() {
    let dir = tempfile::tempdir().unwrap();
    let ws = dir.path().join("fixture-workspace");
    write_fixture(&ws);

    let tar_path = dir.path().join("bundle.tar");
    let manifest = build_bundle(&ws, "fixture-workspace", "0.1", &tar_path)
        .await
        .expect("build_bundle");
    assert!(!manifest.entries.is_empty(), "fixture has files");
    let tar_bytes = fs::read(&tar_path).expect("read tar");

    let reg = MemoryRegistry::new();
    let reference = OciReference::parse("registry.local/argo/fixture-workspace:0.1").unwrap();
    reg.push(&reference, &tar_bytes, &manifest).await.unwrap();
    assert_eq!(reg.len().await, 1);

    let (pulled_tar, pulled_manifest) = reg.pull(&reference).await.unwrap();
    assert_eq!(pulled_tar, tar_bytes, "tar bytes round-trip");
    assert_eq!(pulled_manifest.digest, manifest.digest);
    assert_eq!(pulled_manifest.workspace_name, manifest.workspace_name);
    assert_eq!(pulled_manifest.tag, manifest.tag);
    assert_eq!(pulled_manifest.entries.len(), manifest.entries.len());
}

#[tokio::test]
async fn pulling_unknown_ref_returns_provider_error() {
    let reg = MemoryRegistry::new();
    let reference = OciReference::parse("registry.local/argo/nope:1").unwrap();
    let err = reg.pull(&reference).await.unwrap_err();
    assert!(format!("{err}").contains("no such reference"));
}

#[tokio::test]
async fn second_bundle_matches_first_for_same_workspace() {
    let dir = tempfile::tempdir().unwrap();
    let ws = dir.path().join("ws");
    write_fixture(&ws);

    let tar1 = dir.path().join("a.tar");
    let tar2 = dir.path().join("b.tar");
    let m1 = build_bundle(&ws, "ws", "1.0", &tar1).await.unwrap();
    let m2 = build_bundle(&ws, "ws", "1.0", &tar2).await.unwrap();
    assert!(
        manifests_match(&m1, &m2),
        "deterministic build_bundle should round-trip identically across runs"
    );
}
