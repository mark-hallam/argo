//! Tests for [`crate`].

use super::*;
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::sync::{Mutex, OnceLock};

fn make_key() -> MasterKey {
    MasterKey::generate()
}

/// Serializes the two tests that mutate `MASTER_KEY_ENV` so cargo's
/// default parallel test runner can't observe one test's env state
/// from the other.
fn env_mutex() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn write_plain_legacy(path: &Path, entries: &[(&str, &str)]) {
    let parent = path.parent().expect("parent dir");
    fs::create_dir_all(parent).expect("create parent");
    let mut body = String::from("[credentials]\n");
    for (k, v) in entries {
        body.push_str(&format!("{k} = \"{v}\"\n"));
    }
    fs::write(path, body).expect("write legacy file");
}

#[test]
fn round_trip_preserves_map() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("credentials.toml");
    let key = make_key();
    let mut credentials = CredentialMap::new();
    credentials.insert("SLACK_BOT_TOKEN".into(), "xoxb-secret".into());
    credentials.insert("STRIPE_API_KEY".into(), "sk_test_42".into());

    write_encrypted(&path, &credentials, &key).unwrap();
    let round_tripped = read_encrypted(&path, &key).unwrap();
    assert_eq!(round_tripped, credentials);
}

#[test]
fn mixed_plaintext_and_encrypted_supported() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("credentials.toml");
    let key = make_key();
    write_plain_legacy(
        &path,
        &[
            ("LEGACY_TOKEN", "plain-value"),
            ("OTHER_TOKEN", "another-plain"),
        ],
    );
    let loaded = read_encrypted(&path, &key).unwrap();
    assert_eq!(loaded.get("LEGACY_TOKEN").map(String::as_str), Some("plain-value"));
    assert_eq!(loaded.get("OTHER_TOKEN").map(String::as_str), Some("another-plain"));
}

#[test]
fn missing_master_key_resolves_to_error() {
    let _guard = env_mutex().lock().unwrap_or_else(|e| e.into_inner());
    let dir = tempfile::tempdir().unwrap();
    let prior = std::env::var(MASTER_KEY_ENV).ok();
    std::env::remove_var(MASTER_KEY_ENV);
    let err = MasterKey::resolve(Some(dir.path())).expect_err("no key configured");
    let msg = format!("{err}");
    assert!(msg.contains(MASTER_KEY_ENV), "msg should name env: {msg}");
    if let Some(v) = prior {
        std::env::set_var(MASTER_KEY_ENV, v);
    }
}

#[test]
fn wrong_master_key_fails_decrypt() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("credentials.toml");
    let key = make_key();
    let mut credentials = CredentialMap::new();
    credentials.insert("X".into(), "secret".into());
    write_encrypted(&path, &credentials, &key).unwrap();
    let wrong = make_key();
    let err = read_encrypted(&path, &wrong).expect_err("wrong key should fail");
    let msg = format!("{err}");
    assert!(msg.contains("decrypt") || msg.contains("age"), "msg: {msg}");
}

#[test]
fn migrate_plaintext_in_place_wraps_legacy_entries() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("credentials.toml");
    let key = make_key();
    write_plain_legacy(&path, &[("LEGACY_TOKEN", "plain-value")]);
    let report = migrate_plaintext_in_place(&path, &key).unwrap();
    assert_eq!(report.encrypted, 1);
    assert_eq!(report.already_encrypted, 0);

    // After migration the on-disk body must NOT contain the plaintext.
    let body = fs::read_to_string(&path).unwrap();
    assert!(!body.contains("plain-value"), "plaintext leaked: {body}");

    // And it must decrypt back to the original value.
    let loaded = read_encrypted(&path, &key).unwrap();
    assert_eq!(loaded.get("LEGACY_TOKEN").map(String::as_str), Some("plain-value"));
}

#[test]
fn migrate_skips_already_encrypted_entries() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("credentials.toml");
    let key = make_key();
    let mut credentials = CredentialMap::new();
    credentials.insert("ALREADY".into(), "encrypted-value".into());
    write_encrypted(&path, &credentials, &key).unwrap();

    let report = migrate_plaintext_in_place(&path, &key).unwrap();
    assert_eq!(report.encrypted, 0);
    assert_eq!(report.already_encrypted, 1);
}

#[test]
fn master_key_debug_is_redacted() {
    let key = MasterKey::from_age_secret(&MasterKey::generate().as_age_secret()).unwrap();
    let rendered = format!("{key:?}");
    assert!(rendered.contains("redacted"), "Debug must redact: {rendered}");
    assert!(!rendered.contains("AGE-SECRET-KEY"), "Debug leaked secret: {rendered}");
}

#[test]
fn resolve_reads_from_env_first() {
    let _guard = env_mutex().lock().unwrap_or_else(|e| e.into_inner());
    let dir = tempfile::tempdir().unwrap();
    let key = MasterKey::generate();
    let secret = key.as_age_secret();
    let prior = std::env::var(MASTER_KEY_ENV).ok();
    std::env::set_var(MASTER_KEY_ENV, &secret);
    let resolved = MasterKey::resolve(Some(dir.path())).unwrap();
    assert_eq!(resolved.as_age_secret(), secret);
    match prior {
        Some(v) => std::env::set_var(MASTER_KEY_ENV, v),
        None => std::env::remove_var(MASTER_KEY_ENV),
    }
}

#[test]
fn resolve_falls_back_to_master_key_file_when_env_unset() {
    let _guard = env_mutex().lock().unwrap_or_else(|e| e.into_inner());
    let home = tempfile::tempdir().unwrap();
    let key_dir = home.path().join(".skycot");
    fs::create_dir_all(&key_dir).unwrap();
    let key = MasterKey::generate();
    let secret = key.as_age_secret();
    let key_path = key_dir.join(".master_key");
    fs::write(&key_path, &secret).unwrap();
    #[cfg(unix)]
    fs::set_permissions(&key_path, fs::Permissions::from_mode(0o600)).unwrap();

    let prior = std::env::var(MASTER_KEY_ENV).ok();
    std::env::remove_var(MASTER_KEY_ENV);
    let resolved = MasterKey::resolve(Some(home.path())).unwrap();
    assert_eq!(resolved.as_age_secret(), secret);
    if let Some(v) = prior {
        std::env::set_var(MASTER_KEY_ENV, v);
    }
}

#[test]
#[cfg(unix)]
fn migrate_warns_on_loose_perms() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("credentials.toml");
    let key = make_key();
    write_plain_legacy(&path, &[("X", "v")]);
    fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
    let report = migrate_plaintext_in_place(&path, &key).unwrap();
    assert!(report.perms_warning);
    // Rewritten file is tightened back to 0600.
    let mode = fs::metadata(&path).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o600);
}

#[test]
fn read_encrypted_missing_file_returns_empty_map() {
    let dir = tempfile::tempdir().unwrap();
    let key = make_key();
    let loaded = read_encrypted(&dir.path().join("nope.toml"), &key).unwrap();
    assert!(loaded.is_empty());
}
