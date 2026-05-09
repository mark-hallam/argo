//! Tests for the `attached_blob` typed scaffold.

use super::{AttachedBlob, BlobId, BlobStore, MemoryAttachedBlob};
use argo_types::ArgoError;

#[tokio::test]
async fn put_then_get_round_trips_payload() {
    let store = MemoryAttachedBlob::new();
    let payload = b"screenshot-bytes-1234".to_vec();
    let blob = AttachedBlob::new(payload.clone(), "image/png".into());
    let original_id = blob.id;
    let original_sha = blob.sha256;

    let r = store.put(blob).await.unwrap();
    assert_eq!(r.id, original_id);
    assert_eq!(r.size_bytes, payload.len() as u64);
    assert_eq!(r.sha256, original_sha);
    assert_eq!(r.mime, "image/png");
    assert_eq!(store.len(), 1);

    let fetched = store.get(original_id).await.unwrap();
    assert_eq!(fetched.bytes, payload);
    assert_eq!(fetched.sha256, original_sha);
    assert_eq!(fetched.mime, "image/png");
}

#[tokio::test]
async fn get_unknown_id_returns_not_found() {
    let store = MemoryAttachedBlob::new();
    let err = store.get(BlobId::new()).await.unwrap_err();
    match err {
        ArgoError::NotFound(msg) => assert!(msg.contains("blob")),
        other => panic!("expected NotFound, got {other:?}"),
    }
}

#[tokio::test]
async fn put_two_distinct_blobs_keeps_both() {
    let store = MemoryAttachedBlob::new();
    let a = AttachedBlob::new(b"a".to_vec(), "text/plain".into());
    let b = AttachedBlob::new(b"b".to_vec(), "text/plain".into());
    let a_id = a.id;
    let b_id = b.id;
    let _ = store.put(a).await.unwrap();
    let _ = store.put(b).await.unwrap();
    assert_eq!(store.len(), 2);
    assert_eq!(store.get(a_id).await.unwrap().bytes, b"a");
    assert_eq!(store.get(b_id).await.unwrap().bytes, b"b");
}

#[test]
fn attached_blob_new_computes_sha256_and_size() {
    let blob = AttachedBlob::new(b"hello".to_vec(), "text/plain".into());
    assert_eq!(blob.size_bytes, 5);
    // SHA-256("hello")
    let want_hex = "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824";
    let got_hex: String = blob.sha256.iter().map(|b| format!("{b:02x}")).collect();
    assert_eq!(got_hex, want_hex);
}

#[test]
fn blob_id_default_and_new_differ() {
    let a = BlobId::new();
    let b = BlobId::default();
    assert_ne!(a, b);
}
