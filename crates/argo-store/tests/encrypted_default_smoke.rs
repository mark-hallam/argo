//! `SessionStore::open_with_hex_key` smoke (PR 4 of
//! mission-sqlcipher-*).
//!
//! Confirms the v1 encrypted-by-default posture: opening a store with
//! an explicit SQLCipher hex key writes encrypted pages that the same
//! key can re-open, and re-opening WITHOUT the key (`open_unencrypted_for_tests`)
//! fails to read the encrypted contents. Compiled only when the
//! `bundled-sqlcipher` feature is on — without a SQLCipher-linked
//! libsqlite3 the `PRAGMA key` is a silent no-op and the round-trip
//! reduces to plaintext, which would defeat the test.

#![cfg(feature = "bundled-sqlcipher")]

use argo_store::wal_tuning::WalTuningProfile;
use argo_store::SessionStore;

const TEST_HEX_KEY: &str =
    "2222222222222222222222222222222222222222222222222222222222222222";

#[tokio::test]
async fn fresh_db_round_trips_through_correct_key() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let db = tmp.path().join("session.db");

    let store = SessionStore::open_with_hex_key(&db, WalTuningProfile::Default, TEST_HEX_KEY)
        .await
        .expect("open with key");
    store.migrate().await.expect("migrate");
    sqlx::query("CREATE TABLE secret_marker (v TEXT NOT NULL)")
        .execute(store.pool())
        .await
        .expect("create marker");
    sqlx::query("INSERT INTO secret_marker (v) VALUES ('tenant-confidential')")
        .execute(store.pool())
        .await
        .expect("insert marker");
    store.close().await;

    // Reopen with the same key — must succeed and see the row.
    let reopened =
        SessionStore::open_with_hex_key(&db, WalTuningProfile::Default, TEST_HEX_KEY)
            .await
            .expect("reopen with key");
    let v: String = sqlx::query_scalar("SELECT v FROM secret_marker")
        .fetch_one(reopened.pool())
        .await
        .expect("read marker back");
    assert_eq!(v, "tenant-confidential");
    reopened.close().await;
}

#[tokio::test]
async fn unencrypted_reopen_of_encrypted_db_fails_loud() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let db = tmp.path().join("session.db");

    let store = SessionStore::open_with_hex_key(&db, WalTuningProfile::Default, TEST_HEX_KEY)
        .await
        .expect("open with key");
    store.migrate().await.expect("migrate");
    sqlx::query("CREATE TABLE secret_marker (v TEXT)")
        .execute(store.pool())
        .await
        .expect("create marker");
    store.close().await;

    // Now try to open the same path as if it were plaintext — SQLCipher
    // should refuse with "file is not a database" / similar when the
    // first non-PRAGMA query runs.
    let plain = SessionStore::open_unencrypted_for_tests(&db).await;
    let err = match plain {
        Ok(s) => match sqlx::query_scalar::<_, String>("SELECT v FROM secret_marker")
            .fetch_optional(s.pool())
            .await
        {
            Ok(_) => panic!("encrypted DB readable without key — SQLCipher link broken"),
            Err(e) => format!("{e}"),
        },
        Err(e) => format!("{e}"),
    };
    let lowered = err.to_ascii_lowercase();
    assert!(
        lowered.contains("not a database")
            || lowered.contains("encrypted")
            || lowered.contains("malformed")
            || lowered.contains("pool timed out"),
        "expected SQLCipher mismatch error, got: {err}",
    );
}
