//! Safety judge — the non-negotiable hard rule from the brief.
//!
//! Any generated action whose `action_type` matches the
//! sensitive-verb pattern `(send|post|pay|publish|delete|share|message)`
//! MUST be `requires_approval = true`. The judge runs as the
//! final pipeline pass before the vertical is written to disk;
//! a violating manifest fails the pipeline with a clear error.
//!
//! This is a structural check, not an LLM call — it runs
//! offline, deterministically, and cannot be bypassed by clever
//! prompting.

use crate::error::GenesisError;
use argo_vertical::{ActionDeclaration, ActionsSection};
use std::collections::BTreeMap;

/// Sensitive verbs that imply a real-world side effect, matched as a
/// **whole `_`-delimited token** (case-insensitive). Any action_type
/// carrying one of these MUST be `requires_approval = true`.
///
/// Token-matching (not substring) is used for the short / ambiguous verbs
/// here so an unrelated word that merely *contains* one of them does not
/// false-positive (e.g. `respond` ≠ `post`, `display` ≠ `pay`). High-signal
/// dangerous stems that ARE safe to match anywhere in a compound name live
/// in [`SENSITIVE_STEMS`].
///
/// ## Synonym coverage (closes the token-exact / synonym-blind hole)
///
/// The original list (`send|post|pay|publish|delete|share|message`) was
/// token-exact + synonym-blind: `payment` ≠ `pay`, and `transfer` /
/// `remove` / `erase` / `export` / `authenticate` / `forward` went uncaught
/// — so money/destroy/exfil/credential actions surfaced autonomous +
/// low-risk and the operator-facing "what it will NOT do" contract
/// understated the danger. The four families are now covered (money,
/// destructive, egress, credential). A false-positive approval gate is
/// SAFE; a false-negative is the hole, so coverage biases toward flagging.
pub const SENSITIVE_VERBS: &[&str] = &[
    // original transmit / publish verbs
    "send", "post", "pay", "publish", "delete", "share", "message",
    // money movement (short/ambiguous → whole-token only)
    "remit", "ach", "trade", "withdraw", "deposit", "wire", "charge",
    // destructive (whole-token)
    "remove", "erase", "empty", "purge", "wipe", "clear", "flush", "destroy", "truncate", "drop",
    // egress (whole-token)
    "export", "upload", "forward", "leak", "extract", "download",
    // credential entry (whole-token)
    "authenticate", "credential", "password", "login", "signin",
];

/// High-signal sensitive **stems** matched anywhere inside a compound
/// action name (substring, case-insensitive) — so `process_payment`,
/// `TRANSFER_funds`, `EXTRACT_customer_records`, `authenticate_portal`,
/// and `empty_archived_storage` all promote to `requires_approval` even
/// though the dangerous family appears as part of a larger token. Every
/// stem here was chosen because a substring hit is almost always genuinely
/// dangerous (low benign-collision risk); short / ambiguous verbs that
/// would false-positive as substrings (e.g. `pay` → `display`, `ach` →
/// `attach`) stay token-only in [`SENSITIVE_VERBS`].
pub const SENSITIVE_STEMS: &[&str] = &[
    // money movement
    "payment", "transfer", "disburse", "payout", "remit", "chargeback", "refund", "withdraw",
    // destructive
    "truncate", "destroy", "deprovision", "purge",
    // egress
    "exfiltrat", "extract", "download",
    // credential
    "authenticate", "credential", "password",
];

/// Run the safety judge over a generated `ActionsSection`.
pub fn judge(actions: &ActionsSection) -> Result<(), GenesisError> {
    let violations = scan(&actions.actions);
    if violations.is_empty() {
        Ok(())
    } else {
        Err(GenesisError::SafetyViolation {
            offenders: violations,
        })
    }
}

/// Pure scanner — return every action_type that matches a
/// sensitive verb but isn't `requires_approval`.
#[must_use]
pub fn scan(actions: &BTreeMap<String, ActionDeclaration>) -> Vec<String> {
    let mut violations: Vec<String> = actions
        .iter()
        .filter_map(|(name, decl)| {
            if matches_sensitive_verb(name) && !decl.requires_approval {
                Some(name.clone())
            } else {
                None
            }
        })
        .collect();
    violations.sort();
    violations.dedup();
    violations
}

/// `true` when an action_type carries a sensitive verb. Two-tier match
/// (deny-by-default biases toward flagging the dangerous false-negative):
/// - a [`SENSITIVE_VERBS`] entry appearing as a whole `_`-delimited token
///   (so `respond` — substring "post" but not a token — does NOT match), OR
/// - a [`SENSITIVE_STEMS`] entry appearing **anywhere** in the name (so
///   `process_payment` / `transfer_funds` / `extract_records` are caught
///   even though the dangerous family is part of a larger token).
#[must_use]
pub fn matches_sensitive_verb(action_type: &str) -> bool {
    let lowered = action_type.to_lowercase();
    let token_match = SENSITIVE_VERBS
        .iter()
        .any(|verb| lowered.split('_').any(|tok| tok == *verb));
    token_match || SENSITIVE_STEMS.iter().any(|stem| lowered.contains(stem))
}

/// Auto-fix pass — promote every action that matches a
/// sensitive verb to `requires_approval = true`. Used by the
/// pipeline BEFORE running the judge so common cases
/// auto-correct rather than fail loudly.
pub fn auto_fix(actions: &mut ActionsSection) -> usize {
    let mut fixed = 0;
    for (name, decl) in actions.actions.iter_mut() {
        if matches_sensitive_verb(name) && !decl.requires_approval {
            decl.requires_approval = true;
            fixed += 1;
        }
    }
    fixed
}

#[cfg(test)]
mod tests {
    use super::*;
    use argo_vertical::ActionRiskLevel;

    fn decl(requires_approval: bool, risk: ActionRiskLevel) -> ActionDeclaration {
        ActionDeclaration {
            risk,
            requires_approval,
            ..ActionDeclaration::default()
        }
    }

    #[test]
    fn matches_sensitive_verb_catches_each_canonical_token() {
        for verb in SENSITIVE_VERBS {
            assert!(
                matches_sensitive_verb(&format!("foo_{verb}_bar")),
                "{verb} should match"
            );
        }
    }

    #[test]
    fn matches_sensitive_verb_is_case_insensitive() {
        assert!(matches_sensitive_verb("Send_Email"));
        assert!(matches_sensitive_verb("POST_LISTING"));
    }

    #[test]
    fn matches_sensitive_verb_does_not_false_positive_on_benign_substring() {
        // `respond` contains "post" only as a substring of a token, not a
        // whole token, and carries no danger stem → must NOT flag.
        assert!(!matches_sensitive_verb("respond_to_user"));
        // Control: ordinary support / customer-success actions stay
        // autonomous (no over-flagging of benign work).
        assert!(!matches_sensitive_verb("triage_ticket"));
        assert!(!matches_sensitive_verb("draft_response"));
        assert!(!matches_sensitive_verb("summarize_thread"));
        assert!(!matches_sensitive_verb("tag_inventory"));
        assert!(!matches_sensitive_verb("read_inbox"));
        assert!(!matches_sensitive_verb("attach_file"));
        assert!(!matches_sensitive_verb("display_dashboard"));
        assert!(!matches_sensitive_verb("reach_out_internally"));
    }

    #[test]
    fn matches_sensitive_verb_catches_synonym_families_the_old_list_missed() {
        // The live stress-test false-negatives — money / destroy / egress /
        // credential synonyms that surfaced AUTONOMOUS + low-risk before.
        for name in [
            "process_payment",
            "transfer_funds",
            "extract_customer_records",
            "upload_to_partner_system",
            "authenticate_portal",
            "empty_archived_storage",
            "issue_refund",
            "remove_account_data",
            "erase_records",
            "export_customer_list",
            "forward_to_external",
            "remit_invoice",
        ] {
            assert!(
                matches_sensitive_verb(name),
                "{name} carries a dangerous family and must require approval"
            );
        }
    }

    #[test]
    fn matches_sensitive_verb_catches_credential_password_action() {
        // Security posture change: an action that handles a password /
        // credential is sensitive (it can exfiltrate or mis-use a secret),
        // so it now requires approval — the old token-exact list let
        // `password_check` through.
        assert!(matches_sensitive_verb("password_check"));
        assert!(matches_sensitive_verb("store_credential"));
    }

    #[test]
    fn judge_passes_when_every_sensitive_action_requires_approval() {
        let mut actions = BTreeMap::new();
        actions.insert("send_email".to_string(), decl(true, ActionRiskLevel::High));
        actions.insert(
            "post_listing".to_string(),
            decl(true, ActionRiskLevel::Medium),
        );
        actions.insert(
            "verify_price".to_string(),
            decl(false, ActionRiskLevel::Low),
        );
        let section = ActionsSection { actions };
        assert!(judge(&section).is_ok());
    }

    #[test]
    fn judge_rejects_when_send_action_is_autonomous_safe() {
        let mut actions = BTreeMap::new();
        actions.insert(
            "send_email".to_string(),
            decl(false, ActionRiskLevel::Medium),
        );
        let section = ActionsSection { actions };
        let err = judge(&section).expect_err("autonomous send must fail");
        assert!(err.to_string().contains("send_email"), "got: {err}");
    }

    #[test]
    fn judge_lists_every_offender_in_one_error() {
        let mut actions = BTreeMap::new();
        actions.insert(
            "send_email".to_string(),
            decl(false, ActionRiskLevel::Medium),
        );
        actions.insert(
            "post_listing".to_string(),
            decl(false, ActionRiskLevel::Medium),
        );
        actions.insert(
            "delete_record".to_string(),
            decl(false, ActionRiskLevel::High),
        );
        actions.insert(
            "verify_price".to_string(),
            decl(false, ActionRiskLevel::Low),
        );
        let section = ActionsSection { actions };
        let err = judge(&section).expect_err("3 offenders");
        let msg = err.to_string();
        assert!(msg.contains("send_email"), "got: {msg}");
        assert!(msg.contains("post_listing"), "got: {msg}");
        assert!(msg.contains("delete_record"), "got: {msg}");
        assert!(!msg.contains("verify_price"), "got: {msg}");
    }

    #[test]
    fn auto_fix_promotes_every_sensitive_action() {
        let mut actions = BTreeMap::new();
        actions.insert(
            "send_email".to_string(),
            decl(false, ActionRiskLevel::Medium),
        );
        actions.insert(
            "post_listing".to_string(),
            decl(false, ActionRiskLevel::Medium),
        );
        actions.insert(
            "verify_price".to_string(),
            decl(false, ActionRiskLevel::Low),
        );
        let mut section = ActionsSection { actions };
        let fixed = auto_fix(&mut section);
        assert_eq!(fixed, 2, "two sensitive actions auto-fixed");
        assert!(judge(&section).is_ok());
        assert!(!section.actions["verify_price"].requires_approval);
    }
}
