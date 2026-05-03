//! Gateway integration tests for `GET /audit/by-vertical` (#688).
//! The CLI counterpart is unit-tested in
//! `crates/argo-cli/src/handlers/audit_by_vertical_tests.rs` —
//! these tests prove the HTTP wiring (dispatch, JSON shape,
//! markdown variant, catalog entry).

mod common;

use common::{boot, body_of, http_get, pick_free_port};

#[test]
fn audit_by_vertical_returns_empty_object_on_clean_db() {
    let port = pick_free_port();
    let (mut child, _stderr) = boot(port, 2);

    let resp = http_get(port, "/audit/by-vertical");
    assert!(resp.contains("HTTP/1.1 200 OK"), "resp: {resp}");
    let v: serde_json::Value = serde_json::from_str(body_of(&resp)).unwrap();
    let by_vertical = v["by_vertical"].as_object().expect("by_vertical object");
    assert!(
        by_vertical.is_empty(),
        "clean DB must yield empty histogram; got {by_vertical:?}"
    );

    // ?since-minutes=… honours the same shape.
    let resp = http_get(port, "/audit/by-vertical?since-minutes=60");
    assert!(resp.contains("HTTP/1.1 200 OK"), "resp: {resp}");
    let v: serde_json::Value = serde_json::from_str(body_of(&resp)).unwrap();
    assert!(v["by_vertical"].as_object().unwrap().is_empty());

    let _ = child.kill();
}

#[test]
fn audit_by_vertical_markdown_variant_returns_a_headline() {
    let port = pick_free_port();
    let (mut child, _stderr) = boot(port, 1);

    let resp = http_get(port, "/audit/by-vertical?format=markdown");
    assert!(resp.contains("HTTP/1.1 200 OK"), "resp: {resp}");
    let body = body_of(&resp);
    assert!(
        body.starts_with("# Audit by vertical"),
        "expected markdown headline; body: {body}"
    );
    // Empty histogram => no table, just the headline.
    assert!(
        !body.contains("| vertical | count |"),
        "empty histogram must not render a table; body: {body}"
    );

    let _ = child.kill();
}

#[test]
fn routes_catalog_lists_audit_by_vertical() {
    let port = pick_free_port();
    let (mut child, _stderr) = boot(port, 1);

    let resp = http_get(port, "/routes");
    assert!(resp.contains("HTTP/1.1 200 OK"), "resp: {resp}");
    let body = body_of(&resp);
    assert!(
        body.contains("/audit/by-vertical"),
        "/routes must list /audit/by-vertical; body: {body}"
    );

    let _ = child.kill();
}
