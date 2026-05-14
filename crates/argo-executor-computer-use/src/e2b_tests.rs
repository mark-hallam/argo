//! Unit tests for [`crate::e2b`]. Pulled in via
//! `#[cfg(test)] #[path = "e2b_tests.rs"] mod tests;` from
//! `e2b.rs` to keep the production module under the 500-LOC
//! per-file invariant.
//!
//! Coverage: `CuAction::summary` formatting, serde tags + default
//! optional fields on `ActionResult`, `E2bClient` construction,
//! and `decode_inline_screenshot` happy / malformed paths.

use super::*;

#[test]
fn cu_action_summary_redacts_long_type_text() {
    let a = CuAction::Type {
        text: "a".repeat(200),
    };
    let s = a.summary();
    assert!(s.starts_with("type \""));
    assert!(s.len() < 60, "should truncate; got {s:?}");
}

#[test]
fn cu_action_summary_formats_each_kind() {
    assert_eq!(
        CuAction::Click { x: 10, y: 20 }.summary(),
        "click (10,20)"
    );
    assert_eq!(
        CuAction::DoubleClick { x: 1, y: 2 }.summary(),
        "double_click (1,2)"
    );
    assert_eq!(
        CuAction::Key {
            keys: vec!["ctrl".into(), "c".into()]
        }
        .summary(),
        "key ctrl+c"
    );
    assert_eq!(
        CuAction::Scroll {
            x: 0,
            y: 0,
            dy: -30
        }
        .summary(),
        "scroll (0,0) dy=-30"
    );
    assert_eq!(
        CuAction::Navigate {
            url: "https://example.com".into()
        }
        .summary(),
        "navigate https://example.com"
    );
}

#[test]
fn cu_action_serialises_with_kind_tag() {
    let a = CuAction::Click { x: 5, y: 7 };
    let v = serde_json::to_value(&a).unwrap();
    assert_eq!(v["kind"], "click");
    assert_eq!(v["x"], 5);
    assert_eq!(v["y"], 7);
}

#[test]
fn action_result_default_optional_fields() {
    let v = serde_json::from_str::<ActionResult>(r#"{"ok": true}"#).unwrap();
    assert!(v.ok);
    assert!(v.text.is_none());
    assert!(v.screenshot_b64.is_none());
}

#[test]
fn e2b_client_builds_with_timeout() {
    // Sanity: client construction doesn't make a network
    // call; it just stamps the reqwest::Client.
    let c = E2bClient::new("http://localhost:1".into(), "k".into(), 5);
    assert!(c.is_ok());
}

#[test]
fn decode_inline_screenshot_returns_raw_bytes_for_valid_b64() {
    // Standard-alphabet base64 of the canonical PNG signature
    // header bytes — exercises the success path the CU loop
    // takes when E2B ships `screenshot_b64` inline.
    let png_header: &[u8] = b"\x89PNG\r\n\x1a\n";
    use base64::engine::general_purpose::STANDARD;
    use base64::Engine;
    let encoded = STANDARD.encode(png_header);
    let bytes = decode_inline_screenshot(&encoded).expect("valid b64 decodes");
    assert_eq!(bytes, png_header);
}

#[test]
fn decode_inline_screenshot_surfaces_protocol_error_for_malformed_b64() {
    // Trailing `!` is not a valid base64 character; decode
    // must surface `CuError::Protocol` rather than silently
    // falling back to a fresh HTTP fetch (which would mask
    // the upstream bug).
    let err = match decode_inline_screenshot("not_valid_base64!!!") {
        Err(e) => e,
        Ok(bytes) => panic!("malformed b64 must error; got {} bytes", bytes.len()),
    };
    match err {
        CuError::Protocol(msg) => {
            assert!(
                msg.contains("malformed inline screenshot"),
                "expected protocol message, got: {msg}",
            );
        }
        other => panic!("expected CuError::Protocol; got: {other:?}"),
    }
}
