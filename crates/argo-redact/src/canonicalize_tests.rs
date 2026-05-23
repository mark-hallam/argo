//! Internal unit tests for `canonicalize` — split out of `canonicalize.rs`
//! to keep the implementation file under the 500-LOC ceiling.

use super::*;

#[test]
fn nfkc_folds_fullwidth_at() {
    let c = canonicalize_for_scan("a\u{FF20}b");
    assert_eq!(c.text, "a@b");
    // Original input is 5 bytes (1 + 3 + 1). Canonical is 3 bytes.
    // Canonical[1] (the @) should map back to original byte 1 where
    // the fullwidth @ starts.
    assert_eq!(c.original_span(1..2), 1..4);
    assert_eq!(c.original_span(0..1), 0..1);
    assert_eq!(c.original_span(2..3), 4..5);
}

#[test]
fn nfkc_folds_fullwidth_digits() {
    let c = canonicalize_for_scan(
        "\u{FF14}\u{FF11}\u{FF11}\u{FF11} \u{FF11}\u{FF11}\u{FF11}\u{FF11}",
    );
    assert_eq!(c.text, "4111 1111");
}

#[test]
fn strips_zero_width_space() {
    let c = canonicalize_for_scan("a\u{200B}b");
    assert_eq!(c.text, "ab");
    // canonical 'a' (byte 0) → original byte 0.
    // canonical 'b' (byte 1) → original byte 4 (after the 3-byte ZWSP).
    assert_eq!(c.original_span(1..2), 4..5);
}

#[test]
fn strips_soft_hyphen() {
    let c = canonicalize_for_scan("acm\u{00AD}eco");
    assert_eq!(c.text, "acmeco");
}

#[test]
fn url_decodes_at_sign() {
    let c = canonicalize_for_scan("sam%40acmeco.com");
    assert_eq!(c.text, "sam@acmeco.com");
    // canonical '@' is at byte 3; it came from original bytes 3..6
    // ("%40").
    assert_eq!(c.original_span(3..4), 3..6);
    // canonical 'c' (after '@') maps to original byte 6.
    assert_eq!(c.original_span(4..5), 6..7);
}

#[test]
fn html_decimal_entity_decodes() {
    let c = canonicalize_for_scan("sam&#64;acmeco.com");
    assert_eq!(c.text, "sam@acmeco.com");
    // canonical '@' is byte 3; it came from original "&#64;" (bytes 3..8).
    assert_eq!(c.original_span(3..4), 3..8);
}

#[test]
fn html_hex_entity_decodes() {
    let c = canonicalize_for_scan("sam&#x40;acmeco.com");
    assert_eq!(c.text, "sam@acmeco.com");
}

#[test]
fn html_named_entity_amp_decodes() {
    let c = canonicalize_for_scan("Q&amp;A");
    assert_eq!(c.text, "Q&A");
}

#[test]
fn url_decode_leaves_non_ascii_alone() {
    // %C3%A9 decodes to two bytes (UTF-8 "é"); we don't fold
    // non-ASCII to preserve string validity.
    let input = "caf%C3%A9";
    let c = canonicalize_for_scan(input);
    assert_eq!(c.text, input);
}

#[test]
fn url_decode_skips_invalid_escape() {
    let c = canonicalize_for_scan("100%off");
    assert_eq!(c.text, "100%off");
}

#[test]
fn html_entity_skips_unknown_named() {
    let c = canonicalize_for_scan("&unknownentity;");
    assert_eq!(c.text, "&unknownentity;");
}

#[test]
fn html_entity_handles_runaway_open() {
    let c = canonicalize_for_scan("just & a stray ampersand");
    assert_eq!(c.text, "just & a stray ampersand");
}

#[test]
fn empty_input_round_trips() {
    let c = canonicalize_for_scan("");
    assert_eq!(c.text, "");
    assert_eq!(c.offsets, vec![0]);
    assert_eq!(c.original_span(0..0), 0..0);
}

#[test]
fn ascii_only_input_identity_offsets() {
    let s = "plain ASCII text";
    let c = canonicalize_for_scan(s);
    assert_eq!(c.text, s);
    assert_eq!(c.offsets, (0..=s.len()).collect::<Vec<_>>());
}

#[test]
fn combined_canonicalisations_compose() {
    // Fullwidth @ + ZWSP + URL-encoded dot.
    let input = "u\u{FF20}\u{200B}c%2Ecom";
    let c = canonicalize_for_scan(input);
    assert_eq!(c.text, "u@c.com");
    // Validate one anchor: canonical '.' (byte 3) → original bytes
    // for "%2E" (3 bytes starting at original position 8).
    // Original layout: 'u'(0) + FF20(1-3) + ZWSP(4-6) + 'c'(7) +
    // '%'(8) + '2'(9) + 'E'(10) + 'c'(11) + 'o'(12) + 'm'(13).
    assert_eq!(c.original_span(3..4), 8..11);
}

#[test]
fn original_span_is_monotonic() {
    let inputs = [
        "abc",
        "sam\u{FF20}acmeco.com",
        "sam%40acmeco.com",
        "sam&#64;acmeco.com",
        "acm\u{00AD}eco.com",
        "mix \u{200B}sam\u{FF20}acm%2Eeco&#46;com",
    ];
    for input in inputs {
        let c = canonicalize_for_scan(input);
        let mut last = 0usize;
        for &o in &c.offsets {
            assert!(o >= last, "offsets non-monotonic for {input:?}: {:?}", c.offsets);
            last = o;
        }
        assert_eq!(c.offsets.last().copied().unwrap_or_default(), input.len());
    }
}

#[test]
fn looks_like_base64_accepts_padding() {
    assert!(looks_like_base64("c2FtQGFjbWVjby5jb20="));
    assert!(looks_like_base64("aGVsbG93b3JsZGFhYWE="));
}

#[test]
fn looks_like_base64_rejects_short_or_invalid() {
    assert!(!looks_like_base64("hi"));
    assert!(!looks_like_base64("hello world!"));
    assert!(!looks_like_base64("not%base64="));
}

#[test]
fn decode_base64_round_trip() {
    assert_eq!(
        decode_base64_token("c2FtQGFjbWVjby5jb20=").as_deref(),
        Some("sam@acmeco.com"),
    );
}

#[test]
fn decode_base64_rejects_non_utf8() {
    // 0xFF 0xFE 0xFD is not valid UTF-8.
    assert!(decode_base64_token("//79").is_none());
}
