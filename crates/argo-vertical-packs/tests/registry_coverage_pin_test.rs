//! Structural pin tests for `eval-vertical-pack-registry-coverage`
//! (2026-05-23). LIVE tests freeze today's behavior; `#[ignore]`'d
//! FINDINGS-ONLY tests document the drift we want to close.
//!
//! Findings report:
//! `docs/skycot/evals/scheduled/2026-05-23/eval-vertical-pack-registry-coverage.md`.

use argo_vertical_packs::{PackLoader, PackLoaderConfig, PackManifest};
use std::path::{Path, PathBuf};

/// Workspace root (two parents up from this crate's manifest dir),
/// used to walk `verticals/` and `packs/` on disk.
fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("crates/<name>/.. should resolve to workspace root")
        .to_path_buf()
}

/// Subdirs of `verticals/` that intentionally do not load through
/// `argo_vertical::load_dir` (template scaffolds, not shipping
/// verticals). Used by the FINDINGS-ONLY test below.
const VERTICALS_NON_SHIPPING_ALLOWLIST: &[&str] = &["_starter-library"];

/// LIVE: a fresh `PackLoader` cache root (tempdir, never populated)
/// reports zero installed packs. Pins the documented "fresh `~/.argo/`
/// has no marketplace bootstrap" behavior — if a future change starts
/// auto-seeding default packs into the cache, this test fires and the
/// audit report needs an update.
#[test]
fn pack_loader_fresh_cache_returns_no_installed_packs() {
    let cache = tempfile::tempdir().expect("tempdir for empty cache");
    let loader = PackLoader::new(PackLoaderConfig::with_cache_dir(cache.path().into()));
    let rows = loader.list().expect("list empty cache");
    assert!(
        rows.is_empty(),
        "fresh PackLoader cache should be empty; found {} rows: {:?}",
        rows.len(),
        rows.iter().map(|r| &r.name).collect::<Vec<_>>()
    );
}

/// LIVE: every `packs/<name>/pack.toml` checked into the repo parses
/// cleanly through `PackManifest::load`. Acts as a structural inventory
/// for the in-tree pack distribution prototype: if a future pack
/// regresses the schema, this catches it before CI clippy does.
#[test]
fn repo_packs_all_pack_toml_round_trip_through_pack_manifest() {
    let packs_dir = workspace_root().join("packs");
    if !packs_dir.is_dir() {
        eprintln!(
            "skipping: no packs/ directory at {} (not running from workspace root?)",
            packs_dir.display()
        );
        return;
    }
    let mut bad: Vec<String> = Vec::new();
    for entry in std::fs::read_dir(&packs_dir).expect("read packs/") {
        let entry = entry.expect("packs/ entry");
        if !entry.file_type().expect("file_type").is_dir() {
            continue;
        }
        let pack_root = entry.path();
        if !pack_root.join("pack.toml").is_file() {
            continue;
        }
        if let Err(e) = PackManifest::load(&pack_root) {
            bad.push(format!("{}: {e}", pack_root.display()));
        }
    }
    assert!(
        bad.is_empty(),
        "{} pack.toml file(s) failed to parse:\n  {}",
        bad.len(),
        bad.join("\n  ")
    );
}

/// FINDINGS-ONLY (`#[ignore]`'d): every pack id declared under
/// `packs/<name>/pack.toml` `[[verticals]]` has a corresponding loadable
/// `verticals/<id>/vertical.toml` on disk.
///
/// **Fails today** for `ap-specialist`, `deal-desk`, `support-tier1`,
/// `vendor-manager` (demo-only stubs without vertical.toml) and
/// `project-coordinator` (no vertical directory at all). Flip to live
/// once `mission-pack-format-adapter-to-vertical-loader` (Finding F-2)
/// lands and the two parallel template formats converge.
#[test]
#[ignore = "FINDINGS-ONLY: documents Finding F-2 (pack format ≠ vertical format) and F-5 (project-coordinator gap) — see eval-vertical-pack-registry-coverage.md"]
fn repo_pack_ids_have_loadable_vertical_counterpart() {
    let packs_dir = workspace_root().join("packs");
    let verticals_dir = workspace_root().join("verticals");
    let mut missing: Vec<String> = Vec::new();
    for entry in std::fs::read_dir(&packs_dir).expect("read packs/") {
        let entry = entry.expect("packs/ entry");
        if !entry.file_type().expect("file_type").is_dir() {
            continue;
        }
        let pack_root = entry.path();
        let manifest = match PackManifest::load(&pack_root) {
            Ok(m) => m,
            Err(_) => continue,
        };
        for v in &manifest.verticals {
            let candidate = verticals_dir.join(&v.id).join("vertical.toml");
            if !candidate.is_file() {
                missing.push(format!(
                    "pack {} declares [[verticals]] id={} but {} does not exist",
                    manifest.pack.name,
                    v.id,
                    candidate.display()
                ));
            }
        }
    }
    assert!(
        missing.is_empty(),
        "{} pack vertical id(s) without a loadable counterpart:\n  {}",
        missing.len(),
        missing.join("\n  ")
    );
}

/// FINDINGS-ONLY (`#[ignore]`'d): every directory under `verticals/`
/// either ships a `vertical.toml` or is named in
/// `VERTICALS_NON_SHIPPING_ALLOWLIST`. Catches the 4 demo-only stub
/// directories (`ap-specialist`, `deal-desk`, `support-tier1`,
/// `vendor-manager`) that currently sit next to real verticals but
/// silently fail to load.
///
/// Flip to live once `verticals-demo-stubs-promote-or-archive`
/// (Finding F-3) lands.
#[test]
#[ignore = "FINDINGS-ONLY: documents Finding F-3 (demo stubs in verticals/) — see eval-vertical-pack-registry-coverage.md"]
fn verticals_subdirs_all_have_manifest_or_demo_marker() {
    let verticals_dir = workspace_root().join("verticals");
    let mut undeclared_stubs: Vec<String> = Vec::new();
    for entry in std::fs::read_dir(&verticals_dir).expect("read verticals/") {
        let entry = entry.expect("verticals/ entry");
        if !entry.file_type().expect("file_type").is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        if VERTICALS_NON_SHIPPING_ALLOWLIST.contains(&name.as_str()) {
            continue;
        }
        if !entry.path().join("vertical.toml").is_file() {
            undeclared_stubs.push(name);
        }
    }
    assert!(
        undeclared_stubs.is_empty(),
        "{} verticals/ subdir(s) without vertical.toml or allowlist entry:\n  {}",
        undeclared_stubs.len(),
        undeclared_stubs.join("\n  ")
    );
}
