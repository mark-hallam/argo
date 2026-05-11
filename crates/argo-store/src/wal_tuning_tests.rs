//! Unit tests for [`super::WalTuningProfile`] + [`super::apply_pragmas`].
//!
//! Mock-aware: every test runs against `:memory:` so it never
//! touches the host filesystem. SQLite silently ignores
//! `PRAGMA journal_mode = WAL` on memory DBs (it reports `memory`
//! back), so the round-trip assertions skip that specific pragma
//! when the underlying mode is `memory`.
#![cfg(test)]

use super::{apply_pragmas, default_pragmas, read_pragma, WalTuningProfile};
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::{Pool, Sqlite};
use std::str::FromStr;

async fn memory_pool() -> Pool<Sqlite> {
    SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap()
}

#[test]
fn default_profile_string_parses() {
    assert_eq!(
        WalTuningProfile::from_str("default").unwrap(),
        WalTuningProfile::Default
    );
    assert_eq!(
        WalTuningProfile::from_str("conservative").unwrap(),
        WalTuningProfile::Conservative
    );
    assert_eq!(
        WalTuningProfile::from_str("aggressive").unwrap(),
        WalTuningProfile::Aggressive
    );
    // Mixed case + whitespace is permissive.
    assert_eq!(
        WalTuningProfile::from_str("  Default ").unwrap(),
        WalTuningProfile::Default
    );
}

#[test]
fn missing_config_defaults_to_default_profile() {
    // Empty string represents "missing [store.wal_tuning] block"
    // — must round-trip to the Default profile so a stripped
    // argo.toml still benefits from the opinionated pragmas.
    assert_eq!(
        WalTuningProfile::from_str("").unwrap(),
        WalTuningProfile::Default
    );
    assert_eq!(WalTuningProfile::default(), WalTuningProfile::Default);
}

#[test]
fn bad_profile_string_errors() {
    let err = WalTuningProfile::from_str("bogus").unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("bogus"), "error should name the bad input: {msg}");
    assert!(
        msg.contains("conservative") && msg.contains("default") && msg.contains("aggressive"),
        "error should list the accepted values: {msg}"
    );
}

#[tokio::test]
async fn default_profile_applies_all_pragmas() {
    let pool = memory_pool().await;
    apply_pragmas(&pool, WalTuningProfile::Default).await.unwrap();

    // Cross-check every pragma. `journal_mode` on memory DBs
    // reports `memory`, and `mmap_size` on memory DBs is reported
    // back as the OS shm-disabled marker (empty/0) — both expected
    // and not a failure.
    for (name, expected) in default_pragmas() {
        let actual = read_pragma(&pool, name).await.unwrap();
        match name {
            "journal_mode" => assert!(
                actual == "wal" || actual == "memory",
                "journal_mode={actual} expected wal or memory"
            ),
            "synchronous" => assert_eq!(actual, "1", "synchronous=NORMAL maps to 1, got {actual}"),
            "temp_store" => assert_eq!(actual, "2", "temp_store=MEMORY maps to 2, got {actual}"),
            "mmap_size" => assert!(
                actual == expected || actual == "0" || actual.is_empty(),
                "mmap_size={actual} expected {expected} or 0 / empty (memory DB)"
            ),
            _ => assert_eq!(
                actual, expected,
                "pragma {name} expected {expected}, got {actual}"
            ),
        }
    }
}

#[tokio::test]
async fn conservative_profile_leaves_sqlite_defaults() {
    let pool = memory_pool().await;
    // Snapshot defaults BEFORE apply — these are sqlx/sqlite baseline.
    let sync_before = read_pragma(&pool, "synchronous").await.unwrap();
    let cache_before = read_pragma(&pool, "cache_size").await.unwrap();

    apply_pragmas(&pool, WalTuningProfile::Conservative)
        .await
        .unwrap();

    let sync_after = read_pragma(&pool, "synchronous").await.unwrap();
    let cache_after = read_pragma(&pool, "cache_size").await.unwrap();
    assert_eq!(
        sync_before, sync_after,
        "Conservative must not touch synchronous"
    );
    assert_eq!(
        cache_before, cache_after,
        "Conservative must not touch cache_size"
    );
}

#[tokio::test]
async fn aggressive_profile_sets_synchronous_off_and_higher_checkpoint() {
    let pool = memory_pool().await;
    apply_pragmas(&pool, WalTuningProfile::Aggressive)
        .await
        .unwrap();

    // synchronous = OFF → 0
    let sync = read_pragma(&pool, "synchronous").await.unwrap();
    assert_eq!(sync, "0", "Aggressive synchronous=OFF maps to 0, got {sync}");

    let autocheckpoint = read_pragma(&pool, "wal_autocheckpoint").await.unwrap();
    assert_eq!(
        autocheckpoint, "10000",
        "Aggressive wal_autocheckpoint=10000, got {autocheckpoint}"
    );

    // Shared pragmas with Default still apply.
    let cache = read_pragma(&pool, "cache_size").await.unwrap();
    assert_eq!(cache, "-64000", "Aggressive shares cache_size with Default");
}

#[tokio::test]
async fn pragma_roundtrip_via_memory_db() {
    // Single-pool round-trip — apply Default, mutate a value
    // back through `apply_pragmas`, confirm the latest write
    // sticks. Guards against accidental "apply once then drop"
    // patterns.
    let pool = memory_pool().await;
    apply_pragmas(&pool, WalTuningProfile::Default).await.unwrap();
    let busy_first = read_pragma(&pool, "busy_timeout").await.unwrap();
    assert_eq!(busy_first, "5000");

    // Aggressive shares busy_timeout=5000 — but flipping back to
    // Conservative is a no-op; the prior value persists.
    apply_pragmas(&pool, WalTuningProfile::Conservative)
        .await
        .unwrap();
    let busy_after = read_pragma(&pool, "busy_timeout").await.unwrap();
    assert_eq!(
        busy_after, "5000",
        "Conservative is a no-op; previously-set pragmas stick"
    );
}
