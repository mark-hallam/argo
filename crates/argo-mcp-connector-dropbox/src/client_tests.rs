//! Wiremock-backed unit tests for `DropboxConnector`.
//!
//! Six scenarios per the brief:
//!  1. `list_folder` round-trips a typed entry array (file + folder + ignored
//!     deleted tombstone).
//!  2. `upload` with `Overwrite` POSTs to the content host, carries the
//!     `Dropbox-API-Arg` header, and decodes the returned file metadata.
//!  3. `download` posts to the content host, returns the response body
//!     bytes verbatim, and asserts the round-trip is byte-identical.
//!  4. `delete` returns `()` from a minimal `{}` response body — exercising
//!     the unit-return path through the JSON decode helper.
//!  5. `create_shared_link` (public visibility) decodes the URL + expiry.
//!  6. `upload` rejects an oversized payload BEFORE any network call —
//!     proves the cap is enforced client-side, not relied on from the wire.
//!
//! A seventh test exercises `search_v2`'s doubly-nested metadata wrapper,
//! which is the Dropbox-shape most likely to break on a future schema bump.

use super::*;
use crate::types::{DbxEntry, DbxWriteMode};
use argo_mcp_connector_shared::RetryPolicy;
use wiremock::matchers::{body_bytes, body_json_string, header, header_exists, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn no_retry() -> RetryPolicy {
    RetryPolicy {
        max_attempts: 1,
        initial_delay_ms: 0,
        multiplier: 1.0,
        max_delay_ms: 0,
        jitter_ms: 0,
    }
}

fn connector(api: &MockServer, content: &MockServer) -> DropboxConnector {
    DropboxConnector::new("seed-token".into())
        .with_api_base(api.uri())
        .with_content_base(content.uri())
        .with_retry_policy(no_retry())
}

#[tokio::test]
async fn list_folder_round_trips_mixed_entries() {
    let api = MockServer::start().await;
    let content = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/2/files/list_folder"))
        .and(header("authorization", "Bearer seed-token"))
        .and(header("content-type", "application/json"))
        .and(body_json_string(r#"{"path":"/inbox","recursive":false}"#))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "entries": [
                {
                    ".tag": "file",
                    "id": "id:abc",
                    "name": "receipt.txt",
                    "path_display": "/inbox/receipt.txt",
                    "size": 42,
                    "content_hash": "deadbeef",
                    "server_modified": "2026-05-11T12:00:00Z"
                },
                {
                    ".tag": "folder",
                    "id": "id:fld",
                    "name": "archive",
                    "path_display": "/inbox/archive"
                },
                {
                    ".tag": "deleted",
                    "name": "old.txt",
                    "path_display": "/inbox/old.txt"
                }
            ]
        })))
        .expect(1)
        .mount(&api)
        .await;

    let dbx = connector(&api, &content);
    let entries = dbx.list_folder("/inbox", false).await.unwrap();
    assert_eq!(entries.len(), 2, "deleted tombstone must be dropped");
    match &entries[0] {
        DbxEntry::File(f) => {
            assert_eq!(f.id, "id:abc");
            assert_eq!(f.name, "receipt.txt");
            assert_eq!(f.path_display, "/inbox/receipt.txt");
            assert_eq!(f.size_bytes, 42);
            assert_eq!(f.content_hash, "deadbeef");
        }
        other => panic!("expected File, got {other:?}"),
    }
    match &entries[1] {
        DbxEntry::Folder(f) => {
            assert_eq!(f.id, "id:fld");
            assert_eq!(f.path_display, "/inbox/archive");
        }
        other => panic!("expected Folder, got {other:?}"),
    }
}

#[tokio::test]
async fn upload_overwrite_posts_to_content_host_with_arg_header() {
    let api = MockServer::start().await;
    let content = MockServer::start().await;
    // The `Dropbox-API-Arg` header carries serde-encoded JSON; field order is
    // serde-implementation-defined, so we assert "header is present" + match
    // the upload body bytes verbatim. The response we get back proves the
    // server side accepted the arg.
    Mock::given(method("POST"))
        .and(path("/2/files/upload"))
        .and(header("authorization", "Bearer seed-token"))
        .and(header("content-type", "application/octet-stream"))
        .and(header_exists("dropbox-api-arg"))
        .and(body_bytes(b"hello".to_vec()))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "id:newfile",
            "name": "r.txt",
            "path_display": "/inbox/r.txt",
            "size": 5,
            "content_hash": "cafef00d",
            "server_modified": "2026-05-11T12:34:56Z"
        })))
        .expect(1)
        .mount(&content)
        .await;

    let dbx = connector(&api, &content);
    let file = dbx
        .upload("/inbox/r.txt", b"hello", DbxWriteMode::Overwrite)
        .await
        .unwrap();
    assert_eq!(file.id, "id:newfile");
    assert_eq!(file.path_display, "/inbox/r.txt");
    assert_eq!(file.size_bytes, 5);
    assert_eq!(file.content_hash, "cafef00d");
}

#[tokio::test]
async fn download_round_trip_returns_raw_bytes() {
    let api = MockServer::start().await;
    let content = MockServer::start().await;
    let payload: &[u8] = b"line one\nline two\n\x00\xffbinary";
    Mock::given(method("POST"))
        .and(path("/2/files/download"))
        .and(header("authorization", "Bearer seed-token"))
        .and(header("dropbox-api-arg", r#"{"path":"/inbox/r.txt"}"#))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header(
                    "Dropbox-API-Result",
                    r#"{"name":"r.txt","path_display":"/inbox/r.txt","size":24}"#,
                )
                .set_body_bytes(payload),
        )
        .expect(1)
        .mount(&content)
        .await;

    let dbx = connector(&api, &content);
    let bytes = dbx.download("/inbox/r.txt").await.unwrap();
    assert_eq!(bytes, payload, "download body must round-trip byte-identical");
}

#[tokio::test]
async fn delete_accepts_minimal_response_body() {
    let api = MockServer::start().await;
    let content = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/2/files/delete_v2"))
        .and(header("authorization", "Bearer seed-token"))
        .and(header("content-type", "application/json"))
        .and(body_json_string(r#"{"path":"/inbox/r.txt"}"#))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({})))
        .expect(1)
        .mount(&api)
        .await;

    let dbx = connector(&api, &content);
    dbx.delete("/inbox/r.txt").await.unwrap();
}

#[tokio::test]
async fn create_shared_link_public_visibility() {
    let api = MockServer::start().await;
    let content = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/2/sharing/create_shared_link_with_settings"))
        .and(header("authorization", "Bearer seed-token"))
        .and(body_json_string(
            r#"{"path":"/inbox/r.txt","settings":{"requested_visibility":"public"}}"#,
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "url": "https://www.dropbox.com/s/abcd/r.txt?dl=0",
            "path_lower": "/inbox/r.txt",
            "expires": "2027-01-01T00:00:00Z"
        })))
        .expect(1)
        .mount(&api)
        .await;
    // Mount a second mock asserting the password visibility serializes
    // correctly under the same endpoint — confirms the acceptance criterion
    // without a separate test method.
    Mock::given(method("POST"))
        .and(path("/2/sharing/create_shared_link_with_settings"))
        .and(body_json_string(
            r#"{"path":"/private/p.txt","settings":{"requested_visibility":"password"}}"#,
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "url": "https://www.dropbox.com/s/xyz/p.txt?dl=0",
            "path_lower": "/private/p.txt"
        })))
        .expect(1)
        .mount(&api)
        .await;

    let dbx = connector(&api, &content);
    let link = dbx.create_shared_link("/inbox/r.txt", false).await.unwrap();
    assert_eq!(link.url, "https://www.dropbox.com/s/abcd/r.txt?dl=0");
    assert_eq!(link.path, "/inbox/r.txt");
    assert!(link.expires.is_some(), "expires must decode when present");

    let pwd = dbx.create_shared_link("/private/p.txt", true).await.unwrap();
    assert_eq!(pwd.expires, None, "absent expires must decode as None");
}

#[tokio::test]
async fn upload_rejects_oversized_payload_without_network_call() {
    let api = MockServer::start().await;
    let content = MockServer::start().await;
    let dbx = connector(&api, &content);
    let oversized = vec![0u8; MAX_UPLOAD_BYTES + 1];
    let err = dbx
        .upload("/inbox/huge.bin", &oversized, DbxWriteMode::Add)
        .await
        .expect_err("oversized upload must be rejected");
    match err {
        ArgoError::Provider(msg) => {
            assert!(msg.contains("150 MiB"), "error must mention cap, got {msg:?}");
            assert!(
                msg.contains("upload_session") || msg.contains("UploadOnly"),
                "error must hint at chunked-upload alternative, got {msg:?}"
            );
        }
        other => panic!("expected ArgoError::Provider, got {other:?}"),
    }
}

#[tokio::test]
async fn search_round_trips_nested_metadata_wrapper() {
    let api = MockServer::start().await;
    let content = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/2/files/search_v2"))
        .and(body_json_string(
            r#"{"query":"receipt","options":{"max_results":5}}"#,
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "matches": [{
                "metadata": {
                    ".tag": "metadata",
                    "metadata": {
                        ".tag": "file",
                        "id": "id:hit",
                        "name": "receipt-jan.txt",
                        "path_display": "/inbox/receipt-jan.txt",
                        "size": 7,
                        "content_hash": "abcd",
                        "server_modified": "2026-05-11T00:00:00Z"
                    }
                }
            }]
        })))
        .expect(1)
        .mount(&api)
        .await;

    let dbx = connector(&api, &content);
    let hits = dbx.search("receipt", 5).await.unwrap();
    assert_eq!(hits.len(), 1);
    match &hits[0] {
        DbxEntry::File(f) => assert_eq!(f.id, "id:hit"),
        other => panic!("expected File, got {other:?}"),
    }
}

#[test]
fn validate_path_contract() {
    assert!(super::validate_path("").is_ok(), "empty is root");
    assert!(super::validate_path("/").is_ok());
    assert!(super::validate_path("/a/b").is_ok());
    let err = super::validate_path("relative").expect_err("relative must fail");
    match err {
        ArgoError::Schema(msg) => assert!(msg.contains("must start with '/'"), "got {msg:?}"),
        other => panic!("expected ArgoError::Schema, got {other:?}"),
    }
}
