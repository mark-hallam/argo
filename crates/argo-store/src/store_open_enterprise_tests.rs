//! Tests for the production-mode at-rest-encryption gate added by
//! `mission-audit-sqlcipher-enterprise-default`.
//!
//! The gate is a pure two-predicate check on
//! `(is_production_mode(), security.encryption.key_source)` — it does
//! not open a database, so these tests stay deterministic on every
//! runner regardless of the linked SQLite driver. The three documented
//! scenarios:
//!
//! 1. dev mode + `key_source = "none"` → opens (back-compat default)
//! 2. production mode + `key_source = "none"` → returns `Err`
//! 3. production mode + `key_source = "env:..."` → opens *iff* the
//!    binary was built with the `sqlcipher` feature
//!
//! Scenario (3) is split into two cases below: one runs only under
//! the `sqlcipher` feature (and asserts `Ok`), the other only without
//! it (and asserts `Err`, because production mode without SQLCipher is
//! never honored). Together they cover every operator-facing build of
//! the gate.

use super::store_open::enforce_enterprise_encryption;
use argo_types::ArgoError;

#[test]
fn dev_mode_with_none_keysource_opens() {
    // Scenario 1: default local-dev config. No regression on the
    // ~666 existing test callsites that pass `key_source = "none"`.
    assert!(enforce_enterprise_encryption(false, "none").is_ok());
    assert!(enforce_enterprise_encryption(false, "").is_ok());
    assert!(enforce_enterprise_encryption(false, "  NONE  ").is_ok());
    // Dev mode is permissive even when a real key is configured —
    // operators sometimes mirror prod config locally for repros.
    assert!(enforce_enterprise_encryption(false, "env:ARGO_DB_KEY").is_ok());
}

#[test]
fn production_mode_with_none_keysource_fails_closed() {
    // Scenario 2: the silent-plaintext trap the brief was written to
    // close. Production mode + key_source=none must always Err,
    // regardless of compile-time feature flag.
    let err = enforce_enterprise_encryption(true, "none").unwrap_err();
    assert!(matches!(err, ArgoError::Schema(_)));
    let msg = format!("{err}");
    // Error message names BOTH possible causes so the operator sees
    // the full diagnosis without consulting the runbook.
    assert!(
        msg.contains("--features bundled-sqlcipher"),
        "missing rebuild hint in: {msg}"
    );
    assert!(
        msg.contains("security.key_source"),
        "missing key_source hint in: {msg}"
    );
    assert!(
        msg.contains("feature_enabled="),
        "missing feature_enabled diagnostic in: {msg}"
    );
    assert!(
        msg.contains("key_source=none"),
        "missing offending key_source value in: {msg}"
    );
    // Empty / whitespace / case-shifted "none" all count as unset.
    assert!(enforce_enterprise_encryption(true, "").is_err());
    assert!(enforce_enterprise_encryption(true, "   ").is_err());
    assert!(enforce_enterprise_encryption(true, "NONE").is_err());
    assert!(enforce_enterprise_encryption(true, " None ").is_err());
}

/// Scenario 3a — `sqlcipher` feature on. Production mode with a real
/// key source must boot cleanly; this asserts the gate's positive
/// path. Per the brief: "production mode + key_source=env → opens
/// (assuming env var is set)". The gate itself does not resolve the
/// env var — it only validates that `key_source` is not `none` —
/// which keeps the test deterministic on every runner.
#[cfg(feature = "sqlcipher")]
#[test]
fn production_mode_with_env_keysource_opens_under_sqlcipher() {
    assert!(enforce_enterprise_encryption(true, "env:ARGO_DB_KEY").is_ok());
    assert!(enforce_enterprise_encryption(true, "file:/etc/argo/db.key").is_ok());
    assert!(enforce_enterprise_encryption(true, "aws-kms:arn:abc?wrapped=env:DEK").is_ok());
}

/// Scenario 3b — `sqlcipher` feature OFF (stock build). Even with a
/// real `key_source`, production mode must Err — the binary is not
/// linked against a SQLCipher driver so `PRAGMA key` would be silently
/// ignored. This is the fail-closed branch the gate exists to defend.
#[cfg(not(feature = "sqlcipher"))]
#[test]
fn production_mode_without_sqlcipher_feature_fails_closed_even_with_key() {
    let err = enforce_enterprise_encryption(true, "env:ARGO_DB_KEY").unwrap_err();
    assert!(matches!(err, ArgoError::Schema(_)));
    let msg = format!("{err}");
    assert!(msg.contains("feature_enabled=false"), "got: {msg}");
}
