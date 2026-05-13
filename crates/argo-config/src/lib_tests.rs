//! Tests for [`workspace_tenant_id`] + [`LEGACY_AUDIT_TENANT_SENTINEL`]
//! (Stream `mission-enterprise-cross-tenant-audit`).

use crate::{workspace_tenant_id, LEGACY_AUDIT_TENANT_SENTINEL};
use std::path::Path;

fn write_argo_toml(dir: &Path, body: &str) {
    std::fs::write(dir.join("argo.toml"), body).expect("write argo.toml");
}

#[test]
fn explicit_tenant_id_overrides_hash_fallback() {
    let tmp = tempfile::tempdir().expect("tempdir");
    write_argo_toml(tmp.path(), "[tenant]\ndefault_id = \"acme-prod\"\n");
    let id = workspace_tenant_id(tmp.path());
    assert_eq!(id, "acme-prod");
}

#[test]
fn hash_fallback_is_stable_when_default_id_absent() {
    let tmp = tempfile::tempdir().expect("tempdir");
    // No argo.toml at all → pure path-hash fallback.
    let first = workspace_tenant_id(tmp.path());
    let second = workspace_tenant_id(tmp.path());
    assert_eq!(first, second, "same path must hash to the same tenant id");
    assert!(
        first.starts_with("ws-"),
        "expected ws-<hex> fallback, got {first}"
    );
    assert_eq!(first.len(), 3 + 16, "ws- + 16 hex chars");

    // A different path must hash to a different id.
    let other = tempfile::tempdir().expect("tempdir2");
    let other_id = workspace_tenant_id(other.path());
    assert_ne!(first, other_id);
}

#[test]
fn never_returns_legacy_sentinel() {
    // Even if an operator literally writes "__legacy__" into the TOML,
    // the helper falls through to the hash fallback so historical-only
    // rows can never collide with a live workspace's reads.
    let tmp = tempfile::tempdir().expect("tempdir");
    write_argo_toml(tmp.path(), "[tenant]\ndefault_id = \"__legacy__\"\n");
    let id = workspace_tenant_id(tmp.path());
    assert_ne!(id, LEGACY_AUDIT_TENANT_SENTINEL);
    assert!(id.starts_with("ws-"));
}

#[test]
fn shipped_default_id_falls_through_to_hash() {
    // `default_id = "default"` is the shipped placeholder and must not
    // be interpreted as an explicit tenant.
    let tmp = tempfile::tempdir().expect("tempdir");
    write_argo_toml(tmp.path(), "[tenant]\ndefault_id = \"default\"\n");
    let id = workspace_tenant_id(tmp.path());
    assert!(id.starts_with("ws-"));
}
