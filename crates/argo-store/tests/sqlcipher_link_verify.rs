//! Integration-level confirmation that the build is linked against
//! SQLCipher rather than vanilla SQLite (mission-sqlcipher-1-
//! libsqlite3-patch).
//!
//! Compiled only when the `bundled-sqlcipher` feature is on; the
//! default unencrypted build skips this file entirely so the gate
//! never affects the regular `cargo test -p argo-store` run.
//!
//! What we assert: a fresh sqlx pool, opened against an on-disk
//! SQLite file and keyed via `PRAGMA key`, reports a non-empty
//! `PRAGMA cipher_version`. Vanilla SQLite silently ignores both
//! pragmas and returns no row for `cipher_version`; SQLCipher
//! returns a string like `"4.5.5 community"`. That single row is
//! the load-bearing signal.

#![cfg(feature = "bundled-sqlcipher")]

use sqlx::Row;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use std::str::FromStr;

/// Hex-encoded 32-byte test key (all 0x11 bytes). Not a real secret.
const TEST_KEY_HEX: &str =
    "1111111111111111111111111111111111111111111111111111111111111111";

#[tokio::test]
async fn pragma_cipher_version_reports_sqlcipher_engine() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let db_path = tmp.path().join("link_verify.db");
    let url = format!("sqlite://{}", db_path.display());

    let opts = SqliteConnectOptions::from_str(&url)
        .expect("connect opts")
        .create_if_missing(true);
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(opts)
        .await
        .expect("open pool");

    let key_stmt = format!("PRAGMA key = \"x'{TEST_KEY_HEX}'\"");
    sqlx::query(&key_stmt)
        .execute(&pool)
        .await
        .expect("apply PRAGMA key");

    let row = sqlx::query("PRAGMA cipher_version")
        .fetch_optional(&pool)
        .await
        .expect("query cipher_version");

    let row = row.expect(
        "PRAGMA cipher_version returned no row — build is not linked \
         against SQLCipher (vanilla SQLite silently drops this pragma)",
    );
    let version: String = row.try_get(0).expect("cipher_version column 0");
    assert!(
        !version.trim().is_empty(),
        "cipher_version returned an empty string — SQLCipher reports \
         e.g. '4.5.5 community'; empty value means the link is wrong"
    );
    eprintln!("SQLCipher linked: cipher_version = {version}");

    sqlx::query("CREATE TABLE t (v INTEGER)")
        .execute(&pool)
        .await
        .expect("create table on encrypted db");
    sqlx::query("INSERT INTO t (v) VALUES (42)")
        .execute(&pool)
        .await
        .expect("insert into encrypted db");
    let v: i64 = sqlx::query_scalar("SELECT v FROM t")
        .fetch_one(&pool)
        .await
        .expect("read back from encrypted db");
    assert_eq!(v, 42);
}
