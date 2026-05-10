//! Unit tests for the chunked list-streaming helpers in
//! `streaming.rs`. The HTTP-write helpers are tested by piping
//! into a `Vec<u8>` (via `tokio::io::AsyncWrite` — which the
//! production helpers are now generic over) and asserting on the
//! exact byte stream the listener loop would emit.

use super::{
    is_streaming_list_request, pick_wire_fmt, stream_json_array, stream_ndjson,
    streaming_list_kind, ListKind, WireFmt,
};
use argo_types::ArgoResult;
use futures::stream;

fn req(line: &str) -> String {
    format!("{line}\r\nHost: localhost\r\n\r\n")
}

fn req_with_accept(line: &str, accept: &str) -> String {
    format!("{line}\r\nHost: localhost\r\nAccept: {accept}\r\n\r\n")
}

#[test]
fn list_kind_matches_three_target_routes() {
    assert!(matches!(
        streaming_list_kind(&req("GET /sessions HTTP/1.1")),
        Some(ListKind::Sessions)
    ));
    assert!(matches!(
        streaming_list_kind(&req("GET /sessions?limit=10 HTTP/1.1")),
        Some(ListKind::Sessions)
    ));
    assert!(matches!(
        streaming_list_kind(&req("GET /audit/recent HTTP/1.1")),
        Some(ListKind::AuditRecent)
    ));
    assert!(matches!(
        streaming_list_kind(&req(
            "GET /sessions/00000000-0000-0000-0000-000000000000/messages HTTP/1.1"
        )),
        Some(ListKind::Messages)
    ));
}

#[test]
fn list_kind_skips_unrelated_paths() {
    // Bare /sessions/<id> is not a list — skip it.
    assert!(streaming_list_kind(&req(
        "GET /sessions/00000000-0000-0000-0000-000000000000 HTTP/1.1"
    ))
    .is_none());
    // /audit (chain summary, small response) is intentionally NOT streamed.
    assert!(streaming_list_kind(&req("GET /audit HTTP/1.1")).is_none());
    // /sessions/<id>/messages/count must not collide with /messages.
    assert!(streaming_list_kind(&req(
        "GET /sessions/00000000-0000-0000-0000-000000000000/messages/count HTTP/1.1"
    ))
    .is_none());
    // Other GETs untouched.
    assert!(streaming_list_kind(&req("GET /health HTTP/1.1")).is_none());
}

#[test]
fn streaming_negotiated_only_when_caller_opts_in() {
    // Default: a small bare /sessions request is NOT streamed.
    assert!(!is_streaming_list_request(&req("GET /sessions HTTP/1.1")));
    // Explicit limit > 1k → stream.
    assert!(is_streaming_list_request(&req(
        "GET /sessions?limit=5000 HTTP/1.1"
    )));
    // ?stream=true → stream.
    assert!(is_streaming_list_request(&req(
        "GET /sessions?stream=true HTTP/1.1"
    )));
    // Accept: application/x-ndjson → stream.
    assert!(is_streaming_list_request(&req_with_accept(
        "GET /sessions HTTP/1.1",
        "application/x-ndjson"
    )));
    // Accept-only with non-streaming path → no.
    assert!(!is_streaming_list_request(&req_with_accept(
        "GET /health HTTP/1.1",
        "application/x-ndjson"
    )));
}

#[test]
fn pick_wire_fmt_honors_accept_header() {
    assert!(matches!(
        pick_wire_fmt(&req_with_accept(
            "GET /sessions HTTP/1.1",
            "application/x-ndjson"
        )),
        WireFmt::Ndjson
    ));
    assert!(matches!(
        pick_wire_fmt(&req("GET /sessions HTTP/1.1")),
        WireFmt::JsonArray
    ));
    // application/json explicit → JSON array.
    assert!(matches!(
        pick_wire_fmt(&req_with_accept(
            "GET /sessions HTTP/1.1",
            "application/json"
        )),
        WireFmt::JsonArray
    ));
}

#[tokio::test]
async fn json_array_empty_stream_emits_brackets() {
    let mut buf: Vec<u8> = Vec::new();
    let s = stream::iter(Vec::<ArgoResult<serde_json::Value>>::new());
    stream_json_array(&mut buf, s).await.unwrap();
    let text = String::from_utf8(buf).unwrap();
    assert!(text.starts_with("HTTP/1.1 200 OK\r\n"), "head: {text}");
    assert!(text.contains("Transfer-Encoding: chunked"));
    let body = decode_chunked_body(&text);
    assert_eq!(body, "[]\n");
}

#[tokio::test]
async fn json_array_multi_row_stream_is_valid_json() {
    let rows: Vec<ArgoResult<serde_json::Value>> = (0..3)
        .map(|i| Ok(serde_json::json!({"i": i})))
        .collect();
    let mut buf: Vec<u8> = Vec::new();
    stream_json_array(&mut buf, stream::iter(rows)).await.unwrap();
    let body = decode_chunked_body(&String::from_utf8(buf).unwrap());
    let parsed: serde_json::Value = serde_json::from_str(&body).expect("valid json");
    let arr = parsed.as_array().unwrap();
    assert_eq!(arr.len(), 3);
    assert_eq!(arr[0]["i"], 0);
    assert_eq!(arr[2]["i"], 2);
}

#[tokio::test]
async fn ndjson_one_value_per_line() {
    let rows: Vec<ArgoResult<serde_json::Value>> = (0..3)
        .map(|i| Ok(serde_json::json!({"i": i})))
        .collect();
    let mut buf: Vec<u8> = Vec::new();
    stream_ndjson(&mut buf, stream::iter(rows)).await.unwrap();
    let text = String::from_utf8(buf).unwrap();
    assert!(text.contains("Content-Type: application/x-ndjson"));
    let body = decode_chunked_body(&text);
    let lines: Vec<&str> = body.trim_end_matches('\n').split('\n').collect();
    assert_eq!(lines.len(), 3);
    for (i, l) in lines.iter().enumerate() {
        let v: serde_json::Value = serde_json::from_str(l).expect("valid json");
        assert_eq!(v["i"], i as i64);
    }
}

#[tokio::test]
async fn ndjson_empty_stream_emits_only_terminator() {
    let mut buf: Vec<u8> = Vec::new();
    let s = stream::iter(Vec::<ArgoResult<serde_json::Value>>::new());
    stream_ndjson(&mut buf, s).await.unwrap();
    let body = decode_chunked_body(&String::from_utf8(buf).unwrap());
    assert!(body.is_empty(), "ndjson empty body expected, got: {body:?}");
}

#[tokio::test]
async fn json_array_propagates_stream_error() {
    let rows: Vec<ArgoResult<serde_json::Value>> = vec![
        Ok(serde_json::json!({"i": 0})),
        Err(argo_types::ArgoError::Provider("boom".to_string())),
    ];
    let mut buf: Vec<u8> = Vec::new();
    let r = stream_json_array(&mut buf, stream::iter(rows)).await;
    assert!(r.is_err(), "expected mid-stream error to surface as io::Error");
}

/// Decode the body of an HTTP/1.1 `Transfer-Encoding: chunked`
/// response. Test-only — the production listener writes chunks but
/// doesn't read them back. Returns the de-chunked body string.
fn decode_chunked_body(raw: &str) -> String {
    let body = raw.split_once("\r\n\r\n").map(|(_, b)| b).unwrap_or("");
    let mut out = String::new();
    let mut rest = body;
    while let Some((size_line, after)) = rest.split_once("\r\n") {
        let size = usize::from_str_radix(size_line.trim(), 16).unwrap_or(0);
        if size == 0 {
            break;
        }
        out.push_str(&after[..size]);
        rest = &after[size + 2..];
    }
    out
}
