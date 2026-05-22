//! Unit tests for `pii_detect`. All tests use the pure `scan` and
//! `apply_policy` entry points so we never mutate process-global
//! env vars (cargo runs tests on multiple threads — env mutation
//! races with sibling tests in the same process).

use super::pii_detect::{apply_policy, scan, PiiFinding, PiiFindings, PiiKind, PolicyConfig};

fn kinds(f: &PiiFindings) -> Vec<PiiKind> {
    f.0.iter().map(|x| x.kind).collect()
}

#[test]
fn ssn_detected() {
    let findings = scan("My SSN is 123-45-6789, please don't share.");
    assert_eq!(kinds(&findings), vec![PiiKind::Ssn]);

    // Shape filter rejects 000/666/9XX area, all-zero group/serial.
    assert!(scan("000-12-3456").is_empty());
    assert!(scan("666-12-3456").is_empty());
    assert!(scan("912-12-3456").is_empty());
    assert!(scan("123-00-3456").is_empty());
    assert!(scan("123-45-0000").is_empty());
}

#[test]
fn credit_card_luhn_filters() {
    // Visa test number 4111-1111-1111-1111 is Luhn-valid.
    let valid = scan("paid with 4111 1111 1111 1111 thanks");
    assert_eq!(kinds(&valid), vec![PiiKind::CreditCard]);

    // Same digit-shape, last digit flipped: Luhn-invalid → ignored.
    let invalid = scan("paid with 4111 1111 1111 1112 thanks");
    assert!(
        invalid.is_empty(),
        "Luhn-invalid CC should not be flagged, got: {invalid:?}"
    );

    // Plain 16-digit Luhn-valid (Mastercard test 5555555555554444).
    let plain = scan("digits 5555555555554444 only");
    assert_eq!(kinds(&plain), vec![PiiKind::CreditCard]);
}

#[test]
fn email_detected() {
    let findings = scan("ping alice.smith+work@example.co.uk for the answer");
    assert_eq!(kinds(&findings), vec![PiiKind::Email]);
    assert_eq!(
        &"ping alice.smith+work@example.co.uk for the answer"[findings.0[0].span.clone()],
        "alice.smith+work@example.co.uk"
    );
}

#[test]
fn us_phone_detected() {
    for s in [
        "call (415) 555-0123 today",
        "call 415-555-0123 today",
        "call 415.555.0123 today",
        "call 4155550123 today",
    ] {
        let findings = scan(s);
        assert_eq!(kinds(&findings), vec![PiiKind::UsPhone], "input was {s:?}");
    }
}

#[test]
fn international_prefixed_phone_detected() {
    // +CC-prefixed shapes that the strict 3-3-4 US regex misses —
    // pinned per `argo-security-pii-detect-broaden-phone-and-address`.
    for s in [
        "reach me at +1-555-0123 today",
        "reach me at +1 555 0123 today",
        "reach me at +1.555.0123 today",
        "reach me at +1-555-555-0123 today",
        "reach me at +44 20 7946 0958 today",
    ] {
        let findings = scan(s);
        assert_eq!(kinds(&findings), vec![PiiKind::UsPhone], "input was {s:?}");
    }
}

#[test]
fn year_like_four_digit_is_not_phone() {
    // Negative locked by the brief: a 4-digit year must never be
    // flagged as a phone number.
    let findings = scan("shipped in 2026 with no PII");
    assert!(findings.is_empty(), "year flagged as phone: {findings:?}");
}

#[test]
fn street_address_detected() {
    let s = "we're at 123 Main St for the meeting";
    let findings = scan(s);
    assert_eq!(kinds(&findings), vec![PiiKind::StreetAddress]);
    let redacted = findings.redact(s);
    assert_eq!(
        redacted,
        "we're at <redacted:street_address> for the meeting"
    );
}

#[test]
fn multi_word_street_address_detected() {
    let s = "ship to 456 North Main Ave please";
    let findings = scan(s);
    assert_eq!(kinds(&findings), vec![PiiKind::StreetAddress]);
}

#[test]
fn bare_integer_is_not_street_address() {
    // Negative locked by the brief: a digit run without `<space>
    // <name-words> <suffix>` must not match.
    let findings = scan("see line 123 of the report");
    assert!(
        findings.is_empty(),
        "bare integer flagged as address: {findings:?}"
    );
}

#[test]
fn lone_street_suffix_is_not_street_address() {
    // Negative locked by the brief: the suffix `St` on its own (no
    // digit prefix + name words) must not match.
    let findings = scan("turn left at St Andrews and stop");
    assert!(
        findings.is_empty(),
        "lone suffix flagged as address: {findings:?}"
    );
}

#[test]
fn iban_detected() {
    // GB82 WEST 1234 5698 7654 32 is a canonical IBAN test value.
    let findings = scan("transfer to GB82WEST12345698765432 today");
    assert_eq!(kinds(&findings), vec![PiiKind::Iban]);

    // Wrong checksum → mod-97 rejects.
    let bad = scan("transfer to GB99WEST12345698765432 today");
    assert!(bad.is_empty(), "bad checksum IBAN should not flag: {bad:?}");
}

#[test]
fn clean_string_yields_no_findings() {
    let findings = scan("the quick brown fox jumps over the lazy dog");
    assert!(findings.is_empty(), "clean string flagged: {findings:?}");

    // A bare 10-digit number shaped like a phone is the closest
    // false-positive risk worth pinning; it IS flagged (we accept that
    // as a soft-flag default — see brief notes).
    let phoney = scan("the order id is 4155550123 only");
    assert_eq!(kinds(&phoney), vec![PiiKind::UsPhone]);
}

#[test]
fn redact_produces_masked_output() {
    // Single-finding redaction.
    let one = scan("ssn 123-45-6789 here");
    assert_eq!(
        one.redact("ssn 123-45-6789 here"),
        "ssn <redacted:ssn> here"
    );

    // Multi-finding ordered redaction.
    let s = "ssn 123-45-6789 mail bob@example.com";
    let multi = scan(s);
    assert_eq!(kinds(&multi), vec![PiiKind::Ssn, PiiKind::Email]);
    assert_eq!(multi.redact(s), "ssn <redacted:ssn> mail <redacted:email>");

    // Empty findings → identity.
    let empty = PiiFindings::default();
    assert_eq!(empty.redact("nothing here"), "nothing here");

    // Findings carry a masked label safe to log.
    let f = &one.0[0];
    assert!(f.masked_value.contains('*'), "expected mask: {f:?}");
    assert_ne!(f.masked_value, "123-45-6789");
}

// ---- Policy / env-knob coverage (acceptance criteria 3 + 4) ----
//
// Exercised via `apply_policy` with explicit `PolicyConfig`, which
// is the same code path `scan_and_maybe_redact` reaches via env.

#[test]
fn detect_disabled_short_circuits() {
    let cfg = PolicyConfig {
        detect: false,
        autoredact: true,
        log_findings: false,
    };
    let s = "ssn 123-45-6789 here";
    let out = apply_policy(s, cfg, "test");
    assert_eq!(out, s, "detect=false should pass through unchanged");
    assert!(matches!(out, std::borrow::Cow::Borrowed(_)));
}

#[test]
fn autoredact_replaces_inline() {
    let cfg = PolicyConfig {
        detect: true,
        autoredact: true,
        log_findings: false,
    };
    let s = "card 4111 1111 1111 1111 thanks";
    let out = apply_policy(s, cfg, "test");
    assert_eq!(out, "card <redacted:credit_card> thanks");

    // detect=true, autoredact=false → original returned, but findings
    // would be logged (we silence logs here).
    let cfg_softflag = PolicyConfig {
        detect: true,
        autoredact: false,
        log_findings: false,
    };
    let out2 = apply_policy(s, cfg_softflag, "test");
    assert_eq!(out2, s);
    assert!(matches!(out2, std::borrow::Cow::Borrowed(_)));
}

#[test]
fn finding_has_span_kind_mask() {
    let s = "card 4111 1111 1111 1111 thanks";
    let findings = scan(s);
    let f: &PiiFinding = &findings.0[0];
    assert_eq!(f.kind, PiiKind::CreditCard);
    assert_eq!(&s[f.span.clone()], "4111 1111 1111 1111");
    assert!(f.masked_value.starts_with("41"));
    assert!(f.masked_value.ends_with("11"));
}
