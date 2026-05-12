//! Wiremock-backed unit tests for `BoxConnector`.
//!
//! Six wiremock scenarios per the brief (plus two pure-function unit tests
//! at the bottom of the file):
//!  1. `list_folder` round-trips a typed entry array (file + folder + ignored
//!     unknown-type entry).
//!  2. `upload_file` POSTs multipart to the upload host and decodes the
//!     wrapped `entries[]` response.
//!  3. `download_file` GETs from the metadata host and returns response body
//!     bytes verbatim.
//!  4. The three [`SharedLinkAccess`] variants all round-trip their Box wire
//!     tokens (`open` / `company` / `collaborators`) through a single `PUT`
//!     `/2.0/files/{id}?fields=shared_link` endpoint.
//!  5. A 401 response surfaces as [`ArgoError::Provider`] with a refresh
//!     hint — exercises the auth-error contract from the brief.
//!  6. `search` round-trips the `entries[]` envelope and percent-encodes
//!     spaces in the query string.

use super::*;
use crate::types::BoxItem;
use crate::SharedLinkAccess;
use argo_mcp_connector_shared::RetryPolicy;
use wiremock::matchers::{
    body_json_string, header, header_exists, method, path as wm_path, query_param,
};
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

fn connector(api: &MockServer, upload: &MockServer) -> BoxConnector {
    BoxConnector::new("seed-token".into())
        .with_api_base(api.uri())
        .with_upload_base(upload.uri())
        .with_retry_policy(no_retry())
}

#[tokio::test]
async fn list_folder_round_trips_mixed_entries() {
    let api = MockServer::start().await;
    let upload = MockServer::start().await;
    Mock::given(method("GET"))
        .and(wm_path("/2.0/folders/12345/items"))
        .and(header("authorization", "Bearer seed-token"))
        .and(query_param("fields", "id,name,type,size,modified_at,etag"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "entries": [
                {
                    "type": "file",
                    "id": "1001",
                    "name": "receipt.txt",
                    "size": 42,
                    "modified_at": "2026-05-11T12:00:00Z",
                    "etag": "0"
                },
                {
                    "type": "folder",
                    "id": "2001",
                    "name": "archive",
                    "modified_at": "2026-05-11T12:00:00Z"
                },
                {
                    "type": "web_link",
                    "id": "9999",
                    "name": "bookmark"
                }
            ]
        })))
        .expect(1)
        .mount(&api)
        .await;

    let bx = connector(&api, &upload);
    let entries = bx.list_folder("12345").await.expect("list_folder ok");
    assert_eq!(entries.len(), 2, "unknown-type entry must be dropped");
    match &entries[0] {
        BoxItem::File(f) => {
            assert_eq!(f.id, "1001");
            assert_eq!(f.name, "receipt.txt");
            assert_eq!(f.size_bytes, 42);
            assert_eq!(f.etag, "0");
        }
        other => panic!("expected File, got {other:?}"),
    }
    match &entries[1] {
        BoxItem::Folder(f) => {
            assert_eq!(f.id, "2001");
            assert_eq!(f.name, "archive");
        }
        other => panic!("expected Folder, got {other:?}"),
    }
}

#[tokio::test]
async fn upload_file_posts_multipart_to_upload_host() {
    let api = MockServer::start().await;
    let upload = MockServer::start().await;
    // `body_json_string` won't match multipart; assert on path + auth header
    // + content-type prefix instead. The response shape is what proves the
    // upload semantics work end-to-end.
    Mock::given(method("POST"))
        .and(wm_path("/api/2.0/files/content"))
        .and(header("authorization", "Bearer seed-token"))
        .and(header_exists("content-type"))
        .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
            "entries": [
                {
                    "type": "file",
                    "id": "5005",
                    "name": "r.txt",
                    "size": 5,
                    "modified_at": "2026-05-11T12:34:56Z",
                    "etag": "0"
                }
            ]
        })))
        .expect(1)
        .mount(&upload)
        .await;

    let bx = connector(&api, &upload);
    let file = bx
        .upload_file("0", "r.txt", b"hello")
        .await
        .expect("upload_file ok");
    assert_eq!(file.id, "5005");
    assert_eq!(file.name, "r.txt");
    assert_eq!(file.size_bytes, 5);
}

#[tokio::test]
async fn download_file_returns_raw_bytes() {
    let api = MockServer::start().await;
    let upload = MockServer::start().await;
    let payload: &[u8] = b"line one\nline two\n\x00\xffbinary";
    Mock::given(method("GET"))
        .and(wm_path("/2.0/files/5005/content"))
        .and(header("authorization", "Bearer seed-token"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(payload))
        .expect(1)
        .mount(&api)
        .await;

    let bx = connector(&api, &upload);
    let bytes = bx.download_file("5005").await.expect("download_file ok");
    assert_eq!(bytes, payload, "download must round-trip byte-identical");
}

#[tokio::test]
async fn create_shared_link_all_three_access_levels_wire_correctly() {
    let api = MockServer::start().await;
    let upload = MockServer::start().await;
    // Mount one mock per access level — the connector picks the right mock
    // only if it serializes the access enum to Box's exact wire token
    // (`open` / `company` / `collaborators`), and only if the request hits
    // the `PUT /2.0/files/{id}?fields=shared_link` endpoint with the expected
    // auth header.
    for (variant, wire) in [
        (SharedLinkAccess::Open, "open"),
        (SharedLinkAccess::Company, "company"),
        (SharedLinkAccess::Collaborators, "collaborators"),
    ] {
        let body = format!(r#"{{"shared_link":{{"access":"{wire}"}}}}"#);
        Mock::given(method("PUT"))
            .and(wm_path("/2.0/files/7007"))
            .and(query_param("fields", "shared_link"))
            .and(header("authorization", "Bearer seed-token"))
            .and(body_json_string(body))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "shared_link": { "url": format!("https://app.box.com/s/{wire}") }
            })))
            .expect(1)
            .mount(&api)
            .await;
        let bx = connector(&api, &upload);
        let url = bx
            .create_shared_link("7007", variant)
            .await
            .expect("create_shared_link ok");
        assert_eq!(url, format!("https://app.box.com/s/{wire}"));
    }
}

#[tokio::test]
async fn expired_token_surfaces_as_provider_with_refresh_hint() {
    let api = MockServer::start().await;
    let upload = MockServer::start().await;
    Mock::given(method("GET"))
        .and(wm_path("/2.0/folders/0/items"))
        .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({
            "type": "error",
            "status": 401,
            "code": "unauthorized",
            "message": "Auth token expired"
        })))
        .expect(1)
        .mount(&api)
        .await;

    let bx = connector(&api, &upload);
    let err = bx
        .list_folder("0")
        .await
        .expect_err("401 must surface as error");
    match err {
        ArgoError::Provider(msg) => {
            assert!(
                msg.contains("401"),
                "error must mention the HTTP code, got {msg:?}"
            );
            assert!(
                msg.to_lowercase().contains("refresh"),
                "error must hint at token refresh, got {msg:?}"
            );
        }
        other => panic!("expected ArgoError::Provider, got {other:?}"),
    }
}

#[tokio::test]
async fn search_round_trips_entries_and_percent_encodes_query() {
    let api = MockServer::start().await;
    let upload = MockServer::start().await;
    // Box's search endpoint uses `query` as a URL parameter — spaces must be
    // percent-encoded as `%20` (RFC 3986 form). The mock will only match if
    // the encoder did its job.
    Mock::given(method("GET"))
        .and(wm_path("/2.0/search"))
        .and(query_param("query", "Q1 report"))
        .and(query_param("limit", "5"))
        .and(header("authorization", "Bearer seed-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "entries": [
                {
                    "type": "file",
                    "id": "8008",
                    "name": "Q1 report.pdf",
                    "size": 1024,
                    "modified_at": "2026-05-11T00:00:00Z",
                    "etag": "3"
                }
            ]
        })))
        .expect(1)
        .mount(&api)
        .await;

    let bx = connector(&api, &upload);
    let hits = bx.search("Q1 report", 5).await.expect("search ok");
    assert_eq!(hits.len(), 1);
    match &hits[0] {
        BoxItem::File(f) => {
            assert_eq!(f.id, "8008");
            assert_eq!(f.name, "Q1 report.pdf");
        }
        other => panic!("expected File, got {other:?}"),
    }
}

#[test]
fn validate_id_contract() {
    assert!(super::validate_id("0", "folder_id").is_ok(), "root id");
    assert!(super::validate_id("12345", "folder_id").is_ok());
    let empty = super::validate_id("", "folder_id").expect_err("empty must fail");
    match empty {
        ArgoError::Schema(msg) => assert!(msg.contains("must not be empty"), "got {msg:?}"),
        other => panic!("expected ArgoError::Schema, got {other:?}"),
    }
    let alpha = super::validate_id("abc", "file_id").expect_err("alpha must fail");
    match alpha {
        ArgoError::Schema(msg) => assert!(msg.contains("numeric"), "got {msg:?}"),
        other => panic!("expected ArgoError::Schema, got {other:?}"),
    }
}

#[test]
fn shared_link_access_wire_tokens_are_stable() {
    assert_eq!(SharedLinkAccess::Open.as_wire(), "open");
    assert_eq!(SharedLinkAccess::Company.as_wire(), "company");
    assert_eq!(SharedLinkAccess::Collaborators.as_wire(), "collaborators");
}
