//! Tests for the image-intake extractor — caption + provenance descriptor,
//! the no-caption refusal (no fake OCR), and the byte caps.

use super::*;
// `Engine` (for `.encode`) is already in scope via `use super::*`; only the
// `general_purpose` path binding is needed here.
use base64::engine::general_purpose;

const CAPTION: &str = "The escalation board shows the red lane is for Sev-1 outages \
paged to the on-call engineer within five minutes.";

#[test]
fn caption_and_provenance_become_gradeable_text() {
    let img = general_purpose::STANDARD.encode(b"\x89PNG\r\n\x1a\nfake-image-bytes");
    let (text, bytes) = describe_image(&img, CAPTION, "board.png", 0).expect("ok");
    assert!(text.contains("board.png"));
    assert!(text.contains("sha256"));
    assert!(text.contains("on-call engineer"));
    assert_eq!(bytes, b"\x89PNG\r\n\x1a\nfake-image-bytes".len());
}

#[test]
fn empty_image_is_rejected() {
    let err = describe_image("   ", CAPTION, "x.png", 0).unwrap_err();
    assert!(err.to_lowercase().contains("empty"));
}

#[test]
fn bad_base64_is_rejected_not_panic() {
    let err = describe_image("!!!not base64!!!", CAPTION, "x.png", 0).unwrap_err();
    assert!(err.to_lowercase().contains("skipped"));
}

#[test]
fn missing_caption_is_refused_no_fake_ocr() {
    // No theater: with no operator description there is nothing real to learn,
    // so the image is skipped rather than persisting an empty knowledge claim.
    let img = general_purpose::STANDARD.encode(b"some-image");
    let err = describe_image(&img, "  ", "x.png", 0).unwrap_err();
    assert!(err.to_lowercase().contains("description"));
}

#[test]
fn total_cap_is_enforced() {
    let img = general_purpose::STANDARD.encode(b"tiny");
    let err = describe_image(&img, CAPTION, "x.png", MAX_TOTAL_BYTES).unwrap_err();
    assert!(err.to_lowercase().contains("total"));
}
