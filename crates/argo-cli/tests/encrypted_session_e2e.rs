//! End-to-end smoke for the gateway encrypted-by-default flip
//! (`mission-sqlcipher-4-default-flip` PR 4 of 4).
//!
//! Mirrors the gateway's `open_store_at` helper at the integration
//! boundary by composing the same primitives — `argo_config::cred::
//! db_key::get_or_create_db_key` via its env-var precedence,
//! `argo_store::encryption::base64_to_hex`, and
//! `SessionStore::open_with_hex_key`. The check is structural: a
//! file-backed session DB written through the boot-flip pipeline
//! must (a) round-trip when re-opened with the same key, and
//! (b) reject the wrong key — proving the `PRAGMA key` is
//! load-bearing on the gateway path.
//!
//! Feature-gated on `argo-store/bundled-sqlcipher`: with the default
//! pure-Rust sqlx driver `PRAGMA key` is silently dropped, the file
//! stays plaintext, and the "wrong key" branch can't fail — which
//! would defeat the test. Verify with:
//!
//! ```text
//! cargo test -p argo-cli --features argo-store/bundled-sqlcipher \
//!   --test encrypted_session_e2e
//! ```

#![cfg(feature = "bundled-sqlcipher")]

use argo_config::cred::db_key::{get_or_create_db_key, DB_KEY_ENV};
use argo_store::encryption::base64_to_hex;
use argo_store::wal_tuning::WalTuningProfile;
use argo_store::SessionStore;
use argo_types::TenantId;
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use secrecy::ExposeSecret;
use tokio::sync::Mutex;

// Serialise env-var mutation across the two tests in this file so
// they can run with the default `cargo test` thread pool. Async-
// aware tokio Mutex avoids the clippy::await_holding_lock that the
// std::sync::Mutex variant trips because the guard is held across
// `SessionStore::open_with_hex_key.await`.
fn env_lock() -> &'static Mutex<()> {
    use std::sync::OnceLock;
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

#[tokio::test]
async fn boot_path_round_trips_through_env_key() {
    let _guard = env_lock().lock().await;
    let raw = [0x42u8; 32];
    let b64 = B64.encode(raw);
    std::env::set_var(DB_KEY_ENV, &b64);
    let tenant = TenantId(uuid::Uuid::nil());
    let secret = get_or_create_db_key(tenant)
        .await
        .expect("derive key via env precedence");
    let hex = base64_to_hex(secret.expose_secret()).expect("base64 → hex");
    let tmp = tempfile::tempdir().expect("tempdir");
    let db = tmp.path().join("session.db");

    let store = SessionStore::open_with_hex_key(&db, WalTuningProfile::Default, &hex)
        .await
        .expect("first open");
    store.migrate().await.expect("migrate");
    sqlx::query("CREATE TABLE token_marker (jwt TEXT NOT NULL)")
        .execute(store.pool())
        .await
        .expect("create token_marker");
    sqlx::query("INSERT INTO token_marker (jwt) VALUES ('magic-link-stub')")
        .execute(store.pool())
        .await
        .expect("insert token_marker");
    store.close().await;

    // Same env → same key → same DB readable again.
    let reopened = SessionStore::open_with_hex_key(&db, WalTuningProfile::Default, &hex)
        .await
        .expect("reopen with key");
    let row: String = sqlx::query_scalar("SELECT jwt FROM token_marker")
        .fetch_one(reopened.pool())
        .await
        .expect("read token_marker back");
    assert_eq!(row, "magic-link-stub");
    reopened.close().await;

    std::env::remove_var(DB_KEY_ENV);
}

#[tokio::test]
async fn wrong_key_rejects_existing_encrypted_db() {
    let _guard = env_lock().lock().await;
    // Shrink the 30-second default acquire-timeout — the wrong-key
    // pool repeatedly fails its `after_connect` hook and we want the
    // pool-timeout signal to surface fast. Set early so
    // `StorePoolConfig::from_env` picks it up inside the open call.
    if std::env::var_os("ARGO_STORE_POOL_ACQUIRE_TIMEOUT_MS").is_none() {
        std::env::set_var("ARGO_STORE_POOL_ACQUIRE_TIMEOUT_MS", "2000");
    }
    let right_hex =
        "33333333333333333333333333333333333333333333333333333333333333".to_string()
            + "33";
    let wrong_hex =
        "44444444444444444444444444444444444444444444444444444444444444".to_string()
            + "44";
    let tmp = tempfile::tempdir().expect("tempdir");
    let db = tmp.path().join("session.db");

    let store = SessionStore::open_with_hex_key(&db, WalTuningProfile::Default, &right_hex)
        .await
        .expect("seed open");
    store.migrate().await.expect("migrate");
    sqlx::query("CREATE TABLE token_marker (jwt TEXT)")
        .execute(store.pool())
        .await
        .expect("create token_marker");
    store.close().await;

    let attempt =
        SessionStore::open_with_hex_key(&db, WalTuningProfile::Default, &wrong_hex).await;
    let err_text = match attempt {
        Ok(s) => match sqlx::query_scalar::<_, String>("SELECT jwt FROM token_marker")
            .fetch_optional(s.pool())
            .await
        {
            Ok(_) => panic!("wrong key opened an encrypted DB — SQLCipher link broken"),
            Err(e) => format!("{e}"),
        },
        Err(e) => format!("{e}"),
    };
    let lowered = err_text.to_ascii_lowercase();
    assert!(
        lowered.contains("not a database")
            || lowered.contains("encrypted")
            || lowered.contains("malformed")
            || lowered.contains("pool timed out"),
        "expected SQLCipher key-mismatch error, got: {err_text}",
    );
}
