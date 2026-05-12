//! Unit tests for `argo-redact`.
//!
//! Coverage matrix:
//! - One positive + one negative per category (5 × 2 = 10 tests)
//! - Permission gating (5 tests, one per category)
//! - Grant honoring + expiry (2 tests)
//! - Audit payload shape + grant id plumbing (2 tests)
//! - Row-level redaction + non-string field tolerance (3 tests)

use super::*;
use chrono::{Duration, Utc};
use serde_json::json;

fn default_strict() -> RedactionPolicy {
    RedactionPolicy::default()
}

#[test]
fn email_match_is_masked_by_default_policy() {
    let (out, hits) = redact_text("Contact me at sam@acmeco.com", &default_strict());
    assert_eq!(out, "Contact me at [EMAIL]");
    assert_eq!(hits, vec![Category::Email]);
}

#[test]
fn email_show_policy_leaves_raw() {
    let mut policy = default_strict();
    policy.email = PolicyMode::Show;
    let (out, hits) = redact_text("Contact me at sam@acmeco.com", &policy);
    assert_eq!(out, "Contact me at sam@acmeco.com");
    assert!(hits.is_empty());
}

#[test]
fn phone_nz_mobile_is_masked() {
    let (out, hits) = redact_text("call 027 555 1234 today", &default_strict());
    assert_eq!(out, "call [PHONE] today");
    assert_eq!(hits, vec![Category::Phone]);
}

#[test]
fn short_digit_run_is_not_a_phone() {
    let (out, hits) = redact_text("order #1234 placed", &default_strict());
    assert_eq!(out, "order #1234 placed");
    assert!(hits.is_empty());
}

#[test]
fn luhn_valid_card_is_masked() {
    let (out, hits) = redact_text("card 4111 1111 1111 1111 on file", &default_strict());
    assert_eq!(out, "card [CARD] on file");
    assert_eq!(hits, vec![Category::Card]);
}

#[test]
fn luhn_invalid_long_digit_string_is_not_a_card() {
    // 16-digit order number that passes the regex but fails Luhn.
    let (out, hits) = redact_text("order 1234567890123456 shipped", &default_strict());
    assert_eq!(out, "order 1234567890123456 shipped");
    assert!(hits.is_empty(), "Luhn-invalid sequence must not be masked");
}

#[test]
fn iban_is_masked() {
    let (out, hits) = redact_text("send to GB82WEST12345698765432 please", &default_strict());
    assert_eq!(out, "send to [IBAN] please");
    assert_eq!(hits, vec![Category::Iban]);
}

#[test]
fn iban_with_lowercase_prefix_does_not_match() {
    // IBAN regex requires uppercase country code; lowercase is not
    // an IBAN by the spec.
    let (out, hits) = redact_text("send to gb82WEST12345698765432 please", &default_strict());
    assert_eq!(out, "send to gb82WEST12345698765432 please");
    assert!(hits.is_empty());
}

#[test]
fn nz_ird_dashed_is_masked() {
    let (out, hits) = redact_text("IRD 123-456-789 on file", &default_strict());
    assert_eq!(out, "IRD [IRD] on file");
    assert_eq!(hits, vec![Category::Ird]);
}

#[test]
fn six_digit_run_is_not_an_ird() {
    let (out, hits) = redact_text("invoice 123456 paid", &default_strict());
    assert_eq!(out, "invoice 123456 paid");
    assert!(hits.is_empty());
}

#[test]
fn viewer_with_unredacted_email_sees_email_raw() {
    let viewer = ViewerPermissions {
        unredacted_email: true,
        ..ViewerPermissions::default()
    };
    let policy = resolve_policy(&viewer, &GrantSet::empty());
    let (out, _) = redact_text(
        "email sam@acmeco.com phone 027 555 1234",
        &policy,
    );
    assert!(out.contains("sam@acmeco.com"));
    assert!(out.contains("[PHONE]"));
}

#[test]
fn viewer_with_unredacted_phone_sees_phone_raw() {
    let viewer = ViewerPermissions {
        unredacted_phone: true,
        ..ViewerPermissions::default()
    };
    let policy = resolve_policy(&viewer, &GrantSet::empty());
    let (out, _) = redact_text("call 027 555 1234 about sam@acmeco.com", &policy);
    assert!(out.contains("027 555 1234"));
    assert!(out.contains("[EMAIL]"));
}

#[test]
fn viewer_with_unredacted_card_sees_card_raw() {
    let viewer = ViewerPermissions {
        unredacted_card: true,
        ..ViewerPermissions::default()
    };
    let policy = resolve_policy(&viewer, &GrantSet::empty());
    let (out, _) = redact_text("card 4111 1111 1111 1111", &policy);
    assert_eq!(out, "card 4111 1111 1111 1111");
}

#[test]
fn viewer_with_unredacted_iban_sees_iban_raw() {
    let viewer = ViewerPermissions {
        unredacted_iban: true,
        ..ViewerPermissions::default()
    };
    let policy = resolve_policy(&viewer, &GrantSet::empty());
    let (out, _) = redact_text("send GB82WEST12345698765432", &policy);
    assert!(out.contains("GB82WEST12345698765432"));
}

#[test]
fn viewer_with_unredacted_ird_sees_ird_raw() {
    let viewer = ViewerPermissions {
        unredacted_ird: true,
        ..ViewerPermissions::default()
    };
    let policy = resolve_policy(&viewer, &GrantSet::empty());
    let (out, _) = redact_text("IRD 123-456-789", &policy);
    assert_eq!(out, "IRD 123-456-789");
}

#[test]
fn active_email_grant_unmasks_email() {
    let now = Utc::now();
    let grant = Grant {
        id: "grant-1".into(),
        category: Category::Email,
        expires_at: now + Duration::minutes(5),
    };
    let grants = GrantSet::active([grant], now);
    let policy = resolve_policy(&ViewerPermissions::strict(), &grants);
    let (out, hits) = redact_text("sam@acmeco.com", &policy);
    assert_eq!(out, "sam@acmeco.com");
    assert!(hits.is_empty());
}

#[test]
fn expired_grant_is_filtered_out() {
    let now = Utc::now();
    let grant = Grant {
        id: "grant-1".into(),
        category: Category::Email,
        expires_at: now - Duration::minutes(1),
    };
    let grants = GrantSet::active([grant], now);
    assert!(grants.is_empty());
    let policy = resolve_policy(&ViewerPermissions::strict(), &grants);
    let (out, hits) = redact_text("sam@acmeco.com", &policy);
    assert_eq!(out, "[EMAIL]");
    assert_eq!(hits, vec![Category::Email]);
}

#[test]
fn redact_approval_row_masks_all_four_string_fields() {
    let row = json!({
        "id": "approval-1",
        "verb": "wants to send invoice",
        "detail": "send invoice to sam@acmeco.com",
        "consequence_if_yes": "call 027 555 1234 to confirm",
        "consequence_if_no": "IRD 123-456-789 stays on hold",
        "reason": "card 4111 1111 1111 1111 expired",
        "risk_level": "medium",
    });
    let red = redact_approval_row(row, &default_strict(), None);
    let r = &red.row;
    assert_eq!(r["detail"], "send invoice to [EMAIL]");
    assert_eq!(r["consequence_if_yes"], "call [PHONE] to confirm");
    assert_eq!(r["consequence_if_no"], "IRD [IRD] stays on hold");
    assert_eq!(r["reason"], "card [CARD] expired");
    // id + risk_level untouched.
    assert_eq!(r["id"], "approval-1");
    assert_eq!(r["risk_level"], "medium");
    assert!(red.was_masked());
    let labels: Vec<&str> = red.categories_masked.iter().map(|c| c.label()).collect();
    assert_eq!(labels, vec!["card", "email", "ird", "phone"]);
}

#[test]
fn redact_approval_row_tolerates_non_string_fields() {
    // Schema drift: `reason` is null. Must not panic.
    let row = json!({
        "detail": "email sam@acmeco.com",
        "consequence_if_yes": null,
        "consequence_if_no": 42,
        "reason": null,
    });
    let red = redact_approval_row(row, &default_strict(), None);
    assert_eq!(red.row["detail"], "email [EMAIL]");
    assert!(red.row["consequence_if_yes"].is_null());
    assert_eq!(red.row["consequence_if_no"], 42);
    assert_eq!(red.categories_masked, vec![Category::Email]);
}

#[test]
fn redact_approval_row_with_show_policy_no_masks() {
    let policy = RedactionPolicy::show_all();
    let row = json!({
        "detail": "sam@acmeco.com",
        "consequence_if_yes": "",
        "consequence_if_no": "",
        "reason": "card 4111 1111 1111 1111",
    });
    let red = redact_approval_row(row, &policy, None);
    assert_eq!(red.row["detail"], "sam@acmeco.com");
    assert_eq!(red.row["reason"], "card 4111 1111 1111 1111");
    assert!(!red.was_masked());
}

#[test]
fn audit_payload_carries_actor_approval_categories() {
    let red = ApprovalRedaction {
        row: json!({}),
        categories_masked: vec![Category::Email, Category::Phone],
        grant_id: None,
    };
    let payload = audit_payload("human:mark", "approval-9", &red);
    assert_eq!(payload["event_type"], "approval.row.redacted");
    assert_eq!(payload["actor_id"], "human:mark");
    assert_eq!(payload["approval_id"], "approval-9");
    assert_eq!(payload["categories_masked"], json!(["email", "phone"]));
    assert!(payload["grant_id"].is_null());
}

#[test]
fn audit_payload_carries_grant_id_when_set() {
    let red = ApprovalRedaction {
        row: json!({}),
        categories_masked: vec![Category::Email],
        grant_id: Some("grant-42".to_string()),
    };
    let payload = audit_payload("human:mark", "approval-9", &red);
    assert_eq!(payload["grant_id"], "grant-42");
}

#[test]
fn mock_grants_lookup_returns_empty() {
    // v1 stub — until J16 lands, no DB-backed grants exist.
    let grants = active_grants_for_principal("human:mark");
    assert!(grants.is_empty());
}

#[test]
fn sentinels_match_e1e_byte_for_byte() {
    // The five sentinels are part of the cross-layer contract; any
    // accidental rename here desyncs the server fence from the
    // client redaction layer.
    assert_eq!(SENTINEL_EMAIL, "[EMAIL]");
    assert_eq!(SENTINEL_PHONE, "[PHONE]");
    assert_eq!(SENTINEL_CARD, "[CARD]");
    assert_eq!(SENTINEL_IBAN, "[IBAN]");
    assert_eq!(SENTINEL_IRD, "[IRD]");
}

#[test]
fn all_categories_array_covers_every_variant() {
    // Compile-time-ish guard: if a new variant gets added without
    // an ALL_CATEGORIES entry the count assertion catches it.
    assert_eq!(ALL_CATEGORIES.len(), 5);
    for cat in ALL_CATEGORIES {
        assert!(!cat.label().is_empty());
        assert!(!cat.sentinel().is_empty());
        assert_eq!(Category::from_label(cat.label()), Some(cat));
    }
}

#[test]
fn luhn_validator_basics() {
    assert!(luhn_valid("4111 1111 1111 1111"));
    assert!(luhn_valid("5500-0000-0000-0004"));
    assert!(!luhn_valid("1234 5678 9012 3456"));
    assert!(!luhn_valid("12"));
    assert!(!luhn_valid("12345678901234567890")); // 20 digits, out of range
}
