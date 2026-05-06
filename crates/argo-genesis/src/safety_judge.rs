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

/// Sensitive verbs that imply a real-world side effect. Any
/// action_type containing one of these (case-insensitive,
/// matched as a sub-token split on `_`) MUST be
/// `requires_approval = true`.
pub const SENSITIVE_VERBS: &[&str] = &[
    "send", "post", "pay", "publish", "delete", "share", "message",
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

/// Sub-token match on the action_type. Split on `_` so we don't
/// false-positive on unrelated substrings (e.g. `respond` —
/// substring "post" but not a token).
#[must_use]
pub fn matches_sensitive_verb(action_type: &str) -> bool {
    let lowered = action_type.to_lowercase();
    SENSITIVE_VERBS
        .iter()
        .any(|verb| lowered.split('_').any(|tok| tok == *verb))
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
    fn matches_sensitive_verb_does_not_false_positive_on_substring() {
        assert!(!matches_sensitive_verb("respond_to_user"));
        assert!(!matches_sensitive_verb("password_check"));
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
