//! Tests for [`crate::encryption`] — KeySource parsing/resolution
//! and the apply/rekey PRAGMA helpers.
//!
//! These tests do **not** verify encrypted page contents — the stock
//! sqlx driver linked here ignores `PRAGMA key`. They cover the API
//! surface that the future SQLCipher-linked build will rely on.

use super::*;
use std::io::Write;
use tempfile::NamedTempFile;

const SAMPLE_KEY: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

#[test]
fn parse_none_variants() {
    assert!(matches!(KeySource::parse("none").unwrap(), KeySource::None));
    assert!(matches!(KeySource::parse("").unwrap(), KeySource::None));
    assert!(matches!(KeySource::parse("  NONE ").unwrap(), KeySource::None));
}

#[test]
fn parse_env_happy() {
    let ks = KeySource::parse("env:ARGO_TEST_KEY").unwrap();
    match ks {
        KeySource::Env(name) => assert_eq!(name, "ARGO_TEST_KEY"),
        other => panic!("expected Env, got {other:?}"),
    }
}

#[test]
fn parse_file_happy() {
    let ks = KeySource::parse("file:/etc/argo/db.key").unwrap();
    match ks {
        KeySource::File(p) => assert_eq!(p.to_string_lossy(), "/etc/argo/db.key"),
        other => panic!("expected File, got {other:?}"),
    }
}

#[test]
fn parse_rejects_unknown_prefix() {
    let err = KeySource::parse("vault:secret/argo").unwrap_err();
    assert!(format!("{err}").contains("invalid key_source"));
}

#[test]
fn parse_rejects_empty_prefix_value() {
    assert!(KeySource::parse("env:").is_err());
    assert!(KeySource::parse("file:").is_err());
}

#[test]
fn resolve_env_happy() {
    // SAFETY: test runs in-process; var name unique to this test to
    // avoid races with other env-mutating tests.
    let var = "ARGO_TEST_RESOLVE_HAPPY";
    // SAFETY: single-threaded mutation of process env in test.
    unsafe { std::env::set_var(var, SAMPLE_KEY) };
    let resolved = KeySource::Env(var.into()).resolve().unwrap();
    // SAFETY: cleanup, same justification.
    unsafe { std::env::remove_var(var) };
    assert_eq!(resolved.as_deref(), Some(SAMPLE_KEY));
}

#[test]
fn resolve_env_missing_var() {
    let err = KeySource::Env("ARGO_TEST_DOES_NOT_EXIST_XYZ".into())
        .resolve()
        .unwrap_err();
    assert!(format!("{err}").contains("encryption key env"));
}

#[test]
fn resolve_file_happy() {
    let mut f = NamedTempFile::new().unwrap();
    writeln!(f, "{SAMPLE_KEY}").unwrap();
    let resolved = KeySource::File(f.path().into()).resolve().unwrap();
    assert_eq!(resolved.as_deref(), Some(SAMPLE_KEY));
}

#[test]
fn resolve_file_missing() {
    let err = KeySource::File("/nonexistent/argo-test-key.bin".into())
        .resolve()
        .unwrap_err();
    assert!(format!("{err}").contains("encryption key file"));
}

#[test]
fn resolve_none_returns_none() {
    assert_eq!(KeySource::None.resolve().unwrap(), None);
}

#[test]
fn resolve_rejects_invalid_hex() {
    // File-based to avoid env::set_var race on multi-threaded tests.
    let mut f = NamedTempFile::new().unwrap();
    writeln!(f, "ghghghghghghghghghghghghghghghghghghghghghghghghghghghghghghghgh").unwrap();
    let err = KeySource::File(f.path().into()).resolve().unwrap_err();
    assert!(
        format!("{err}").contains("non-hex"),
        "error did not flag non-hex: {err}"
    );
}

#[test]
fn resolve_rejects_wrong_length() {
    let mut f = NamedTempFile::new().unwrap();
    writeln!(f, "deadbeef").unwrap();
    let err = KeySource::File(f.path().into()).resolve().unwrap_err();
    assert!(
        format!("{err}").contains("64 hex"),
        "error did not flag length: {err}"
    );
}

#[test]
fn debug_impl_is_redacted() {
    let ks = KeySource::Env("MY_SECRET_VAR_NAME".into());
    let s = format!("{ks:?}");
    assert!(!s.contains("MY_SECRET_VAR_NAME"), "Debug leaked env name: {s}");
    assert!(s.contains("redacted"));

    let ks = KeySource::File("/very/secret/path.key".into());
    let s = format!("{ks:?}");
    assert!(!s.contains("secret"), "Debug leaked file path: {s}");
    assert!(s.contains("redacted"));
}

#[tokio::test]
async fn apply_and_rekey_round_trip_on_stock_sqlite() {
    // Stock sqlx ignores PRAGMA key — this test verifies the
    // statement executes cleanly and that rekey is callable after
    // open. It does NOT verify on-disk encryption.
    let store = crate::SessionStore::open(":memory:").await.unwrap();
    apply_encryption_pragma(store.pool(), SAMPLE_KEY).await.unwrap();
    let new_key = "fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210";
    apply_rekey_pragma(store.pool(), new_key).await.unwrap();
}
