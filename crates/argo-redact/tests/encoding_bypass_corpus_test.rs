//! Encoding-bypass corpus for `argo-redact` v1 (regex-only fence).
//!
//! Black-box probes the redactor with payloads engineered to slip past
//! a pattern-only PII fence: base64/URL/HTML-entity wrappers, Unicode
//! homoglyph lookalikes, zero-width-character injection, char-jitter,
//! soft hyphens, IP-literal email domains, MIME quoted-printable, en-
//! dash phone separators, and so on.
//!
//! Why this lives in `tests/` rather than `lib_tests.rs`: this corpus
//! is an **audit artifact** — it pins current regex behaviour and
//! enumerates the leak surface that motivates the
//! `mission-redact-promote-to-real-fence` follow-up. Keeping it in a
//! sibling integration test makes it easy to re-run as a regression
//! check after the canonicalisation layer lands.
//!
//! Two tracks of tests live here:
//!
//! 1. **Pin tests** (`#[test]`): always-passing snapshots of what the
//!    v1 regex layer does today. They flag any regression introduced
//!    by future tuning — if a pin moves, the maintainer decides
//!    whether the move is intended.
//! 2. **FINDINGS-only `#[ignore]`'d tests**: assert what a real fence
//!    (NER + Unicode canonicalisation + wrapper decoding) MUST catch.
//!    They fail today and turn green when the promoted fence lands.
//!    Un-ignore in the follow-up `mission-redact-promote-to-real-fence`
//!    PR.
//!
//! See companion report:
//! `docs/skycot/evals/scheduled/2026-05-23/eval-redact-crate-promote-from-regex-to-real-fence.md`.

use argo_redact::{redact_text, RedactionPolicy};

fn strict() -> RedactionPolicy {
    RedactionPolicy::default()
}

/// Assert this input is NOT redacted (current regex MISSES it).
#[track_caller]
fn assert_pin_miss(label: &str, input: &str) {
    let (out, hits) = redact_text(input, &strict());
    assert_eq!(out, input, "{label}: expected MISS, got mask: {out:?}");
    assert!(
        hits.is_empty(),
        "{label}: expected no hits, got {hits:?}"
    );
}

/// Assert input IS redacted (current regex CATCHES it).
#[track_caller]
fn assert_pin_catch(label: &str, input: &str) {
    let (out, hits) = redact_text(input, &strict());
    assert_ne!(
        out, input,
        "{label}: expected mask, got identity: {out:?}"
    );
    assert!(!hits.is_empty(), "{label}: expected hits, got none");
}

// ============================================================================
// § Email — REGEX MISSES (bypass surface)
// ============================================================================

#[test]
fn pin_email_base64_wrapped_is_missed() {
    // "sam@acmeco.com" base64-encoded. A regex fence cannot see the
    // `@` literal; only a wrapper-decode pass plus re-scan would.
    assert_pin_miss("base64 email", "credential: c2FtQGFjbWVjby5jb20=");
}

#[test]
fn pin_email_url_encoded_is_missed() {
    // `%40` is the URL-encoded `@`. Common in audit payloads pulled
    // from HTTP query strings or webhook fragments.
    assert_pin_miss("url-encoded email", "callback?to=sam%40acmeco.com");
}

#[test]
fn pin_email_html_entity_at_is_missed() {
    // `&#64;` is the HTML decimal entity for `@`. Common when the
    // gateway receives a rendered HTML notification body verbatim.
    assert_pin_miss("html-entity email", "mailto link sam&#64;acmeco.com");
}

#[test]
fn pin_email_fullwidth_at_is_missed() {
    // U+FF20 FULLWIDTH COMMERCIAL AT renders visually identical to
    // `@` but is a different code point. Trivial homoglyph bypass.
    assert_pin_miss("fullwidth @", "ping sam\u{FF20}acmeco.com today");
}

#[test]
fn pin_email_zero_width_space_split_is_missed() {
    // U+200B ZERO WIDTH SPACE between `@` and the domain. Invisible
    // to a human reader; lethal to a `\b[A-Za-z0-9.\-]+\.[A-Za-z]{2,}`
    // domain class.
    assert_pin_miss(
        "zero-width-space split",
        "addr sam@\u{200B}acmeco.com confirmed",
    );
}

#[test]
fn pin_email_whitespace_jitter_is_missed() {
    // Char-jitter is the classic ad-hoc obfuscation: insert a space
    // between every character. Defeats `\b[A-Za-z0-9._%+\-]+@`
    // entirely.
    assert_pin_miss(
        "whitespace jitter",
        "reach s a m @ c u r b l e . c o m maybe",
    );
}

#[test]
fn pin_email_bracketed_at_dot_is_missed() {
    // Common in mailing-list footers to defeat scrapers; same trick
    // defeats a regex fence.
    assert_pin_miss("bracketed [at] [dot]", "sam[at]acmeco[dot]com");
}

#[test]
fn pin_email_soft_hyphen_in_domain_is_missed() {
    // U+00AD SOFT HYPHEN renders as nothing in most fonts but
    // breaks the contiguous domain match.
    assert_pin_miss(
        "soft hyphen in domain",
        "from sam@acm\u{00AD}eco.com received",
    );
}

#[test]
fn pin_email_ip_literal_domain_is_missed() {
    // RFC-5321-legal address using an IP literal. The v1 regex's
    // domain class `[A-Za-z0-9.\-]+` rejects `[` and `]`.
    assert_pin_miss(
        "ip-literal domain",
        "admin sam@[192.168.1.1] on the box",
    );
}

#[test]
fn pin_email_combining_acute_in_local_is_missed() {
    // `sa\u{0301}m` — `s`, `a`, combining acute, `m`. Visually
    // `sám@acmeco.com`. Combining mark not in the local-part class.
    assert_pin_miss(
        "combining acute",
        "contact sa\u{0301}m@acmeco.com if needed",
    );
}

#[test]
fn pin_email_mime_quoted_printable_at_is_missed() {
    // `=40` is the MIME quoted-printable encoding of `@`. Common
    // when audit captures a raw QP-encoded mail body.
    assert_pin_miss(
        "mime quoted-printable",
        "header sam=40acmeco.com end",
    );
}

#[test]
fn pin_email_newline_split_is_missed() {
    // Hard line break between local and domain. Some renderers
    // strip the newline for display; the regex cannot.
    assert_pin_miss(
        "newline split",
        "wrapped sam@\nacmeco.com end",
    );
}

#[test]
fn pin_email_quoted_local_form_is_missed() {
    // RFC-5321 §4.1.2 permits `"quoted local"@domain`. The v1 regex
    // local-part class `[A-Za-z0-9._%+\-]` rejects `"`, and the
    // backtracker cannot find a contiguous run ending in `@`. Real
    // PII fences (Presidio, GuardrailsAI) all canonicalise the local
    // part before pattern matching.
    assert_pin_miss(
        "quoted-local RFC-5321 form",
        "see \"sam.lastname\"@acmeco.com please",
    );
}

// ============================================================================
// § Email — REGEX CATCHES (sanity pins for non-obvious positives)
// ============================================================================

#[test]
fn pin_email_subaddress_plus_is_caught() {
    // `+suffix` sub-addressing is RFC-5233 legal and inside the
    // local-part class. Caught cleanly.
    assert_pin_catch("subaddress plus", "to sam+billing@acmeco.com");
}

// ============================================================================
// § Phone — REGEX MISSES
// ============================================================================

#[test]
fn pin_phone_devanagari_digits_are_missed() {
    // `\d` in regex Unicode mode matches `\p{Nd}` — Devanagari digits
    // qualify. BUT the NZ-trunk branch requires a literal ASCII `0`
    // ahead of the run, so Devanagari `\u{0966}` (digit-zero) fails
    // the anchor. US 3-3-4 has no literal anchor; whether this hits
    // depends on `\b` behaviour with non-ASCII-script word chars.
    // This pin captures whichever the current build does.
    let s = "phone \u{0966}\u{096A}\u{0967} \u{096B}\u{096B}\u{096B} \u{0966}\u{0967}\u{0968}\u{0969} now";
    let (out, hits) = redact_text(s, &strict());
    // Whatever the current behaviour, document it. If the regex
    // engine catches the US-shape branch on Unicode digits, this
    // assertion will need flipping — and that flip is itself a
    // useful audit signal.
    if hits.is_empty() {
        assert_eq!(out, s, "Devanagari-digit phone is currently a MISS");
    } else {
        assert!(
            out.contains("[PHONE]"),
            "Devanagari-digit phone hit but did not produce [PHONE] sentinel"
        );
    }
}

#[test]
fn pin_phone_words_spelled_out_is_missed() {
    // Sales / support transcripts often spell numbers as words.
    assert_pin_miss(
        "spelled-out phone",
        "call oh-two-seven five-five-five one-two-three-four",
    );
}

#[test]
fn pin_phone_en_dash_separator_is_missed() {
    // U+2013 EN DASH instead of U+002D HYPHEN. Looks the same in
    // most fonts; the regex's literal `[\-\.]` rejects it.
    assert_pin_miss(
        "en-dash phone",
        "dial 555\u{2013}555\u{2013}0123 today",
    );
}

// ============================================================================
// § Phone — REGEX CATCHES
// ============================================================================

#[test]
fn pin_phone_with_trailing_extension_text_is_caught() {
    // The (555) 555-0123 prefix should still match; `ext. 42` is
    // outside the consumed range. Pin: prefix is caught.
    assert_pin_catch(
        "phone with text extension",
        "office (555) 555-0123 ext. 42",
    );
}

// ============================================================================
// § Card — REGEX MISSES (Luhn-aware path)
// ============================================================================

#[test]
fn pin_card_dot_separator_is_missed() {
    // Card regex is `\d[ \-]?` between groups — only space or hyphen
    // accepted between digit pairs. Dots are not.
    assert_pin_miss(
        "card with dot separators",
        "saved 4111.1111.1111.1111 on file",
    );
}

#[test]
fn pin_card_underscore_separator_is_missed() {
    assert_pin_miss(
        "card with underscore separators",
        "csv col 4111_1111_1111_1111 row",
    );
}

#[test]
fn pin_card_letter_obfuscation_is_missed() {
    // `l` for `1`, `O` for `0`. Used in legacy log-redaction scripts
    // and as a manual paste-time mask. Defeats the digit-class regex.
    assert_pin_miss(
        "card l-for-1 obfuscation",
        "saved 4l11 llll llll llll on file",
    );
}

#[test]
fn pin_card_fullwidth_digits_behaviour_is_pinned() {
    // U+FF11 .. U+FF14 are fullwidth `1`..`4`. `\d` (Unicode) catches
    // them; `char::to_digit(10)` does NOT (ASCII-only). Therefore
    // the regex finds a candidate, Luhn rejects it, and the card is
    // not masked → bypass succeeds.
    let s = "card \u{FF14}\u{FF11}\u{FF11}\u{FF11} \u{FF11}\u{FF11}\u{FF11}\u{FF11} \u{FF11}\u{FF11}\u{FF11}\u{FF11} \u{FF11}\u{FF11}\u{FF11}\u{FF11} ok";
    let (out, hits) = redact_text(s, &strict());
    if hits.is_empty() {
        assert_eq!(out, s, "fullwidth-digit card is a MISS (Luhn rejects non-ASCII)");
    } else {
        // If a future regex/luhn upgrade catches it, document the
        // flip; the assertion still passes because we got a sentinel.
        assert!(
            out.contains("[CARD]"),
            "fullwidth-digit card matched but sentinel missing"
        );
    }
}

// ============================================================================
// § Card — REGEX CATCHES
// ============================================================================

#[test]
fn pin_card_with_inline_text_is_caught() {
    assert_pin_catch(
        "card embedded mid-sentence",
        "I gave them 4111 1111 1111 1111 over the phone",
    );
}

// ============================================================================
// § IBAN — REGEX MISSES
// ============================================================================

#[test]
fn pin_iban_with_canonical_spacing_is_missed() {
    // IBANs are conventionally rendered with a space every 4 chars
    // (`GB82 WEST 1234 5698 7654 32`). The v1 regex requires the
    // 30-char run be unbroken — a paste from a banking UI bypasses.
    assert_pin_miss(
        "spaced IBAN canonical form",
        "send to GB82 WEST 1234 5698 7654 32 please",
    );
}

#[test]
fn pin_iban_lowercase_country_code_is_missed() {
    // Already documented in lib_tests; pinned here as part of the
    // single-source bypass surface.
    assert_pin_miss(
        "lowercase IBAN prefix",
        "send to gb82WEST12345698765432 please",
    );
}

// ============================================================================
// § Address — REGEX MISSES
// ============================================================================

#[test]
fn pin_address_unrecognized_suffix_terrace_is_missed() {
    // `Terrace` and `Tce` are not in the suffix whitelist.
    assert_pin_miss(
        "unrecognized suffix Terrace",
        "meet at 123 Main Terrace then",
    );
}

#[test]
fn pin_address_unrecognized_suffix_circle_is_missed() {
    assert_pin_miss(
        "unrecognized suffix Circle",
        "ship to 456 Maple Circle thanks",
    );
}

#[test]
fn pin_address_au_unit_slash_form_is_missed() {
    // `5/123 Main St` is the canonical Australian unit-slash form.
    // The pattern requires `\d{1,5}\s+...` — the `5/` group breaks
    // the leading number into a fragment with no whitespace.
    // Captures the AU/NZ blind spot.
    let s = "post to 5/123 Main St Carlton";
    let (out, hits) = redact_text(s, &strict());
    // The regex MAY still catch `123 Main St` as a substring. Pin
    // whatever the current build does.
    if hits.is_empty() {
        assert_eq!(out, s, "AU unit-slash address is a MISS");
    } else {
        // If it caught the inner `123 Main St`, the unit number `5`
        // leaks. Real-fence work should fold the unit into the mask.
        assert!(
            out.contains("[ADDR]"),
            "AU unit-slash address matched but sentinel missing"
        );
    }
}

// ============================================================================
// § Cross-category multi-PII (sanity)
// ============================================================================

#[test]
fn pin_multi_pii_inline_is_caught() {
    // Three PIIs in one line, regex catches all.
    let s = "user sam@acmeco.com on 027 555 1234 paid 4111 1111 1111 1111";
    let (out, hits) = redact_text(s, &strict());
    assert!(out.contains("[EMAIL]"), "email missed in multi-PII line");
    assert!(out.contains("[PHONE]"), "phone missed in multi-PII line");
    assert!(out.contains("[CARD]"), "card missed in multi-PII line");
    assert_eq!(hits.len(), 3, "expected 3 categories, got {hits:?}");
}

// ============================================================================
// § FINDINGS-ONLY (ignored) — what a real fence MUST catch
// ============================================================================
//
// These tests assert the OUTCOME a canonicalisation-aware fence would
// produce. They fail under v1's regex-only redactor. Un-ignore them in
// the follow-up `mission-redact-promote-to-real-fence` PR once the
// canonicalisation pre-pass (NFKC + URL-decode + base64-detect + HTML-
// entity-decode + zero-width / soft-hyphen strip) lands.

#[test]
#[ignore = "FINDINGS-ONLY: real fence must mask base64-encoded PII once a wrapper-decode + rescan pass exists"]
fn finding_base64_email_must_be_masked() {
    assert_pin_catch("base64 email", "credential: c2FtQGFjbWVjby5jb20=");
}

#[test]
#[ignore = "FINDINGS-ONLY: real fence must mask URL-encoded PII once a decode + rescan pass exists"]
fn finding_url_encoded_email_must_be_masked() {
    assert_pin_catch("url-encoded email", "callback?to=sam%40acmeco.com");
}

#[test]
#[ignore = "FINDINGS-ONLY: real fence must mask HTML-entity-encoded PII once a decode + rescan pass exists"]
fn finding_html_entity_email_must_be_masked() {
    assert_pin_catch("html-entity email", "mailto link sam&#64;acmeco.com");
}

#[test]
#[ignore = "FINDINGS-ONLY: real fence must NFKC-normalise input before regex pass to catch homoglyphs"]
fn finding_fullwidth_at_email_must_be_masked() {
    assert_pin_catch("fullwidth @", "ping sam\u{FF20}acmeco.com today");
}

#[test]
#[ignore = "FINDINGS-ONLY: real fence must strip U+200B ZERO WIDTH SPACE before regex pass"]
fn finding_zero_width_space_email_must_be_masked() {
    assert_pin_catch(
        "zero-width-space split",
        "addr sam@\u{200B}acmeco.com confirmed",
    );
}

#[test]
#[ignore = "FINDINGS-ONLY: real fence must strip U+00AD SOFT HYPHEN before regex pass"]
fn finding_soft_hyphen_email_must_be_masked() {
    assert_pin_catch(
        "soft hyphen in domain",
        "from sam@acm\u{00AD}eco.com received",
    );
}

#[test]
#[ignore = "FINDINGS-ONLY: real fence must accept en-dash + em-dash separators in phone shapes"]
fn finding_en_dash_phone_must_be_masked() {
    assert_pin_catch(
        "en-dash phone",
        "dial 555\u{2013}555\u{2013}0123 today",
    );
}

#[test]
#[ignore = "FINDINGS-ONLY: real fence must accept dot-separated card digit groups after canonicalisation"]
fn finding_dot_separated_card_must_be_masked() {
    assert_pin_catch(
        "card with dots",
        "saved 4111.1111.1111.1111 on file",
    );
}

#[test]
#[ignore = "FINDINGS-ONLY: real fence must accept canonical IBAN spacing (every-4-chars convention)"]
fn finding_spaced_iban_must_be_masked() {
    assert_pin_catch(
        "spaced IBAN",
        "send to GB82 WEST 1234 5698 7654 32 please",
    );
}

#[test]
#[ignore = "FINDINGS-ONLY: real fence must broaden street-suffix vocabulary (Terrace, Circle, Mews, Crescent, …)"]
fn finding_terrace_address_must_be_masked() {
    assert_pin_catch(
        "Terrace suffix",
        "meet at 123 Main Terrace then",
    );
}
