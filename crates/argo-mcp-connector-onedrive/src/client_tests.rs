//! Wiremock-backed unit tests for `OneDriveConnector`.
//!
//! Eight scenarios:
//!  1. `list_folder` against root + drops items lacking both `file` and
//!     `folder` facets.
//!  2. `upload_small_file` PUTs `/content` with Bearer auth + binary body
//!     and decodes the returned DriveItem.
//!  3. `upload_small_file` rejects a 4 MiB payload BEFORE any network call.
//!  4. `upload_large_file` walks a three-chunk resumable session
//!     (`createUploadSession` → 202, 202, 200-final) via `with_chunk_bytes`.
//!  5. `upload_large_file` recovers from a mid-session 5xx by GETting the
//!     session URL, parsing `nextExpectedRanges`, and resuming.
//!  6. `download_file` round-trips body bytes verbatim AND surfaces a 401
//!     as `ArgoError::Provider` mentioning "refresh".
//!  7. `create_share_link` serializes each (`LinkType` × `LinkScope`) pair
//!     correctly and decodes the `link.webUrl` field.
//!  8. `search` posts the query string to `/search(q='…')` and decodes the
//!     typed DriveItem array.
//!
//! A ninth plain-unit test locks down `encode_item_id` for shared-drive
//! composite ids without a network mock.

use super::*;
use crate::types::DriveItem;
use argo_mcp_connector_shared::RetryPolicy;
use serde_json::json;
use wiremock::matchers::{body_bytes, body_json_string, header, method, path};
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

fn connector(server: &MockServer) -> OneDriveConnector {
    OneDriveConnector::new("seed-token".into())
        .with_base(server.uri())
        .with_retry_policy(no_retry())
}

fn file_item(id: &str, name: &str, size: u64) -> serde_json::Value {
    json!({
        "id": id, "name": name, "size": size,
        "lastModifiedDateTime": "2026-05-11T12:00:00Z",
        "file": { "mimeType": "text/plain" }
    })
}

async fn mock(server: &MockServer, m: &str, p: &str, resp: ResponseTemplate) {
    Mock::given(method(m)).and(path(p)).respond_with(resp).expect(1).mount(server).await;
}

#[tokio::test]
async fn list_folder_root_drops_items_without_facets() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/me/drive/root/children"))
        .and(header("authorization", "Bearer seed-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "value": [
                file_item("01ABC", "receipt.txt", 42),
                {"id": "01FLD", "name": "inbox", "folder": {"childCount": 3}},
                {"id": "01OTH", "name": "package.pkg"}
            ]
        })))
        .expect(1)
        .mount(&server)
        .await;

    let entries = connector(&server).list_folder(None).await.unwrap();
    assert_eq!(entries.len(), 2, "facet-less item must be dropped");
    match &entries[0] {
        DriveItem::File(f) => {
            assert_eq!(f.id, "01ABC");
            assert_eq!(f.size_bytes, 42);
            assert_eq!(f.mime_type.as_deref(), Some("text/plain"));
        }
        other => panic!("expected File, got {other:?}"),
    }
    match &entries[1] {
        DriveItem::Folder(f) => {
            assert_eq!(f.id, "01FLD");
            assert_eq!(f.child_count, 3);
        }
        other => panic!("expected Folder, got {other:?}"),
    }
}

#[tokio::test]
async fn upload_small_file_puts_content_with_bearer_and_body() {
    let server = MockServer::start().await;
    Mock::given(method("PUT"))
        .and(path("/me/drive/items/root:/hello.txt:/content"))
        .and(header("authorization", "Bearer seed-token"))
        .and(header("content-type", "application/octet-stream"))
        .and(body_bytes(b"hello".to_vec()))
        .respond_with(ResponseTemplate::new(201).set_body_json(file_item("01NEW", "hello.txt", 5)))
        .expect(1)
        .mount(&server)
        .await;

    let item = connector(&server)
        .upload_small_file("root", "hello.txt", b"hello")
        .await
        .unwrap();
    match item {
        DriveItem::File(f) => {
            assert_eq!(f.id, "01NEW");
            assert_eq!(f.size_bytes, 5);
        }
        other => panic!("expected File, got {other:?}"),
    }
}

#[tokio::test]
async fn upload_small_file_rejects_oversized_without_network_call() {
    let server = MockServer::start().await;
    let oversized = vec![0u8; SIMPLE_UPLOAD_CAP];
    let err = connector(&server)
        .upload_small_file("root", "big.bin", &oversized)
        .await
        .expect_err("oversized small upload must be rejected");
    match err {
        ArgoError::Provider(msg) => {
            assert!(msg.contains("4 MiB"), "must mention cap, got {msg:?}");
            assert!(
                msg.contains("upload_large_file"),
                "must redirect to resumable path, got {msg:?}"
            );
        }
        other => panic!("expected ArgoError::Provider, got {other:?}"),
    }
}

#[tokio::test]
async fn upload_large_file_resumable_three_chunk_session_success() {
    let server = MockServer::start().await;
    let session = "/upload-sessions/abc";
    mock(
        &server, "POST", "/me/drive/items/root:/big.bin:/createUploadSession",
        ResponseTemplate::new(200).set_body_json(json!({
            "uploadUrl": format!("{}{session}", server.uri())
        })),
    ).await;
    chunk_mock(&server, session, "bytes 0-1/5", 202, json!({"nextExpectedRanges": ["2-"]})).await;
    chunk_mock(&server, session, "bytes 2-3/5", 202, json!({"nextExpectedRanges": ["4-"]})).await;
    chunk_mock(
        &server, session, "bytes 4-4/5", 200,
        file_item("01BIG", "big.bin", 5),
    ).await;

    let item = connector(&server)
        .with_chunk_bytes(2)
        .upload_large_file("root", "big.bin", &[1, 2, 3, 4, 5])
        .await
        .unwrap();
    match item {
        DriveItem::File(f) => assert_eq!(f.id, "01BIG"),
        other => panic!("expected File, got {other:?}"),
    }
}

#[tokio::test]
async fn upload_large_file_resumes_after_connection_drop() {
    let server = MockServer::start().await;
    let session = "/upload-sessions/drop";
    mock(
        &server, "POST", "/me/drive/items/root:/big.bin:/createUploadSession",
        ResponseTemplate::new(200).set_body_json(json!({
            "uploadUrl": format!("{}{session}", server.uri())
        })),
    ).await;
    // Chunk 1 fails 500 → connector falls through to GET status.
    chunk_mock(&server, session, "bytes 0-1/5", 500, json!({})).await;
    mock(
        &server, "GET", session,
        ResponseTemplate::new(200).set_body_json(json!({"nextExpectedRanges": ["2-"]})),
    ).await;
    chunk_mock(&server, session, "bytes 2-3/5", 202, json!({"nextExpectedRanges": ["4-"]})).await;
    chunk_mock(
        &server, session, "bytes 4-4/5", 201,
        file_item("01RES", "big.bin", 5),
    ).await;

    let item = connector(&server)
        .with_chunk_bytes(2)
        .upload_large_file("root", "big.bin", &[1, 2, 3, 4, 5])
        .await
        .unwrap();
    match item {
        DriveItem::File(f) => assert_eq!(f.id, "01RES"),
        other => panic!("expected File, got {other:?}"),
    }
}

async fn chunk_mock(
    server: &MockServer,
    p: &str,
    range: &str,
    status: u16,
    body: serde_json::Value,
) {
    Mock::given(method("PUT"))
        .and(path(p))
        .and(header("content-range", range))
        .respond_with(ResponseTemplate::new(status).set_body_json(body))
        .expect(1)
        .mount(server)
        .await;
}

#[tokio::test]
async fn download_file_returns_body_verbatim_and_401_carries_refresh_hint() {
    let server = MockServer::start().await;
    let payload: &[u8] = b"line one\n\x00\xffbinary";
    Mock::given(method("GET"))
        .and(path("/me/drive/items/abc/content"))
        .and(header("authorization", "Bearer seed-token"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(payload))
        .expect(1)
        .mount(&server)
        .await;
    mock(
        &server, "GET", "/me/drive/items/expired/content",
        ResponseTemplate::new(401).set_body_string("InvalidAuthenticationToken"),
    ).await;

    let od = connector(&server);
    let bytes = od.download_file("abc").await.unwrap();
    assert_eq!(bytes, payload, "download body must round-trip byte-identical");

    let err = od.download_file("expired").await.expect_err("401 must surface");
    match err {
        ArgoError::Provider(msg) => {
            assert!(msg.contains("401"), "must mention status, got {msg:?}");
            assert!(msg.contains("refresh"), "must hint at refresh, got {msg:?}");
        }
        other => panic!("expected ArgoError::Provider, got {other:?}"),
    }
}

#[tokio::test]
async fn create_share_link_serializes_each_type_and_scope() {
    let server = MockServer::start().await;
    let cases = [
        ("v1", "view", "anonymous", "https://1drv.ms/v/view-anon"),
        ("e1", "edit", "organization", "https://1drv.ms/e/edit-org"),
        ("b1", "embed", "users", "https://1drv.ms/b/embed-users"),
    ];
    for (id, t, s, url) in cases {
        Mock::given(method("POST"))
            .and(path(format!("/me/drive/items/{id}/createLink")))
            .and(body_json_string(format!(r#"{{"type":"{t}","scope":"{s}"}}"#)))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "link": { "webUrl": url }
            })))
            .expect(1)
            .mount(&server)
            .await;
    }

    let od = connector(&server);
    let want_v = od.create_share_link("v1", LinkType::View, LinkScope::Anonymous).await.unwrap();
    assert_eq!(want_v, "https://1drv.ms/v/view-anon");
    let want_e = od.create_share_link("e1", LinkType::Edit, LinkScope::Organization).await.unwrap();
    assert_eq!(want_e, "https://1drv.ms/e/edit-org");
    let want_b = od.create_share_link("b1", LinkType::Embed, LinkScope::Users).await.unwrap();
    assert_eq!(want_b, "https://1drv.ms/b/embed-users");
}

#[tokio::test]
async fn search_returns_typed_drive_items() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/me/drive/root/search(q='receipt')"))
        .and(header("authorization", "Bearer seed-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "value": [file_item("01HIT", "receipt-jan.txt", 7)]
        })))
        .expect(1)
        .mount(&server)
        .await;

    let hits = connector(&server).search("receipt").await.unwrap();
    assert_eq!(hits.len(), 1);
    match &hits[0] {
        DriveItem::File(f) => {
            assert_eq!(f.id, "01HIT");
            assert_eq!(f.name, "receipt-jan.txt");
        }
        other => panic!("expected File, got {other:?}"),
    }
}

#[test]
fn encode_item_id_routes_shared_drive_composites() {
    let od = OneDriveConnector::new("t".into());
    assert_eq!(od.encode_item_id("plain"), "/me/drive/items/plain");
    assert_eq!(
        od.encode_item_id("driveX!itemY"),
        "/drives/driveX/items/driveX!itemY"
    );
    assert_eq!(od.encode_item_id("trailing!"), "/me/drive/items/trailing!");
}
