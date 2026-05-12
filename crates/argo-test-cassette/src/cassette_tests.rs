use super::*;
use std::collections::BTreeMap;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn temp_path(name: &str) -> std::path::PathBuf {
    let dir = tempfile::tempdir().unwrap().keep();
    dir.join(name)
}

fn headers(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs.iter().map(|(k, v)| ((*k).to_string(), (*v).to_string())).collect()
}

#[tokio::test]
async fn round_trip_record_then_replay() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/x"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"ok": true})))
        .mount(&server)
        .await;

    let path_buf = temp_path("rt.cassette.json");
    let cassette = Cassette::new(&path_buf);
    let mut rec = CassetteRecorder::new(cassette).with_mode(CassetteMode::Record);
    let url = format!("{}/v1/x", server.uri());
    let body = br#"{"hello":"world","api_key":"shh"}"#.to_vec();
    let recorded = rec
        .execute("POST", &url, headers(&[("authorization", "Bearer abc")]), body.clone())
        .await
        .unwrap();
    assert_eq!(recorded.status, 200);
    drop(rec);

    let loaded = Cassette::load(&path_buf).unwrap();
    assert_eq!(loaded.len(), 1);
    let mut rec2 = CassetteRecorder::new(loaded).with_mode(CassetteMode::Replay);
    let replayed = rec2
        .execute("POST", &url, headers(&[("authorization", "Bearer abc")]), body)
        .await
        .unwrap();
    assert_eq!(replayed.body_json, serde_json::json!({"ok": true}));
}

#[tokio::test]
async fn sequential_match_order_enforced() {
    let mut c = Cassette::new(temp_path("order.cassette.json"));
    for tag in ["A", "B"] {
        c.append(CassetteExchange {
            request: build_request_record("GET", &format!("https://x/{tag}"), &BTreeMap::new(), b""),
            response: CassetteResponse { status: 200, headers: BTreeMap::new(), body_json: serde_json::json!(tag) },
            recorded_at: "t".into(),
        });
    }
    let mut rec = CassetteRecorder::new(c).with_mode(CassetteMode::Replay);
    // Asking for B first when A is at the cursor must error.
    let err = rec
        .execute("GET", "https://x/B", BTreeMap::new(), Vec::new())
        .await
        .unwrap_err();
    assert!(matches!(err, CassetteError::Mismatch { .. }));
}

#[tokio::test]
async fn mismatch_on_replay_errors() {
    let mut c = Cassette::new(temp_path("mismatch.cassette.json"));
    c.append(CassetteExchange {
        request: build_request_record("POST", "https://x/foo", &BTreeMap::new(), b""),
        response: CassetteResponse { status: 200, headers: BTreeMap::new(), body_json: serde_json::Value::Null },
        recorded_at: "t".into(),
    });
    let mut rec = CassetteRecorder::new(c).with_mode(CassetteMode::Replay);
    let err = rec
        .execute("GET", "https://x/bar", BTreeMap::new(), Vec::new())
        .await
        .unwrap_err();
    match err {
        CassetteError::Mismatch { expected_method, actual_method, .. } => {
            assert_eq!(expected_method, "POST");
            assert_eq!(actual_method, "GET");
        }
        other => panic!("expected Mismatch, got {other:?}"),
    }
}

#[test]
fn env_driven_mode_selection() {
    assert_eq!(CassetteMode::parse(Some("record")), CassetteMode::Record);
    assert_eq!(CassetteMode::parse(Some("REPLAY")), CassetteMode::Replay);
    assert_eq!(CassetteMode::parse(Some("passthrough")), CassetteMode::Passthrough);
    assert_eq!(CassetteMode::parse(None), CassetteMode::Replay);
    assert_eq!(CassetteMode::parse(Some("garbage")), CassetteMode::Replay);
}

#[test]
fn secret_redaction_in_request_record() {
    let hdrs = headers(&[
        ("authorization", "Bearer leaked"),
        ("content-type", "application/json"),
    ]);
    let body = br#"{"api_key":"S","password":"P","model":"opus","nested":{"bearer_token":"Z"}}"#;
    let req = build_request_record("POST", "https://x/", &hdrs, body);
    assert_eq!(req.headers.get("authorization").map(String::as_str), Some(REDACTED_PLACEHOLDER));
    assert_eq!(req.headers.get("content-type").map(String::as_str), Some("application/json"));
    let body_v = &req.body_json_redacted;
    assert_eq!(body_v.get("api_key"), Some(&serde_json::json!(REDACTED_PLACEHOLDER)));
    assert_eq!(body_v.get("password"), Some(&serde_json::json!(REDACTED_PLACEHOLDER)));
    assert_eq!(body_v.get("model"), Some(&serde_json::json!("opus")));
    let nested = body_v.get("nested").unwrap();
    assert_eq!(nested.get("bearer_token"), Some(&serde_json::json!(REDACTED_PLACEHOLDER)));
    assert!(!req.body_sha256.is_empty());
}

#[test]
fn header_allowlist_filter_drops_unlisted() {
    let hdrs = headers(&[
        ("content-type", "application/json"),
        ("x-trace-id", "abc-123"),
        ("cookie", "session=xyz"),
        ("authorization", "Bearer T"),
    ]);
    let filtered = redact_headers(&hdrs, DEFAULT_HEADER_ALLOWLIST);
    assert!(filtered.contains_key("content-type"));
    assert_eq!(filtered.get("authorization").map(String::as_str), Some(REDACTED_PLACEHOLDER));
    assert!(!filtered.contains_key("x-trace-id"), "non-allowlisted header should be dropped");
    assert!(!filtered.contains_key("cookie"), "cookie should be dropped");
}
