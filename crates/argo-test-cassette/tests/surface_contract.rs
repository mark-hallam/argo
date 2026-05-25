//! Public-surface and compatibility contract for `argo-test-cassette`.
//!
//! These tests are intentionally offline. They pin the crate's exported
//! inventory, cassette JSON shape, redaction rules, replay default, and request
//! record construction without opening a socket.

use argo_test_cassette::{
    build_request_record, redact_body_json, redact_headers, Cassette, CassetteError,
    CassetteExchange, CassetteMode, CassetteRecorder, CassetteRequest, CassetteResponse, Result,
    DEFAULT_HEADER_ALLOWLIST, REDACTED_PLACEHOLDER,
};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

const EXPECTED_PUBLIC_MODULES: &[&str] = &["cassette", "recorder"];

const EXPECTED_CASSETTE_EXPORTS: &[&str] = &[
    "redact_body_json",
    "redact_headers",
    "Cassette",
    "CassetteError",
    "CassetteExchange",
    "CassetteRequest",
    "CassetteResponse",
    "Result",
    "DEFAULT_HEADER_ALLOWLIST",
    "REDACTED_PLACEHOLDER",
];

const EXPECTED_RECORDER_EXPORTS: &[&str] =
    &["build_request_record", "CassetteMode", "CassetteRecorder"];

#[test]
fn lib_rs_public_inventory_matches_snapshot() {
    let lib_rs = std::fs::read_to_string(crate_root().join("src/lib.rs"))
        .expect("read argo-test-cassette src/lib.rs");

    let public_modules = public_modules(&lib_rs);
    assert_eq!(
        public_modules,
        set(EXPECTED_PUBLIC_MODULES),
        "public module inventory drifted; update this audit snapshot only for \
         intentional API-surface changes"
    );

    assert_eq!(
        pub_use_group(&lib_rs, "cassette"),
        set(EXPECTED_CASSETTE_EXPORTS),
        "cassette re-export inventory drifted"
    );
    assert_eq!(
        pub_use_group(&lib_rs, "recorder"),
        set(EXPECTED_RECORDER_EXPORTS),
        "recorder re-export inventory drifted"
    );
}

#[test]
fn public_reexports_compile_at_expected_paths() {
    let path = std::env::temp_dir().join("surface-contract.cassette.json");
    let mut cassette = Cassette::new(&path);
    let request = CassetteRequest {
        method: "GET".into(),
        url: "https://api.example.test/v1/ping".into(),
        headers: BTreeMap::new(),
        body_sha256: String::new(),
        body_json_redacted: serde_json::Value::Null,
    };
    let response = CassetteResponse {
        status: 204,
        headers: BTreeMap::new(),
        body_json: serde_json::Value::Null,
    };
    cassette.append(CassetteExchange {
        request,
        response,
        recorded_at: "2026-05-24T00:00:00Z".into(),
    });

    let recorder = CassetteRecorder::new(cassette).with_mode(CassetteMode::Replay);
    assert_eq!(recorder.mode(), CassetteMode::Replay);
    let _typed_result: Result<()> = Ok(());
    let _typed_error = CassetteError::Exhausted(0);
}

#[test]
fn fixture_load_save_preserves_version_and_exchange_shape() {
    let source = crate_root().join("fixtures/sample.cassette.json");
    let tempdir = tempfile::tempdir().expect("tempdir");
    let copy = tempdir.path().join("sample.cassette.json");
    std::fs::copy(&source, &copy).expect("copy fixture");

    let cassette = Cassette::load(&copy).expect("load sample fixture");
    assert_eq!(cassette.version, "1");
    assert_eq!(cassette.len(), 1);
    assert_eq!(cassette.path(), copy.as_path());

    let exchange = cassette.exchanges.first().expect("one exchange");
    assert_eq!(exchange.request.method, "POST");
    assert_eq!(exchange.request.url, "https://api.example.test/v1/echo");
    assert_eq!(
        exchange
            .request
            .headers
            .get("authorization")
            .map(String::as_str),
        Some(REDACTED_PLACEHOLDER)
    );
    assert_eq!(
        exchange.request.body_json_redacted["api_key"],
        serde_json::json!(REDACTED_PLACEHOLDER)
    );
    assert_eq!(exchange.response.status, 200);
    assert_eq!(exchange.response.body_json["ok"], serde_json::json!(true));

    cassette.save().expect("save fixture copy");
    let saved: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&copy).expect("read saved fixture"))
            .expect("saved fixture remains json");
    assert_eq!(saved["version"], serde_json::json!("1"));
    assert_eq!(saved["exchanges"].as_array().map(Vec::len), Some(1));
    assert!(
        saved.get("cursor").is_none(),
        "runtime cursor must not serialize"
    );
    assert!(
        saved.get("path").is_none(),
        "runtime path must not serialize"
    );
}

#[test]
fn redaction_covers_sensitive_headers_and_nested_body_values() {
    let redacted_headers = redact_headers(
        &headers(&[
            ("authorization", "Bearer leaked"),
            ("x-api-key", "key-leaked"),
            ("x-auth-token", "token-leaked"),
            ("proxy-authorization", "proxy-leaked"),
            ("content-type", "application/json"),
            ("cookie", "session=leaked"),
        ]),
        DEFAULT_HEADER_ALLOWLIST,
    );

    for key in [
        "authorization",
        "x-api-key",
        "x-auth-token",
        "proxy-authorization",
    ] {
        assert_eq!(
            redacted_headers.get(key).map(String::as_str),
            Some(REDACTED_PLACEHOLDER),
            "{key} must be retained for shape but redacted"
        );
    }
    assert_eq!(
        redacted_headers.get("content-type").map(String::as_str),
        Some("application/json")
    );
    assert!(
        !redacted_headers.contains_key("cookie"),
        "non-allowlisted sensitive headers must be dropped"
    );

    let body = serde_json::json!({
        "access_token": "tok",
        "client_secret": "sec",
        "nested": [
            {"password": "pw", "public_value": "keep"},
            {"bearer": "bearer-value", "api_key": "api"}
        ]
    });
    let redacted = redact_body_json(body);
    assert_eq!(
        redacted["access_token"],
        serde_json::json!(REDACTED_PLACEHOLDER)
    );
    assert_eq!(
        redacted["client_secret"],
        serde_json::json!(REDACTED_PLACEHOLDER)
    );
    assert_eq!(
        redacted["nested"][0]["password"],
        serde_json::json!(REDACTED_PLACEHOLDER)
    );
    assert_eq!(
        redacted["nested"][0]["public_value"],
        serde_json::json!("keep")
    );
    assert_eq!(
        redacted["nested"][1]["bearer"],
        serde_json::json!(REDACTED_PLACEHOLDER)
    );
    assert_eq!(
        redacted["nested"][1]["api_key"],
        serde_json::json!(REDACTED_PLACEHOLDER)
    );
}

#[tokio::test]
async fn replay_default_serves_recorded_response_without_network() {
    assert_eq!(CassetteMode::parse(None), CassetteMode::Replay);

    let mut cassette = Cassette::new(std::env::temp_dir().join("offline-replay.cassette.json"));
    cassette.append(CassetteExchange {
        request: build_request_record(
            "GET",
            "https://offline.invalid/no-network",
            &BTreeMap::new(),
            b"",
        ),
        response: CassetteResponse {
            status: 202,
            headers: BTreeMap::new(),
            body_json: serde_json::json!({"offline": true}),
        },
        recorded_at: "2026-05-24T00:00:00Z".into(),
    });

    let mut recorder = CassetteRecorder::new(cassette).with_mode(CassetteMode::parse(None));
    let response = recorder
        .execute(
            "GET",
            "https://offline.invalid/no-network",
            BTreeMap::new(),
            Vec::new(),
        )
        .await
        .expect("replay mode returns recorded response");
    assert_eq!(response.status, 202);
    assert_eq!(response.body_json["offline"], serde_json::json!(true));
    assert!(recorder.cassette().is_exhausted());
}

#[test]
fn request_record_shape_pins_method_url_headers_body_hash_and_response_status() {
    let body = br#"{"api_key":"secret","payload":"ok"}"#;
    let request = build_request_record(
        "post",
        "https://api.example.test/v1/items?cursor=1",
        &headers(&[
            ("authorization", "Bearer secret"),
            ("content-type", "application/json"),
            ("x-debug", "drop-me"),
        ]),
        body,
    );

    assert_eq!(request.method, "POST");
    assert_eq!(request.url, "https://api.example.test/v1/items?cursor=1");
    assert_eq!(
        request.headers.get("authorization").map(String::as_str),
        Some(REDACTED_PLACEHOLDER)
    );
    assert_eq!(
        request.headers.get("content-type").map(String::as_str),
        Some("application/json")
    );
    assert!(!request.headers.contains_key("x-debug"));
    assert_eq!(
        request.body_sha256,
        "93ce7e8b00a0c77e9ebd0ae7f60bd66c724c279a2451bf38b72f9d659b34136e"
    );
    assert_eq!(
        request.body_json_redacted["api_key"],
        serde_json::json!(REDACTED_PLACEHOLDER)
    );
    assert_eq!(
        request.body_json_redacted["payload"],
        serde_json::json!("ok")
    );

    let exchange = CassetteExchange {
        request,
        response: CassetteResponse {
            status: 201,
            headers: headers(&[("content-type", "application/json")]),
            body_json: serde_json::json!({"created": true}),
        },
        recorded_at: "2026-05-24T00:00:00Z".into(),
    };
    assert_eq!(exchange.response.status, 201);
    assert_eq!(
        exchange.response.body_json["created"],
        serde_json::json!(true)
    );
}

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn public_modules(contents: &str) -> BTreeSet<String> {
    contents
        .lines()
        .map(str::trim)
        .filter_map(|line| line.strip_prefix("pub mod "))
        .filter_map(|rest| rest.strip_suffix(';'))
        .map(str::to_string)
        .collect()
}

fn pub_use_group(contents: &str, module: &str) -> BTreeSet<String> {
    let needle = format!("pub use {module}::{{");
    let start = contents
        .find(&needle)
        .unwrap_or_else(|| panic!("missing `{needle}` block"));
    let after_start = start + needle.len();
    let after = &contents[after_start..];
    let end = after
        .find("};")
        .unwrap_or_else(|| panic!("missing end of `{needle}` block"));
    after[..end]
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(str::to_string)
        .collect()
}

fn set(items: &[&str]) -> BTreeSet<String> {
    items.iter().map(|item| (*item).to_string()).collect()
}

fn headers(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
        .collect()
}
