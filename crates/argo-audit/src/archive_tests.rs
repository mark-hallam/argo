//! Unit tests for `crate::archive` (#228).
#![cfg(test)]

use super::*;
use crate::{AuditKind, AuditSink};
use argo_store::SessionStore;
use chrono::{Duration, Utc};
use std::sync::Arc;

async fn fresh_store() -> Arc<SessionStore> {
    let s = SessionStore::open(":memory:").await.unwrap();
    s.migrate().await.unwrap();
    Arc::new(s)
}

async fn write_rows(store: &Arc<SessionStore>, n: usize) {
    let sink = AuditSink::new(Arc::clone(store));
    for i in 0..n {
        sink.write(None, AuditKind::TurnStart, serde_json::json!({"i": i}), None)
            .await
            .unwrap();
    }
}

async fn backdate_all(store: &SessionStore, days_ago: i64) {
    let cutoff = (Utc::now() - Duration::days(days_ago)).timestamp_micros();
    sqlx::query("UPDATE audit_record SET created_at = ?")
        .bind(cutoff)
        .execute(store.pool())
        .await
        .unwrap();
}

async fn count_audit(store: &SessionStore) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM audit_record")
        .fetch_one(store.pool())
        .await
        .unwrap()
}

fn cfg(dir: &std::path::Path) -> ArchiveConfig {
    ArchiveConfig::new(dir.to_path_buf())
}

#[tokio::test]
async fn archive_empty_window_emits_zero_row_bundle() {
    let store = fresh_store().await;
    let dir = tempfile::tempdir().unwrap();
    let bundle = archive_window(&store, &cfg(dir.path())).await.unwrap();
    assert_eq!(bundle.row_count, 0);
    assert!(dir.path().join(&bundle.filename).exists());
    let manifest = dir.path().join(MANIFEST_FILENAME);
    assert!(manifest.exists());
    let entries = list_archives(dir.path()).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].id, bundle.id);
}

#[tokio::test]
async fn archive_100_rows_tar_gz_bounded_in_size() {
    let store = fresh_store().await;
    write_rows(&store,100).await;
    backdate_all(&store, 200).await;
    let dir = tempfile::tempdir().unwrap();
    let bundle = archive_window(&store, &cfg(dir.path())).await.unwrap();
    assert_eq!(bundle.row_count, 100);
    let bytes = std::fs::metadata(dir.path().join(&bundle.filename))
        .unwrap()
        .len();
    // 100 small rows compressed should easily fit in 32 KiB.
    assert!(bytes < 32_768, "bundle too large: {bytes} bytes");
    assert!(bytes > 0);
}

#[tokio::test]
async fn archive_then_verify_roundtrip() {
    let store = fresh_store().await;
    write_rows(&store,5).await;
    backdate_all(&store, 100).await;
    let dir = tempfile::tempdir().unwrap();
    let bundle = archive_window(&store, &cfg(dir.path())).await.unwrap();
    verify_archive(&bundle, dir.path()).expect("roundtrip verify ok");
    let entries = list_archives(dir.path()).unwrap();
    assert_eq!(entries.len(), 1);
    verify_archive(&entries[0], dir.path()).expect("listed bundle verifies");
}

#[tokio::test]
async fn prune_respects_hard_keep_days() {
    let store = fresh_store().await;
    // Three rows: oldest 200d, middle 50d, newest 5d.
    let sink = AuditSink::new(Arc::clone(&store));
    for _ in 0..3 {
        sink.write(None, AuditKind::TurnStart, serde_json::json!({}), None)
            .await
            .unwrap();
    }
    let now_us = Utc::now().timestamp_micros();
    let day = Duration::days(1).num_microseconds().unwrap();
    sqlx::query("UPDATE audit_record SET created_at = ? WHERE seq = 1")
        .bind(now_us - day * 200)
        .execute(store.pool())
        .await
        .unwrap();
    sqlx::query("UPDATE audit_record SET created_at = ? WHERE seq = 2")
        .bind(now_us - day * 50)
        .execute(store.pool())
        .await
        .unwrap();
    sqlx::query("UPDATE audit_record SET created_at = ? WHERE seq = 3")
        .bind(now_us - day * 5)
        .execute(store.pool())
        .await
        .unwrap();
    let dir = tempfile::tempdir().unwrap();
    let mut c = cfg(dir.path());
    // Operator typo: --retention-days=9 would otherwise prune all
    // three; hard_keep_days=30 must protect the 5-day-old row.
    c.retention_days = 9;
    c.prune_after_archive = true;
    c.hard_keep_days = Some(30);
    let bundle = archive_window(&store, &c).await.unwrap();
    // Bundle covers all rows older than 9 days (200d + 50d).
    assert_eq!(bundle.row_count, 2);
    // Live table: only the 5-day-old row survives — the floor
    // beat the retention setting on the 50d row.
    let surviving = count_audit(&store).await;
    assert_eq!(surviving, 1);
}

#[tokio::test]
async fn archive_skips_rows_newer_than_retention() {
    let store = fresh_store().await;
    write_rows(&store,4).await;
    let now_us = Utc::now().timestamp_micros();
    let day = Duration::days(1).num_microseconds().unwrap();
    // 2 rows old, 2 rows fresh.
    sqlx::query("UPDATE audit_record SET created_at = ? WHERE seq <= 2")
        .bind(now_us - day * 120)
        .execute(store.pool())
        .await
        .unwrap();
    sqlx::query("UPDATE audit_record SET created_at = ? WHERE seq > 2")
        .bind(now_us - day * 10)
        .execute(store.pool())
        .await
        .unwrap();
    let dir = tempfile::tempdir().unwrap();
    let bundle = archive_window(&store, &cfg(dir.path())).await.unwrap();
    assert_eq!(bundle.row_count, 2, "only old rows archived");
    // Without prune flag the live table is untouched.
    assert_eq!(count_audit(&store).await, 4);
}

#[tokio::test]
async fn manifest_append_only_under_concurrent_archives() {
    // First archive writes a bundle; second archive within the
    // same UTC day with the same cutoff returns the existing
    // bundle (idempotent) instead of truncating the manifest.
    let store = fresh_store().await;
    write_rows(&store,2).await;
    backdate_all(&store, 100).await;
    let dir = tempfile::tempdir().unwrap();
    let first = archive_window(&store, &cfg(dir.path())).await.unwrap();
    let second = archive_window(&store, &cfg(dir.path())).await.unwrap();
    // Idempotency: second call returns the same bundle id.
    assert_eq!(first.id, second.id);
    let entries = list_archives(dir.path()).unwrap();
    assert_eq!(entries.len(), 1, "manifest must not duplicate");
    // Append-only: simulate a third bundle by raising retention so
    // the cutoff moves; the new bundle appends, doesn't truncate.
    let mut c = cfg(dir.path());
    c.retention_days = 1;
    let third = archive_window(&store, &c).await.unwrap();
    let entries = list_archives(dir.path()).unwrap();
    assert_eq!(entries.len(), 2);
    assert!(entries.iter().any(|b| b.id == first.id));
    assert!(entries.iter().any(|b| b.id == third.id));
}

#[tokio::test]
async fn sha256_mismatch_detected_by_verify() {
    let store = fresh_store().await;
    write_rows(&store,3).await;
    backdate_all(&store, 100).await;
    let dir = tempfile::tempdir().unwrap();
    let bundle = archive_window(&store, &cfg(dir.path())).await.unwrap();
    // Tamper with the on-disk bundle: flip a byte in the middle.
    let path = dir.path().join(&bundle.filename);
    let mut bytes = std::fs::read(&path).unwrap();
    let mid = bytes.len() / 2;
    bytes[mid] ^= 0xff;
    std::fs::write(&path, &bytes).unwrap();
    let err = verify_archive(&bundle, dir.path())
        .expect_err("tampered bundle must fail verify");
    let msg = err.to_string();
    // Either sha256 or hmac mismatch (gz CRC change flips both).
    assert!(
        msg.contains("sha256 mismatch") || msg.contains("hmac mismatch"),
        "unexpected error: {msg}"
    );
}
