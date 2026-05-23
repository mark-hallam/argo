//! Cross-tenant isolation tests for [`crate::cred`].
//!
//! Brief: `mission-cred-get-tenant-parameter` (P0 from #2080 §4).
//!
//! These tests pin the guarantee that one tenant's worker cannot read
//! another tenant's per-tenant credential entries through the public
//! `cred::get(tenant_id, name)` API.

use super::*;
use argo_types::TenantId;

#[test]
fn tenant_get_returns_own_value() {
    let dir = tempfile::tempdir().unwrap();
    let tenant_a = TenantId::new();
    let key = "SLACK_BOT_TOKEN";

    set_tenant(dir.path(), &tenant_a, key, "xoxb-a").unwrap();

    let value = get_from_workspace(dir.path(), &tenant_a, key).unwrap();
    assert_eq!(value, "xoxb-a");
}

#[test]
fn tenant_get_does_not_leak_across_tenants() {
    let dir = tempfile::tempdir().unwrap();
    let tenant_a = TenantId::new();
    let tenant_b = TenantId::new();
    let key = "SLACK_BOT_TOKEN";
    // Cross-tenant: keep env clean so file-backed lookup is exercised.
    env::remove_var(key);

    set_tenant(dir.path(), &tenant_a, key, "xoxb-a").unwrap();
    set_tenant(dir.path(), &tenant_b, key, "xoxb-b").unwrap();

    // tenant_b sees tenant_b's value, NOT tenant_a's.
    let value_b = get_from_workspace(dir.path(), &tenant_b, key).unwrap();
    assert_eq!(value_b, "xoxb-b", "tenant_b read returned tenant_a's value");

    // tenant_a sees tenant_a's value, NOT tenant_b's.
    let value_a = get_from_workspace(dir.path(), &tenant_a, key).unwrap();
    assert_eq!(value_a, "xoxb-a", "tenant_a read returned tenant_b's value");
}

#[test]
fn tenant_get_returns_not_found_for_other_tenants_key() {
    let dir = tempfile::tempdir().unwrap();
    let tenant_a = TenantId::new();
    let tenant_b = TenantId::new();
    let key = "TENANT_A_ONLY_TOKEN";
    env::remove_var(key);

    // Only tenant_a has the key set.
    set_tenant(dir.path(), &tenant_a, key, "secret-a").unwrap();

    // tenant_b asking for the key must NOT receive tenant_a's value.
    let err = get_from_workspace(dir.path(), &tenant_b, key).unwrap_err();
    assert!(
        matches!(err, std::env::VarError::NotPresent),
        "tenant_b read of tenant_a's exclusive key must error NotPresent, got {err:?}"
    );
}

#[test]
fn legacy_root_value_visible_to_any_tenant_with_warning() {
    // Migration ramp: a legacy root-level key is visible to any tenant
    // (with a tracing::warn!). The cross-tenant isolation P0 only
    // applies to *per-tenant sections*; pre-migration callsites still
    // need their keys to resolve.
    let dir = tempfile::tempdir().unwrap();
    let tenant_a = TenantId::new();
    let key = "ARGO_TEST_LEGACY_FALLBACK_TOKEN";
    env::remove_var(key);

    // Legacy root-level write (the pre-mission shape).
    set(dir.path(), key, "legacy-value").unwrap();

    // Tenant-scoped read falls through to the legacy root section.
    let value = get_from_workspace(dir.path(), &tenant_a, key).unwrap();
    assert_eq!(value, "legacy-value");
}

#[test]
fn tenant_section_overrides_legacy_root() {
    // When both shapes have the key, the tenant section wins (no warn).
    let dir = tempfile::tempdir().unwrap();
    let tenant_a = TenantId::new();
    let key = "ARGO_TEST_OVERRIDE_TOKEN";
    env::remove_var(key);

    set(dir.path(), key, "legacy-shared").unwrap();
    set_tenant(dir.path(), &tenant_a, key, "tenant-a-private").unwrap();

    let value = get_from_workspace(dir.path(), &tenant_a, key).unwrap();
    assert_eq!(value, "tenant-a-private");
}

#[test]
fn load_tenant_returns_only_tenants_section() {
    let dir = tempfile::tempdir().unwrap();
    let tenant_a = TenantId::new();
    let tenant_b = TenantId::new();

    set_tenant(dir.path(), &tenant_a, "TOKEN_A", "a-value").unwrap();
    set_tenant(dir.path(), &tenant_b, "TOKEN_B", "b-value").unwrap();
    set(dir.path(), "LEGACY_TOKEN", "legacy-value").unwrap();

    let loaded_a = load_tenant(dir.path(), &tenant_a).unwrap();
    assert_eq!(loaded_a.len(), 1);
    assert_eq!(loaded_a.get("TOKEN_A").map(String::as_str), Some("a-value"));
    assert!(!loaded_a.contains_key("TOKEN_B"));
    assert!(!loaded_a.contains_key("LEGACY_TOKEN"));
}

#[test]
fn clear_all_wipes_legacy_and_tenant_sections() {
    let dir = tempfile::tempdir().unwrap();
    let tenant_a = TenantId::new();
    let tenant_b = TenantId::new();

    set_tenant(dir.path(), &tenant_a, "TOKEN_A", "a-value").unwrap();
    set_tenant(dir.path(), &tenant_b, "TOKEN_B", "b-value").unwrap();
    set(dir.path(), "LEGACY_TOKEN", "legacy-value").unwrap();

    let removed = clear_all(dir.path()).unwrap();
    assert_eq!(removed, 3);
    assert!(load(dir.path()).unwrap().is_empty());
    assert!(load_tenant(dir.path(), &tenant_a).unwrap().is_empty());
    assert!(load_tenant(dir.path(), &tenant_b).unwrap().is_empty());
}

#[test]
fn env_var_overrides_tenant_section() {
    // Process env always wins, even with tenant scoping.
    let dir = tempfile::tempdir().unwrap();
    let tenant_a = TenantId::new();
    let key = "ARGO_TEST_TENANT_ENV_OVERRIDE_TOKEN";

    set_tenant(dir.path(), &tenant_a, key, "from-file").unwrap();
    env::set_var(key, "from-env");

    let value = get_from_workspace(dir.path(), &tenant_a, key).unwrap();
    assert_eq!(value, "from-env");
    env::remove_var(key);
}
