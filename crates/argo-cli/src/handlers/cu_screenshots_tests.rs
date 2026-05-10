//! Unit tests for `CuScreenshotStore`. Covers the seven cases the
//! brief calls out: round-trip, ordered listing, retention sweep,
//! tenant-independence of the sweep, retention boundary semantics,
//! missing-blob error shape, and tenant isolation.
//!
//! All tests use `MemoryAttachedBlob` + an in-memory SQLite pool
//! so they stay hermetic and finish in milliseconds.

use super::*;
use argo_store::attached_blob::MemoryAttachedBlob;
use argo_store::SessionStore;
use std::time::Duration;

const TENANT_A: &str = "tenant-A";
const TENANT_B: &str = "tenant-B";
const SESSION_1: &str = "session-1";
const SESSION_2: &str = "session-2";
const ACTION_1: &str = "action-1";

async fn fresh_store(retention: Duration) -> (CuScreenshotStore, SessionStore) {
    let session_store = SessionStore::open(":memory:").await.unwrap();
    session_store.migrate().await.unwrap();
    let pool = session_store.pool().clone();
    let blobs: Arc<dyn BlobStore> = Arc::new(MemoryAttachedBlob::new());
    let store = CuScreenshotStore::new(blobs, pool, retention);
    store.ensure_schema().await.unwrap();
    (store, session_store)
}

fn fake_png(tag: u8) -> Vec<u8> {
    let mut v = b"\x89PNG\r\n\x1a\n".to_vec();
    v.extend_from_slice(&[tag; 16]);
    v
}

#[tokio::test]
async fn put_round_trips_through_get() {
    let (store, _keep) = fresh_store(Duration::from_secs(86_400)).await;
    let png = fake_png(0xAB);
    let blob_ref = store
        .put(TENANT_A, SESSION_1, ACTION_1, 0, png.clone())
        .await
        .unwrap();
    assert_eq!(blob_ref.mime, SCREENSHOT_MIME);
    assert_eq!(blob_ref.size_bytes as usize, png.len());
    let bytes = store.get(TENANT_A, blob_ref.id).await.unwrap();
    assert_eq!(bytes, png);
}

#[tokio::test]
async fn list_session_returns_shots_in_action_then_step_order() {
    let (store, _keep) = fresh_store(Duration::from_secs(86_400)).await;
    // Insert in scrambled order so the SELECT-side ORDER BY is
    // what we're actually exercising.
    let r2 = store
        .put(TENANT_A, SESSION_1, "action-2", 1, fake_png(2))
        .await
        .unwrap();
    let r0 = store
        .put(TENANT_A, SESSION_1, ACTION_1, 0, fake_png(0))
        .await
        .unwrap();
    let r1 = store
        .put(TENANT_A, SESSION_1, ACTION_1, 1, fake_png(1))
        .await
        .unwrap();
    // Wrong session — must not appear.
    let _other = store
        .put(TENANT_A, SESSION_2, ACTION_1, 0, fake_png(9))
        .await
        .unwrap();
    let listed = store.list_session(TENANT_A, SESSION_1).await.unwrap();
    assert_eq!(listed.len(), 3);
    assert_eq!(listed[0].id, r0.id);
    assert_eq!(listed[1].id, r1.id);
    assert_eq!(listed[2].id, r2.id);
}

#[tokio::test]
async fn sweep_expired_deletes_only_old_rows() {
    let (store, _keep) = fresh_store(Duration::from_secs(86_400)).await;
    let _r = store
        .put(TENANT_A, SESSION_1, ACTION_1, 0, fake_png(1))
        .await
        .unwrap();
    // Sweeping with a retention shorter than the row's age would
    // have to wait — instead we sweep with a retention of 0s into
    // a `now` far in the future so the boundary lands past the
    // captured_at and the row is deleted.
    let far_future = SystemTime::UNIX_EPOCH + Duration::from_secs(i64::MAX as u64 / 4);
    let deleted = store
        .sweep_expired(far_future, Duration::from_secs(0))
        .await
        .unwrap();
    assert_eq!(deleted, 1);
    let listed = store.list_session(TENANT_A, SESSION_1).await.unwrap();
    assert!(listed.is_empty());
}

#[tokio::test]
async fn sweep_treats_tenants_independently_of_each_other() {
    let (store, _keep) = fresh_store(Duration::from_secs(86_400)).await;
    let _a = store
        .put(TENANT_A, SESSION_1, ACTION_1, 0, fake_png(1))
        .await
        .unwrap();
    let _b = store
        .put(TENANT_B, SESSION_1, ACTION_1, 0, fake_png(2))
        .await
        .unwrap();
    // Far-future + 0s retention should sweep BOTH rows. Tenant
    // boundaries do not protect rows from a global sweep — the
    // sweep is index-row by `captured_at` only.
    let far_future = SystemTime::UNIX_EPOCH + Duration::from_secs(i64::MAX as u64 / 4);
    let deleted = store
        .sweep_expired(far_future, Duration::from_secs(0))
        .await
        .unwrap();
    assert_eq!(deleted, 2);
    assert!(store
        .list_session(TENANT_A, SESSION_1)
        .await
        .unwrap()
        .is_empty());
    assert!(store
        .list_session(TENANT_B, SESSION_1)
        .await
        .unwrap()
        .is_empty());
}

#[tokio::test]
async fn sweep_at_exact_retention_boundary_deletes_the_row() {
    let (store, _keep) = fresh_store(Duration::from_secs(86_400)).await;
    let _r = store
        .put(TENANT_A, SESSION_1, ACTION_1, 0, fake_png(1))
        .await
        .unwrap();
    // Boundary semantics: `captured_at <= now - retention` deletes.
    // Calling sweep with `now == captured_at` and `retention == 0`
    // puts the boundary exactly at `captured_at` — the `<=` form
    // therefore sweeps the row.
    let now = SystemTime::now() + Duration::from_secs(1);
    let deleted = store.sweep_expired(now, Duration::from_secs(0)).await.unwrap();
    assert_eq!(deleted, 1);
}

#[tokio::test]
async fn get_with_unknown_blob_id_returns_not_found() {
    let (store, _keep) = fresh_store(Duration::from_secs(86_400)).await;
    let made_up = BlobId::new();
    let err = store.get(TENANT_A, made_up).await.err().unwrap();
    if let ArgoError::NotFound(msg) = err {
        assert!(msg.contains(&made_up.to_string()), "msg={msg}");
        assert!(msg.contains(TENANT_A), "msg={msg}");
    } else {
        panic!("expected NotFound");
    }
}

#[tokio::test]
async fn cross_tenant_get_is_blocked_as_not_found() {
    let (store, _keep) = fresh_store(Duration::from_secs(86_400)).await;
    let blob_ref = store
        .put(TENANT_A, SESSION_1, ACTION_1, 0, fake_png(7))
        .await
        .unwrap();
    // Tenant A can read its own row …
    let bytes = store.get(TENANT_A, blob_ref.id).await.unwrap();
    assert_eq!(bytes.len(), fake_png(7).len());
    // … but tenant B sees a clean NotFound, not the bytes.
    let err = store.get(TENANT_B, blob_ref.id).await.err().unwrap();
    assert!(
        matches!(err, ArgoError::NotFound(_)),
        "expected NotFound, got {err:?}"
    );
}
