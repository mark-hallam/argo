//! Tests for [`super`] — the per-tenant master-key derivation and
//! workspace-root resolution paths added by
//! `mission-credentials-per-tenant-master-key`.
//!
//! Tests that mutate `SKYCOT_WORKSPACE_ROOT_KEY_BASE64` or
//! `ARGO_CREDENTIALS_MASTER_KEY` serialize through [`env_mutex`] so
//! cargo's parallel runner can't observe one test's env state from
//! another. Each test clears the per-tenant cache on entry so the
//! HKDF derivation actually runs (otherwise a previous test's
//! `Arc<MasterKey>` would shortcut the resolver).

use super::*;
use crate::age_wrap::{decrypt_value, decrypt_value_with_fallback, encrypt_value, KeyVersion};
use argo_types::TenantId;
use base64::engine::general_purpose::STANDARD as B64;
use std::sync::{Mutex, OnceLock};

fn env_mutex() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn fixed_root() -> WorkspaceRootKey {
    let mut bytes = [0u8; WORKSPACE_ROOT_KEY_LEN];
    for (i, b) in bytes.iter_mut().enumerate() {
        *b = (i as u8).wrapping_mul(31).wrapping_add(7);
    }
    WorkspaceRootKey::from_bytes(bytes)
}

fn set_env_root(bytes: &[u8; WORKSPACE_ROOT_KEY_LEN]) -> Option<String> {
    let prior = env::var(WORKSPACE_ROOT_KEY_ENV).ok();
    env::set_var(WORKSPACE_ROOT_KEY_ENV, B64.encode(bytes));
    prior
}

fn restore_env(key: &str, prior: Option<String>) {
    match prior {
        Some(value) => env::set_var(key, value),
        None => env::remove_var(key),
    }
}

#[test]
fn from_bytes_round_trips_through_age_secret() {
    let mut bytes = [0u8; 32];
    for (i, b) in bytes.iter_mut().enumerate() {
        *b = i as u8 ^ 0x5A;
    }
    let key = MasterKey::from_bytes(bytes).expect("from_bytes");
    let serialized = key.as_age_secret();
    assert!(
        serialized.starts_with("AGE-SECRET-KEY-1"),
        "must serialize as canonical age secret: {serialized}"
    );
    // Round-trip: parsing the serialized form yields a key that decrypts
    // ciphertext produced by the original. The cleanest evidence that
    // both `Identity` instances wrap the same StaticSecret.
    let parsed = MasterKey::from_age_secret(&serialized).expect("from_age_secret");
    let blob = encrypt_value("payload", &key).expect("encrypt");
    let decrypted = decrypt_value(&blob, &parsed).expect("decrypt via parsed");
    assert_eq!(decrypted, "payload");
}

#[test]
fn derive_for_tenant_is_deterministic() {
    let root = fixed_root();
    let tenant = TenantId::new();
    let a = MasterKey::derive_for_tenant(&root, tenant).expect("derive a");
    let b = MasterKey::derive_for_tenant(&root, tenant).expect("derive b");
    assert_eq!(
        a.as_age_secret(),
        b.as_age_secret(),
        "same root + same tenant must derive the same key"
    );
}

/// 🔴 P0 ACCEPTANCE — eval-credentials-cross-tenant-isolation §3 P0
/// (round-9, PR #2080). The fix-brief's load-bearing invariant: two
/// distinct tenants MUST NOT share a master key.
#[test]
fn derive_for_two_tenants_yields_distinct_keys() {
    let root = fixed_root();
    let tenant_a = TenantId::new();
    let tenant_b = TenantId::new();
    assert_ne!(tenant_a, tenant_b);
    let key_a = MasterKey::derive_for_tenant(&root, tenant_a).expect("derive a");
    let key_b = MasterKey::derive_for_tenant(&root, tenant_b).expect("derive b");
    assert_ne!(
        key_a.as_age_secret(),
        key_b.as_age_secret(),
        "two tenants MUST derive distinct master keys (cross-tenant isolation)"
    );
}

/// 🔴 P0 ACCEPTANCE — decrypt-with-tenant_b-key on tenant_a's payload
/// MUST fail. Not "silent-empty," not "returns wrong value": a real
/// AEAD authentication failure surfaced as [`ArgoError::Provider`].
#[test]
fn decrypt_cross_tenant_fails_loudly() {
    let root = fixed_root();
    let tenant_a = TenantId::new();
    let tenant_b = TenantId::new();
    let key_a = MasterKey::derive_for_tenant(&root, tenant_a).expect("derive a");
    let key_b = MasterKey::derive_for_tenant(&root, tenant_b).expect("derive b");

    let plaintext = "sk-ant-tenant-a-private";
    let blob = encrypt_value(plaintext, &key_a).expect("encrypt under tenant_a key");

    // Decrypting with the wrong tenant's key must fail — and the error
    // message must mention age/decrypt so the operator can diagnose.
    let err = decrypt_value(&blob, &key_b).expect_err("cross-tenant decrypt must fail");
    let msg = format!("{err}");
    assert!(
        msg.contains("age") || msg.contains("decrypt"),
        "error must surface as AEAD failure: {msg}"
    );

    // Sanity: the matching key still works.
    let decrypted = decrypt_value(&blob, &key_a).expect("tenant_a's own key decrypts");
    assert_eq!(decrypted, plaintext);
}

#[test]
fn workspace_root_key_resolves_from_env() {
    let _guard = env_mutex().lock().unwrap_or_else(|e| e.into_inner());
    _clear_tenant_key_cache_for_tests();
    let bytes = [0xAB; WORKSPACE_ROOT_KEY_LEN];
    let prior = set_env_root(&bytes);
    let root = WorkspaceRootKey::resolve().expect("resolve from env");
    assert_eq!(root.as_bytes(), &bytes);
    restore_env(WORKSPACE_ROOT_KEY_ENV, prior);
}

#[test]
fn workspace_root_key_rejects_wrong_length() {
    let _guard = env_mutex().lock().unwrap_or_else(|e| e.into_inner());
    _clear_tenant_key_cache_for_tests();
    let prior = env::var(WORKSPACE_ROOT_KEY_ENV).ok();
    let prior_legacy = env::var(MASTER_KEY_ENV).ok();
    // Set a base64-valid but wrong-length value AND clear the legacy
    // fallback so `resolve()` actually hits the env branch.
    env::set_var(WORKSPACE_ROOT_KEY_ENV, B64.encode([0u8; 16]));
    env::remove_var(MASTER_KEY_ENV);
    let err = WorkspaceRootKey::resolve().expect_err("16-byte root must fail");
    let msg = format!("{err}");
    assert!(msg.contains("wrong length"), "msg: {msg}");
    assert!(msg.contains(WORKSPACE_ROOT_KEY_ENV), "msg: {msg}");
    restore_env(WORKSPACE_ROOT_KEY_ENV, prior);
    restore_env(MASTER_KEY_ENV, prior_legacy);
}

#[test]
fn workspace_root_falls_back_to_legacy_master_key() {
    let _guard = env_mutex().lock().unwrap_or_else(|e| e.into_inner());
    _clear_tenant_key_cache_for_tests();
    let prior_root = env::var(WORKSPACE_ROOT_KEY_ENV).ok();
    let prior_legacy = env::var(MASTER_KEY_ENV).ok();
    env::remove_var(WORKSPACE_ROOT_KEY_ENV);
    let legacy = MasterKey::generate();
    env::set_var(MASTER_KEY_ENV, legacy.as_age_secret());
    let root = WorkspaceRootKey::resolve().expect("fall back to legacy");
    let expected = WorkspaceRootKey::from_legacy_master_key(&legacy);
    assert_eq!(
        root.as_bytes(),
        expected.as_bytes(),
        "back-compat derivation must be deterministic"
    );
    restore_env(WORKSPACE_ROOT_KEY_ENV, prior_root);
    restore_env(MASTER_KEY_ENV, prior_legacy);
}

#[test]
fn workspace_root_errors_when_no_source_configured() {
    let _guard = env_mutex().lock().unwrap_or_else(|e| e.into_inner());
    _clear_tenant_key_cache_for_tests();
    let prior_root = env::var(WORKSPACE_ROOT_KEY_ENV).ok();
    let prior_legacy = env::var(MASTER_KEY_ENV).ok();
    let prior_home = env::var("HOME").ok();
    let tmp_home = tempfile::tempdir().expect("tempdir");
    env::remove_var(WORKSPACE_ROOT_KEY_ENV);
    env::remove_var(MASTER_KEY_ENV);
    env::set_var("HOME", tmp_home.path());
    let err = WorkspaceRootKey::resolve().expect_err("no source → error");
    let msg = format!("{err}");
    assert!(msg.contains(WORKSPACE_ROOT_KEY_ENV), "msg names env: {msg}");
    assert!(msg.contains(MASTER_KEY_ENV), "msg names legacy env: {msg}");
    restore_env(WORKSPACE_ROOT_KEY_ENV, prior_root);
    restore_env(MASTER_KEY_ENV, prior_legacy);
    restore_env("HOME", prior_home);
}

#[test]
fn resolve_for_tenant_caches_keys() {
    let _guard = env_mutex().lock().unwrap_or_else(|e| e.into_inner());
    _clear_tenant_key_cache_for_tests();
    let bytes = [0xCD; WORKSPACE_ROOT_KEY_LEN];
    let prior = set_env_root(&bytes);
    let tenant = TenantId::new();
    let first = MasterKey::resolve_for_tenant(tenant).expect("first resolve");
    let second = MasterKey::resolve_for_tenant(tenant).expect("second resolve");
    assert!(
        Arc::ptr_eq(&first, &second),
        "second resolve must hit the cache (same Arc)"
    );
    restore_env(WORKSPACE_ROOT_KEY_ENV, prior);
}

#[test]
fn resolve_for_tenant_uses_env_root() {
    let _guard = env_mutex().lock().unwrap_or_else(|e| e.into_inner());
    _clear_tenant_key_cache_for_tests();
    let bytes = [0x42; WORKSPACE_ROOT_KEY_LEN];
    let prior = set_env_root(&bytes);
    let tenant = TenantId::new();
    let key = MasterKey::resolve_for_tenant(tenant).expect("resolve");
    let expected =
        MasterKey::derive_for_tenant(&WorkspaceRootKey::from_bytes(bytes), tenant).expect("derive");
    assert_eq!(
        key.as_age_secret(),
        expected.as_age_secret(),
        "env-derived key must equal direct derivation"
    );
    restore_env(WORKSPACE_ROOT_KEY_ENV, prior);
}

/// Lazy migration: a credential wrapped under the legacy global key
/// must still decrypt via [`decrypt_value_with_fallback`], and the
/// returned [`KeyVersion`] tells the caller to re-encrypt under the
/// per-tenant key.
#[test]
fn decrypt_with_fallback_returns_legacy_marker() {
    let legacy = MasterKey::generate();
    let per_tenant =
        MasterKey::derive_for_tenant(&fixed_root(), TenantId::new()).expect("derive");
    let blob = encrypt_value("legacy-payload", &legacy).expect("wrap under legacy");
    let result = decrypt_value_with_fallback(&blob, &per_tenant, Some(&legacy))
        .expect("fallback must succeed");
    assert_eq!(result.plaintext, "legacy-payload");
    assert_eq!(result.key_version, KeyVersion::Legacy);
}

#[test]
fn decrypt_with_fallback_returns_per_tenant_marker_on_primary_hit() {
    let primary =
        MasterKey::derive_for_tenant(&fixed_root(), TenantId::new()).expect("derive primary");
    let legacy = MasterKey::generate();
    let blob = encrypt_value("current-payload", &primary).expect("wrap under primary");
    let result = decrypt_value_with_fallback(&blob, &primary, Some(&legacy))
        .expect("primary must succeed");
    assert_eq!(result.plaintext, "current-payload");
    assert_eq!(result.key_version, KeyVersion::PerTenant);
}

#[test]
fn decrypt_with_fallback_errors_when_no_legacy_and_primary_wrong() {
    let primary =
        MasterKey::derive_for_tenant(&fixed_root(), TenantId::new()).expect("derive primary");
    let other = MasterKey::generate();
    let blob = encrypt_value("orphan", &other).expect("wrap under other");
    let err =
        decrypt_value_with_fallback(&blob, &primary, None).expect_err("no fallback → error");
    let msg = format!("{err}");
    assert!(msg.contains("age") || msg.contains("decrypt"), "msg: {msg}");
}

#[test]
fn debug_redacts_workspace_root_key() {
    let root = fixed_root();
    let rendered = format!("{root:?}");
    assert!(
        rendered.contains("redacted"),
        "Debug must redact: {rendered}"
    );
    assert!(
        !rendered.contains("[7,"),
        "Debug leaked raw bytes: {rendered}"
    );
}
