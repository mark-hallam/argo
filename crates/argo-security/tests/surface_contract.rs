//! Surface-contract pin for the `argo-security` crate.
//!
//! This integration test consumes only the public crate API (via
//! `use argo_security::*`) so any rename, removal, or signature drift
//! of an exported item is a compile failure rather than a silent
//! semantic regression in a downstream caller. The tests below also
//! pin behavioral contracts that the existing in-module unit tests
//! cover from inside `super::pii_detect` — restating them through the
//! public surface is the point of a contract test.
//!
//! Coverage matrix (matches `eval-argo-security-crate-surface`
//! acceptance criteria):
//!
//! - Pin the exported module/type/function inventory from
//!   `argo_security::lib`.
//! - Positive + negative coverage for every detector class the public
//!   API surfaces today: SSN, credit-card (Luhn), email, phone, IBAN,
//!   and street address.
//! - Byte-span stability across two multi-byte (emoji + CJK) and one
//!   punctuation-heavy input — the documented contract is that
//!   `PiiFinding::span` is a byte range that slices cleanly from the
//!   original input regardless of upstream Unicode width.
//! - End-to-end `apply_policy` round-trip with each of the three
//!   env-knob branches (`detect=false`, soft-flag, autoredact).
//!
//! The test does not read process env. `apply_policy` accepts an
//! explicit `PolicyConfig`, so we can pin every branch without
//! racing sibling tests in the same process.

use argo_security::pii_detect::{
    apply_policy, scan, scan_and_maybe_redact, PiiFinding, PiiFindings, PiiKind, PolicyConfig,
};
use std::borrow::Cow;

// ---------- Inventory pin --------------------------------------------------

/// Compile-fence: every name in the crate's published surface is
/// referenced here. Adding or removing a public name must update this
/// test, which forces a deliberate decision rather than silent drift.
#[test]
fn public_surface_inventory_pinned() {
    // Functions
    let _: fn(&str) -> PiiFindings = scan;
    let _: for<'a> fn(&'a str, &'static str) -> Cow<'a, str> = scan_and_maybe_redact;
    let _: for<'a> fn(&'a str, PolicyConfig, &'static str) -> Cow<'a, str> = apply_policy;

    // Types reachable from a fresh consumer.
    let _kinds: [PiiKind; 6] = [
        PiiKind::Ssn,
        PiiKind::CreditCard,
        PiiKind::Email,
        PiiKind::UsPhone,
        PiiKind::Iban,
        PiiKind::StreetAddress,
    ];

    // PiiFinding fields are public.
    let f = PiiFinding {
        kind: PiiKind::Email,
        span: 0..0,
        masked_value: String::new(),
    };
    assert_eq!(f.kind, PiiKind::Email);

    // PiiFindings tuple field is public.
    let fs = PiiFindings(Vec::<PiiFinding>::new());
    assert!(fs.is_empty());

    // PolicyConfig fields are public.
    let cfg = PolicyConfig {
        detect: true,
        autoredact: false,
        log_findings: false,
    };
    assert!(cfg.detect);

    // PolicyConfig::from_env reads the env knobs — call it once so the
    // function symbol is referenced from the integration crate (signal
    // for accidental removal).
    let _runtime_cfg = PolicyConfig::from_env();
}

/// Stable wire labels for every `PiiKind`. These labels appear in
/// `<redacted:label>` markers persisted to the audit log; a rename
/// breaks downstream consumers that grep audit JSON.
#[test]
fn pii_kind_labels_pinned() {
    assert_eq!(PiiKind::Ssn.label(), "ssn");
    assert_eq!(PiiKind::CreditCard.label(), "credit_card");
    assert_eq!(PiiKind::Email.label(), "email");
    assert_eq!(PiiKind::UsPhone.label(), "us_phone");
    assert_eq!(PiiKind::Iban.label(), "iban");
    assert_eq!(PiiKind::StreetAddress.label(), "street_address");
}

// ---------- Detector classes: positive + negative -------------------------

fn kinds(f: &PiiFindings) -> Vec<PiiKind> {
    f.0.iter().map(|x| x.kind).collect()
}

#[test]
fn ssn_positive_and_negative() {
    let hit = scan("note: my ssn is 123-45-6789 ok?");
    assert_eq!(kinds(&hit), vec![PiiKind::Ssn]);

    // Shape filter: area `9XX` must be rejected (issuance reserved).
    let miss = scan("note: my ssn is 912-45-6789 ok?");
    assert!(miss.is_empty(), "9xx-area SSN must be rejected: {miss:?}");

    // Shape filter: all-zero serial.
    let miss2 = scan("note: my ssn is 123-45-0000 ok?");
    assert!(miss2.is_empty(), "all-zero serial must be rejected: {miss2:?}");
}

#[test]
fn credit_card_positive_and_negative() {
    // Visa test number, Luhn-valid.
    let hit = scan("pay with 4111 1111 1111 1111 today");
    assert_eq!(kinds(&hit), vec![PiiKind::CreditCard]);

    // Last digit flipped: same shape, Luhn-invalid, must NOT match.
    let miss = scan("pay with 4111 1111 1111 1112 today");
    assert!(miss.is_empty(), "Luhn-invalid CC must be rejected: {miss:?}");
}

#[test]
fn email_positive_and_negative() {
    let hit = scan("ping alice.smith+work@example.co.uk for the answer");
    assert_eq!(kinds(&hit), vec![PiiKind::Email]);

    // `@` without a TLD shape (or with a 1-letter TLD) must not match.
    let miss = scan("not an email: alice@localhost or bob@x");
    assert!(miss.is_empty(), "bare host / 1-char TLD must miss: {miss:?}");
}

#[test]
fn phone_positive_and_negative() {
    // US 10-digit with parens.
    let hit = scan("call (415) 555-0123 today");
    assert_eq!(kinds(&hit), vec![PiiKind::UsPhone]);

    // +CC international.
    let hit_intl = scan("reach me at +44 20 7946 0958 today");
    assert_eq!(kinds(&hit_intl), vec![PiiKind::UsPhone]);

    // 4-digit year must NOT match.
    let miss = scan("shipped in 2026, no PII here");
    assert!(miss.is_empty(), "4-digit year flagged as phone: {miss:?}");
}

#[test]
fn iban_positive_and_negative() {
    // Canonical GB IBAN, mod-97 valid.
    let hit = scan("send to GB82WEST12345698765432 today");
    assert_eq!(kinds(&hit), vec![PiiKind::Iban]);

    // Same shape, wrong checksum.
    let miss = scan("send to GB99WEST12345698765432 today");
    assert!(miss.is_empty(), "bad-checksum IBAN must miss: {miss:?}");
}

#[test]
fn street_address_positive_and_negative() {
    let hit = scan("meet at 456 North Main Ave at noon");
    assert_eq!(kinds(&hit), vec![PiiKind::StreetAddress]);

    // Bare integer with no `<digits> <name-words> <suffix>` shape.
    let miss = scan("see line 123 of the report");
    assert!(miss.is_empty(), "bare integer must miss: {miss:?}");

    // Lone suffix `St` with no leading digits must not match.
    let miss2 = scan("turn left at St Andrews and stop");
    assert!(
        miss2.is_empty(),
        "lone suffix flagged as address: {miss2:?}"
    );
}

// ---------- Byte-span stability across Unicode / punctuation --------------

#[test]
fn byte_span_stable_with_emoji_prefix() {
    // Each rocket emoji is 4 bytes in UTF-8; `ping ` is 5 ASCII bytes.
    // The span must point at the email's bytes in the ORIGINAL input
    // so callers can `&input[f.span]` without manual rune accounting.
    let input = "🚀 ping alice@example.com 🎉";
    let findings = scan(input);
    assert_eq!(kinds(&findings), vec![PiiKind::Email]);
    let f = &findings.0[0];
    assert_eq!(
        &input[f.span.clone()],
        "alice@example.com",
        "byte span must slice cleanly through multi-byte prefix"
    );
    // Redact must keep all non-finding bytes (emoji included) intact.
    let redacted = findings.redact(input);
    assert_eq!(redacted, "🚀 ping <redacted:email> 🎉");
}

#[test]
fn byte_span_stable_with_cjk_prefix() {
    // CJK characters are 3 bytes each in UTF-8; the phone number sits
    // after a `联系 ` prefix (3+3+1 = 7 bytes before the digits).
    let input = "联系 415-555-0123 谢谢";
    let findings = scan(input);
    assert_eq!(kinds(&findings), vec![PiiKind::UsPhone]);
    let f = &findings.0[0];
    assert_eq!(
        &input[f.span.clone()],
        "415-555-0123",
        "byte span must slice cleanly through CJK prefix"
    );
    let redacted = findings.redact(input);
    assert_eq!(redacted, "联系 <redacted:us_phone> 谢谢");
}

#[test]
fn byte_span_stable_with_punctuation_heavy_input() {
    // Punctuation around a credit card: the regex must anchor to the
    // 16-digit run only, and the slice must round-trip.
    let input = "(card:4111-1111-1111-1111;ok)";
    let findings = scan(input);
    assert_eq!(kinds(&findings), vec![PiiKind::CreditCard]);
    let f = &findings.0[0];
    assert_eq!(
        &input[f.span.clone()],
        "4111-1111-1111-1111",
        "span must exclude surrounding punctuation"
    );
    let redacted = findings.redact(input);
    assert_eq!(redacted, "(card:<redacted:credit_card>;ok)");
}

// ---------- Multi-finding ordering + de-overlap pin -----------------------

#[test]
fn multi_finding_ordering_pinned() {
    // Two non-overlapping findings of different kinds: spans must be
    // sorted by `start`, redaction must preserve the inter-finding text.
    let s = "ssn 123-45-6789 mail bob@example.com here";
    let findings = scan(s);
    assert_eq!(kinds(&findings), vec![PiiKind::Ssn, PiiKind::Email]);
    assert!(
        findings.0[0].span.end <= findings.0[1].span.start,
        "spans must be non-overlapping and sorted"
    );
    assert_eq!(
        findings.redact(s),
        "ssn <redacted:ssn> mail <redacted:email> here"
    );
}

// ---------- PolicyConfig branch coverage ----------------------------------

#[test]
fn policy_detect_false_short_circuits() {
    let cfg = PolicyConfig {
        detect: false,
        autoredact: true,
        log_findings: true,
    };
    let s = "card 4111 1111 1111 1111 thanks";
    let out = apply_policy(s, cfg, "contract-test");
    assert_eq!(out, s, "detect=false must pass through unchanged");
    assert!(
        matches!(out, Cow::Borrowed(_)),
        "detect=false must avoid allocation"
    );
}

#[test]
fn policy_soft_flag_returns_borrowed_original() {
    let cfg = PolicyConfig {
        detect: true,
        autoredact: false,
        log_findings: false,
    };
    let s = "card 4111 1111 1111 1111 thanks";
    let out = apply_policy(s, cfg, "contract-test");
    assert_eq!(out, s, "soft-flag must keep the original bytes");
    assert!(
        matches!(out, Cow::Borrowed(_)),
        "soft-flag must avoid allocation"
    );
}

#[test]
fn policy_autoredact_rewrites_inline() {
    let cfg = PolicyConfig {
        detect: true,
        autoredact: true,
        log_findings: false,
    };
    let s = "card 4111 1111 1111 1111 thanks";
    let out = apply_policy(s, cfg, "contract-test");
    assert_eq!(out, "card <redacted:credit_card> thanks");
    assert!(
        matches!(out, Cow::Owned(_)),
        "autoredact must return an owned string"
    );
}

#[test]
fn policy_no_findings_returns_borrowed_even_with_autoredact() {
    let cfg = PolicyConfig {
        detect: true,
        autoredact: true,
        log_findings: false,
    };
    let s = "the quick brown fox jumps over the lazy dog";
    let out = apply_policy(s, cfg, "contract-test");
    assert_eq!(out, s);
    assert!(
        matches!(out, Cow::Borrowed(_)),
        "no findings + autoredact must still borrow"
    );
}

// ---------- Masking contract ----------------------------------------------

#[test]
fn finding_masked_value_never_leaks_full_value() {
    let s = "card 4111 1111 1111 1111 thanks";
    let findings = scan(s);
    let f: &PiiFinding = &findings.0[0];
    assert_eq!(f.kind, PiiKind::CreditCard);
    assert!(
        f.masked_value.contains('*'),
        "masked value must contain mask characters: {f:?}"
    );
    assert_ne!(
        f.masked_value, "4111 1111 1111 1111",
        "masked value must not equal the raw match"
    );
}
