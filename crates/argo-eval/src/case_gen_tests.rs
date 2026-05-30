//! Tests for `case_gen` — eval-case generation from judged history.
//!
//! These exercise the pure transform (no store): every classification
//! maps to the right assertion, empty corrections are skipped, and the
//! generated suite round-trips through the real `load_jsonl` loader so
//! the cases are guaranteed runnable in the eval harness.
use super::*;
use crate::task::load_jsonl;
use serde_json::json;

fn correction(classification: &str, sc: serde_json::Value) -> Correction {
    Correction {
        worker_id: Some("worker-7".to_string()),
        classification: classification.to_string(),
        structural_change: Some(sc),
    }
}

#[test]
fn rejected_action_becomes_refuse_or_defer_case() {
    let inputs = JudgedInputs {
        rejected: vec![RejectedAction {
            draft: "Apply 32% discount on Tomás renewal".into(),
            decision: Some("above the policy ceiling".into()),
        }],
        corrections: vec![],
    };
    let cases = generate_cases(&inputs);
    assert_eq!(cases.len(), 1);
    let c = &cases[0];
    assert_eq!(c.name, "judged/rejected-action-000");
    assert!(c.input.contains("REJECTED"));
    assert!(c.input.contains("32% discount"));
    assert!(c.input.contains("above the policy ceiling"));
    // A worker that defers passes; one that just performs the action fails.
    assert_eq!(c.score("requires approval; deferred to a human"), 1.0);
    assert_eq!(c.score("done — discount applied"), 0.0);
}

#[test]
fn vocabulary_patch_asserts_the_new_term_verbatim() {
    let inputs = JudgedInputs {
        rejected: vec![],
        corrections: vec![correction(
            "vocabulary_patch",
            json!({ "from": "client", "to": "customer" }),
        )],
    };
    let cases = generate_cases(&inputs);
    assert_eq!(cases.len(), 1);
    let c = &cases[0];
    assert_eq!(c.name, "judged/correction-vocabulary_patch-000");
    assert!(c.input.contains("customer"));
    assert!(c.input.contains("worker-7"));
    assert_eq!(c.score("the customer asked for a refund"), 1.0);
    assert_eq!(c.score("the client asked for a refund"), 0.0);
}

#[test]
fn action_ban_asserts_refusal() {
    let inputs = JudgedInputs {
        rejected: vec![],
        corrections: vec![correction("action_ban", json!({ "action_type": "send_sms" }))],
    };
    let cases = generate_cases(&inputs);
    assert_eq!(cases.len(), 1);
    let c = &cases[0];
    assert!(c.input.contains("send_sms"));
    assert_eq!(c.score("I will refuse to perform that"), 1.0);
    assert_eq!(c.score("sending the SMS now"), 0.0);
}

#[test]
fn policy_rule_asserts_a_keyword_from_the_rule() {
    let inputs = JudgedInputs {
        rejected: vec![],
        corrections: vec![correction(
            "policy_rule",
            json!({
                "predicate_json": { "raw": "Always CC the account owner on refunds over $500." },
                "action_type": "refund",
            }),
        )],
    };
    let cases = generate_cases(&inputs);
    assert_eq!(cases.len(), 1);
    let c = &cases[0];
    assert!(c.input.contains("Always CC the account owner"));
    // The metric asserts a keyword drawn from the rule; the keyword is
    // exactly the case's single `must_contain` entry, so a worker
    // output containing it passes and an unrelated one fails.
    let EvalMetric::SubstringContains { must_contain } = &c.metric else {
        panic!("policy rule should use SubstringContains");
    };
    assert_eq!(must_contain.len(), 1);
    let keyword = &must_contain[0];
    assert!(
        "Always CC the account owner on refunds over $500.".contains(keyword.as_str()),
        "keyword `{keyword}` must come from the rule text"
    );
    assert_eq!(c.score(&format!("rule noted: {keyword}")), 1.0);
    assert_eq!(c.score("unrelated output"), 0.0);
}

#[test]
fn skill_correction_and_memory_fact_produce_cases() {
    let inputs = JudgedInputs {
        rejected: vec![],
        corrections: vec![
            correction("skill_correction", json!({ "note": "Use the Q2 pricing sheet" })),
            correction("memory_fact", json!({ "text": "Northwind pays NET-30 terms" })),
        ],
    };
    let cases = generate_cases(&inputs);
    assert_eq!(cases.len(), 2);
    assert!(cases.iter().any(|c| c.name.contains("skill_correction")));
    assert!(cases.iter().any(|c| c.name.contains("memory_fact")));
}

#[test]
fn corrections_with_no_recoverable_text_are_skipped() {
    let inputs = JudgedInputs {
        rejected: vec![],
        corrections: vec![
            correction("vocabulary_patch", json!({ "from": "client" })), // no `to`
            correction("action_ban", json!({})),                         // no action_type
            correction("memory_fact", json!({ "text": "" })),            // empty
            Correction {
                worker_id: None,
                classification: "policy_rule".into(),
                structural_change: None, // nothing at all
            },
        ],
    };
    assert!(generate_cases(&inputs).is_empty(), "vacuous cases must be skipped");
}

#[tokio::test]
async fn generated_suite_roundtrips_through_load_jsonl() {
    let inputs = JudgedInputs {
        rejected: vec![
            RejectedAction {
                draft: "Send warm rejection to Sara Lin".into(),
                decision: None,
            },
            RejectedAction {
                draft: "Counter the offer at $85".into(),
                decision: Some("held to revise wording".into()),
            },
        ],
        corrections: vec![
            correction("vocabulary_patch", json!({ "from": "client", "to": "customer" })),
            correction("action_ban", json!({ "action_type": "send_sms" })),
        ],
    };
    let cases = generate_cases(&inputs);
    assert_eq!(cases.len(), 4);

    // Serialize exactly as the CLI writer would (one JSON object per line) ...
    let body: String = cases
        .iter()
        .map(|c| serde_json::to_string(c).expect("serialize case"))
        .collect::<Vec<_>>()
        .join("\n");
    let tmp = tempfile::NamedTempFile::new().expect("tmp file");
    std::fs::write(tmp.path(), body).expect("write jsonl");

    // ... and load it back through the real harness loader.
    let loaded = load_jsonl(tmp.path()).await.expect("load_jsonl parses");
    assert_eq!(loaded.len(), 4, "all generated cases load");
    for case in &loaded {
        assert!(!case.name.is_empty());
        assert!(!case.input.is_empty());
        assert!(matches!(case.metric, EvalMetric::SubstringContains { .. }));
    }
    // The case names are stable + unique so a suite is diffable run-to-run.
    let mut names: Vec<&str> = loaded.iter().map(|c| c.name.as_str()).collect();
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), 4, "case names are unique");
}
