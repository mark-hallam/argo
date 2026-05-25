//! Round-trip integration test for the pack → vertical adapter.
//!
//! This test asserts that one real pack from `packs/` translates
//! through `PackLoader::load_as_vertical` into a `LoadedVertical`
//! the runtime `argo_vertical` loader is willing to register —
//! proving the bridge layer (brief
//! `mission-pack-format-adapter-to-vertical-loader`) end-to-end.

use argo_vertical::Registry;
use argo_vertical_packs::{PackLoader, PackLoaderConfig};
use std::path::{Path, PathBuf};

/// Locate the workspace root by walking up until we find
/// `Cargo.lock` (cargo guarantees one at workspace root). Tests
/// run from `target/.../deps/`, so `CARGO_MANIFEST_DIR` is the
/// crate dir and we need one more level up.
fn workspace_root() -> PathBuf {
    let mut cur = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    loop {
        if cur.join("Cargo.lock").is_file() {
            return cur;
        }
        if !cur.pop() {
            panic!("workspace root with Cargo.lock not found");
        }
    }
}

#[test]
fn ap_specialist_pack_round_trips_through_vertical_loader() {
    let pack_root = workspace_root().join("packs").join("ap-specialist");
    assert!(
        pack_root.join("pack.toml").is_file(),
        "fixture pack at {} missing pack.toml — test assumes packs/ap-specialist is the canonical AP pack",
        pack_root.display()
    );

    // Dev-link instead of installing from an archive so the test
    // exercises real on-disk pack content and never touches the
    // network or `~/.argo/packs`.
    let cache = tempfile::tempdir().unwrap();
    let loader = PackLoader::new(PackLoaderConfig::with_cache_dir(cache.path().into()));
    loader.dev_link("ap-specialist", &pack_root).unwrap();

    let loaded = loader
        .load_as_vertical("ap-specialist", None)
        .unwrap()
        .expect("dev-linked pack must resolve");
    assert_eq!(loaded.len(), 1, "ap-specialist declares one vertical");

    let vertical = &loaded[0];
    assert_eq!(vertical.manifest.vertical.id, "ap-specialist");
    assert_eq!(vertical.manifest.vertical.version, "1.0.0");
    assert!(
        !vertical.manifest.vertical.name.is_empty(),
        "adapter must derive a non-empty display name"
    );
    assert_eq!(canonical(&vertical.root), canonical(&pack_root));

    // Plug the adapted vertical into a fresh registry — proves the
    // bridge layer produces something the rest of the runtime is
    // happy to ingest, not just a free-floating struct.
    let mut registry = Registry::new();
    registry.insert(vertical.clone()).unwrap();
    assert!(registry.get("ap-specialist").is_some());
}

#[test]
fn unresolved_pack_returns_none() {
    let cache = tempfile::tempdir().unwrap();
    let loader = PackLoader::new(PackLoaderConfig::with_cache_dir(cache.path().into()));
    assert!(loader
        .load_as_vertical("never-installed", Some("9.9.9"))
        .unwrap()
        .is_none());
}

fn canonical(p: &Path) -> PathBuf {
    p.canonicalize().unwrap_or_else(|_| p.to_path_buf())
}
