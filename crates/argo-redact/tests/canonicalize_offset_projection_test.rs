//! End-to-end offset-projection tests for Stage-1 canonicalisation.
//!
//! These tests pin the contract that [`redact_text`] keeps the
//! **un-canonicalised** input wherever a sentinel doesn't replace a hit
//! — i.e. mask offsets are projected back to the original byte
//! positions so callers (audit / SPA preview) still see the source text
//! around each mask, not the canonical form.

use argo_redact::{redact_text, RedactionPolicy};

fn strict() -> RedactionPolicy {
    RedactionPolicy::default()
}

#[test]
fn fullwidth_at_preserves_surrounding_original_bytes() {
    // Original has a fullwidth `@` (U+FF20, 3 bytes). Canonicalisation
    // folds it to ASCII `@` for the regex pass; offset projection must
    // mask the whole original token's byte range — not a 1-byte slice
    // of the original.
    let input = "ping sam\u{FF20}acmeco.com today";
    let (out, _) = redact_text(input, &strict());
    assert!(
        out.contains("[EMAIL]"),
        "expected fullwidth email caught, got {out:?}"
    );
    // The literal surrounding words must survive unchanged.
    assert!(out.starts_with("ping "), "lead preserved: {out:?}");
    assert!(out.ends_with(" today"), "trail preserved: {out:?}");
    // Neither the original fullwidth `@` nor the canonical ASCII `@`
    // should leak around the sentinel — the mask spans the full
    // original local+domain.
    assert!(
        !out.contains('\u{FF20}'),
        "fullwidth @ leaked into masked output: {out:?}"
    );
    assert!(
        !out.contains("sam"),
        "local-part `sam` leaked into masked output: {out:?}"
    );
    assert!(
        !out.contains("acmeco"),
        "domain `acmeco` leaked into masked output: {out:?}"
    );
}

#[test]
fn url_encoded_at_masks_three_byte_escape() {
    let input = "callback?to=sam%40acmeco.com";
    let (out, _) = redact_text(input, &strict());
    assert_eq!(out, "callback?to=[EMAIL]");
}

#[test]
fn html_entity_at_masks_five_byte_entity() {
    let input = "mailto link sam&#64;acmeco.com";
    let (out, _) = redact_text(input, &strict());
    assert_eq!(out, "mailto link [EMAIL]");
}

#[test]
fn zero_width_space_strip_keeps_unaffected_bytes() {
    let input = "addr sam@\u{200B}acmeco.com confirmed";
    let (out, _) = redact_text(input, &strict());
    assert!(out.contains("[EMAIL]"), "expected mask, got {out:?}");
    assert!(out.starts_with("addr "));
    assert!(out.ends_with(" confirmed"));
    assert!(!out.contains('\u{200B}'), "ZWSP must not leak into output: {out:?}");
}

#[test]
fn soft_hyphen_strip_keeps_unaffected_bytes() {
    let input = "from sam@acm\u{00AD}eco.com received";
    let (out, _) = redact_text(input, &strict());
    assert!(out.contains("[EMAIL]"), "expected mask, got {out:?}");
    assert!(out.starts_with("from "));
    assert!(out.ends_with(" received"));
    assert!(!out.contains('\u{00AD}'));
}

#[test]
fn base64_email_token_masks_whole_token_inplace() {
    let input = "credential: c2FtQGFjbWVjby5jb20= please verify";
    let (out, _) = redact_text(input, &strict());
    assert_eq!(out, "credential: [EMAIL] please verify");
}

#[test]
fn fullwidth_digit_card_passes_luhn_after_canonicalisation() {
    // PAN 4111-1111-1111-1111 expressed entirely in fullwidth digits.
    // `char::to_digit(10)` rejects fullwidth, so the v1 Luhn check
    // would have dropped the candidate — leaking the full PAN. After
    // NFKC the digits become ASCII and Luhn validates.
    let input = "card \u{FF14}\u{FF11}\u{FF11}\u{FF11} \u{FF11}\u{FF11}\u{FF11}\u{FF11} \u{FF11}\u{FF11}\u{FF11}\u{FF11} \u{FF11}\u{FF11}\u{FF11}\u{FF11} ok";
    let (out, _) = redact_text(input, &strict());
    assert!(
        out.contains("[CARD]"),
        "fullwidth-digit PAN must Luhn-validate after NFKC, got {out:?}"
    );
    assert!(out.starts_with("card "));
    assert!(out.ends_with(" ok"));
}

#[test]
fn multiple_canonicalisations_in_one_input_all_caught() {
    let input =
        "users sam\u{FF20}acmeco.com and bob%40acme.io and carol&#64;example.org";
    let (out, _) = redact_text(input, &strict());
    let count = out.matches("[EMAIL]").count();
    assert_eq!(count, 3, "expected 3 [EMAIL] sentinels, got {out:?}");
    assert!(!out.contains('\u{FF20}'));
    assert!(!out.contains("%40"));
    assert!(!out.contains("&#64;"));
}

#[test]
fn canonicalisation_does_not_break_plain_ascii_path() {
    // Sanity: input with no canonicalisation-needing chars round-trips
    // identically to the v1 regex layer.
    let input = "Contact me at sam@acmeco.com or call 027 555 1234";
    let (out, _) = redact_text(input, &strict());
    assert_eq!(out, "Contact me at [EMAIL] or call [PHONE]");
}

#[test]
fn base64_without_pii_is_not_masked() {
    // 16-char base64-shaped token that decodes to "helloworldaaaa" — no
    // PII inside, must not be masked (would be a high-noise false
    // positive on hashes / IDs).
    let input = "tag aGVsbG93b3JsZGFhYWE= ok";
    let (out, _) = redact_text(input, &strict());
    assert_eq!(out, "tag aGVsbG93b3JsZGFhYWE= ok");
}

#[test]
fn invalid_url_escape_passes_through() {
    let input = "100% off promo code ABCDEF";
    let (out, _) = redact_text(input, &strict());
    assert_eq!(out, input);
}
