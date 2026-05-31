//! Tests for tenant-scoped Genesis vertical resolution (DEC-025 #9 /
//! Genesis P5 part (b)).

use super::*;

/// Write a minimal `vertical.toml` for `id` under `<root>/.genesis/<slug>/`.
fn write_genesis_vertical(root: &Path, tenant_slug: &str, id: &str) {
    let dir = root.join(".genesis").join(tenant_slug).join(id);
    std::fs::create_dir_all(&dir).expect("mkdir");
    std::fs::write(
        dir.join("vertical.toml"),
        "schema_version = \"1\"\n[vertical]\nid = \"x\"\nname = \"X\"\nversion = \"0.1.0\"\n",
    )
    .expect("write manifest");
}

#[test]
fn sanitize_tenant_lowercases_and_strips() {
    assert_eq!(sanitize_tenant("Op-ABC123"), "op-abc123");
    assert_eq!(sanitize_tenant("a b!c"), "a-b-c");
    assert_eq!(sanitize_tenant("---"), "anonymous");
    assert_eq!(sanitize_tenant(""), "anonymous");
    // Trims leading/trailing separators.
    assert_eq!(sanitize_tenant("-mid-"), "mid");
}

#[test]
fn resolves_a_vertical_under_the_callers_own_tenant() {
    let tmp = tempfile::tempdir().expect("tmp");
    let root = tmp.path();
    write_genesis_vertical(root, "op-alice", "genesis-intern");

    let dir = genesis_vertical_dir_for_tenant(root, "op-alice", "genesis-intern")
        .expect("own-tenant vertical must resolve");
    assert!(dir.join("vertical.toml").is_file());
    assert!(genesis_vertical_exists_for_tenant(root, "op-alice", "genesis-intern"));
}

#[test]
fn does_not_resolve_a_vertical_owned_by_a_different_tenant() {
    // THE BUG THIS CLOSES: alice generated `genesis-intern`; bob must NOT be
    // able to resolve it just by id. Fail-closed: bob sees not-found.
    let tmp = tempfile::tempdir().expect("tmp");
    let root = tmp.path();
    write_genesis_vertical(root, "op-alice", "genesis-intern");

    assert!(
        genesis_vertical_dir_for_tenant(root, "op-bob", "genesis-intern").is_none(),
        "another tenant's vertical must NOT resolve for the caller"
    );
    assert!(!genesis_vertical_exists_for_tenant(root, "op-bob", "genesis-intern"));
}

#[test]
fn cross_tenant_only_match_is_detected_for_audit() {
    let tmp = tempfile::tempdir().expect("tmp");
    let root = tmp.path();
    write_genesis_vertical(root, "op-alice", "genesis-intern");

    // alice's own vertical is NOT a cross-tenant match for alice.
    assert!(!genesis_vertical_exists_for_other_tenant(
        root,
        "op-alice",
        "genesis-intern"
    ));
    // But it IS a cross-tenant-only match when bob asks (exists for alice,
    // not for bob) — the security event the hire gate audits.
    assert!(genesis_vertical_exists_for_other_tenant(
        root,
        "op-bob",
        "genesis-intern"
    ));
    // A genuinely unknown id is NOT a cross-tenant match (it exists nowhere).
    assert!(!genesis_vertical_exists_for_other_tenant(
        root,
        "op-bob",
        "no-such-vertical"
    ));
}

#[test]
fn slug_collision_is_resolved_consistently() {
    // Two tenant ids that sanitize to the SAME slug (e.g. casing) share the
    // segment — by construction, since the writer + reader both sanitize.
    let tmp = tempfile::tempdir().expect("tmp");
    let root = tmp.path();
    write_genesis_vertical(root, "op-alice", "genesis-intern");
    // "OP-ALICE" sanitizes to "op-alice" → resolves to the same dir.
    assert!(genesis_vertical_exists_for_tenant(root, "OP-ALICE", "genesis-intern"));
}

#[test]
fn missing_genesis_root_is_not_found_not_a_panic() {
    let tmp = tempfile::tempdir().expect("tmp");
    // No `.genesis/` dir at all.
    assert!(genesis_vertical_dir_for_tenant(tmp.path(), "op-alice", "x").is_none());
    assert!(!genesis_vertical_exists_for_other_tenant(tmp.path(), "op-alice", "x"));
}
