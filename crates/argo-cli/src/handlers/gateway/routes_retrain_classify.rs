//! Keyword classifier for `routes_retrain` — split out of
//! `routes_retrain_helpers` for the 500-LOC ceiling. Pure, store-free
//! heuristics that map a plain-English retrain instruction onto one of
//! the five retrain classifications + its structural change. Mounted as
//! a child of `helpers`; `classify_input` / `ClassifyOutcome` are
//! re-exported from there, `trim_for_display` is used by
//! `helpers::retrain_summary`.

use serde_json::{json, Value};

pub(crate) struct ClassifyOutcome {
    pub(crate) classification: String,
    pub(crate) plain_english: String,
    pub(crate) structural_change: Value,
    pub(crate) needs_generalization: Option<Value>,
}
pub(crate) fn classify_input(input: &str) -> ClassifyOutcome {
    let lower = input.to_ascii_lowercase();
    if let Some((from, to)) = detect_vocabulary_patch(&lower) {
        return outcome(
            "vocabulary_patch",
            format!("Replace '{from}' with '{to}'."),
            json!({ "from": from, "to": to }),
            None,
        );
    }
    if let Some(banned) = detect_action_ban(&lower) {
        return outcome(
            "action_ban",
            format!("Ban action '{banned}'."),
            json!({ "action_type": banned }),
            None,
        );
    }
    if matches_any(
        &lower,
        &[
            "always ",
            "must ",
            "must not ",
            "before ",
            "require ",
            "only when ",
        ],
    ) {
        return outcome(
            "policy_rule",
            format!("Add policy rule: {}", trim_for_display(input)),
            json!({ "predicate_json": { "raw": input }, "action_type": guess_action_type(&lower) }),
            None,
        );
    }
    if matches_any(
        &lower,
        &["you got it wrong", "next time", "incorrect", "wrong answer"],
    ) {
        return outcome(
            "skill_correction",
            format!("Correct skill output: {}", trim_for_display(input)),
            json!({ "note": input }),
            Some(json!({
                "question": "Apply to all similar inputs or only this exact case?",
                "options": ["all similar", "this case only"],
            })),
        );
    }
    outcome(
        "memory_fact",
        format!("Remember: {}", trim_for_display(input)),
        json!({ "text": input }),
        None,
    )
}
fn outcome(c: &str, p: String, s: Value, g: Option<Value>) -> ClassifyOutcome {
    ClassifyOutcome {
        classification: c.into(),
        plain_english: p,
        structural_change: s,
        needs_generalization: g,
    }
}
fn matches_any(lower: &str, keys: &[&str]) -> bool {
    keys.iter().any(|k| lower.contains(k))
}
fn detect_vocabulary_patch(lower: &str) -> Option<(String, String)> {
    let idx = lower.find(" instead of ")?;
    let head = &lower[..idx];
    let new_word = head.rsplit_once(' ').map_or(head, |(_, w)| w);
    let tail = &lower[idx + " instead of ".len()..];
    let old_word = tail.split([' ', '.', ',']).next().unwrap_or(tail);
    (!new_word.is_empty() && !old_word.is_empty())
        .then(|| (old_word.to_string(), new_word.to_string()))
}

fn detect_action_ban(lower: &str) -> Option<String> {
    for prefix in ["stop doing ", "don't ever ", "never "] {
        if let Some(idx) = lower.find(prefix) {
            let tail = &lower[idx + prefix.len()..];
            let first = tail.split([' ', '.', ',']).next().unwrap_or(tail);
            if !first.is_empty() {
                return Some(first.to_string());
            }
        }
    }
    None
}

fn guess_action_type(lower: &str) -> String {
    for kw in [
        "send_email",
        "send_sms",
        "send",
        "post",
        "pay",
        "refund",
        "schedule",
    ] {
        if lower.contains(kw) {
            return kw.into();
        }
    }
    "unspecified".into()
}

pub(super) fn trim_for_display(input: &str) -> String {
    let s = input.trim();
    if s.len() <= 120 {
        s.into()
    } else {
        format!("{}…", s.chars().take(117).collect::<String>())
    }
}
