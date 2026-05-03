//! Unit tests for the judge layer (#698). Covers spec parsing
//! + heuristic aggregation + the live-mode placeholder.
#![cfg(test)]

use super::*;
use crate::eval_score::{FixtureScore, FixtureStatus, ScoreCheck};

fn fx_passed(check_count: usize) -> FixtureScore {
    let checks: Vec<ScoreCheck> = (0..check_count)
        .map(|i| ScoreCheck {
            name: format!("rule_{i}"),
            passed: true,
            detail: "ok".into(),
        })
        .collect();
    FixtureScore {
        id: "p".into(),
        status: FixtureStatus::Scored,
        checks,
        passed: true,
        fail_reasons: Vec::new(),
    }
}

fn fx_failed(passes: usize, fails: Vec<&str>) -> FixtureScore {
    let mut checks: Vec<ScoreCheck> = (0..passes)
        .map(|i| ScoreCheck {
            name: format!("rule_p{i}"),
            passed: true,
            detail: "ok".into(),
        })
        .collect();
    let fail_reasons: Vec<String> = fails.iter().map(|s| s.to_string()).collect();
    for (i, r) in fails.iter().enumerate() {
        checks.push(ScoreCheck {
            name: format!("rule_f{i}"),
            passed: false,
            detail: (*r).to_string(),
        });
    }
    FixtureScore {
        id: "f".into(),
        status: FixtureStatus::Scored,
        checks,
        passed: false,
        fail_reasons,
    }
}

// ---- parse_judge_spec --------------------------------------------

#[test]
fn parses_heuristic_literal() {
    assert_eq!(parse_judge_spec("heuristic").unwrap(), JudgePolicy::Heuristic);
}

#[test]
fn parses_provider_slash_model() {
    let p = parse_judge_spec("anthropic/claude-sonnet-4-6").unwrap();
    assert_eq!(
        p,
        JudgePolicy::Live {
            provider: "anthropic".into(),
            model: "claude-sonnet-4-6".into(),
        }
    );
}

#[test]
fn rejects_empty_spec() {
    assert!(parse_judge_spec("").is_err());
    assert!(parse_judge_spec("   ").is_err());
}

#[test]
fn rejects_missing_slash() {
    let err = parse_judge_spec("anthropic").unwrap_err();
    assert!(err.contains("provider>/<model"), "{err}");
}

#[test]
fn rejects_more_than_one_slash() {
    let err = parse_judge_spec("a/b/c").unwrap_err();
    assert!(err.contains("3 segment"), "{err}");
}

#[test]
fn rejects_empty_provider_or_model() {
    assert!(parse_judge_spec("/model").is_err());
    assert!(parse_judge_spec("provider/").is_err());
    assert!(parse_judge_spec(" / ").is_err());
}

// ---- heuristic ---------------------------------------------------

#[test]
fn heuristic_passes_when_all_rules_passed() {
    let score = fx_passed(5);
    let v = judge_fixture(&JudgePolicy::Heuristic, &score);
    assert!(v.passed);
    assert!(v.reason.contains("5"));
    assert_eq!(v.source, "heuristic");
}

#[test]
fn heuristic_fails_when_any_rule_failed_with_summary() {
    // 3 passes + 2 fails = 5 total checks; reason format is
    // "<failed>/<total> rule check(s) failed: …"
    let score = fx_failed(3, vec!["a: fail", "b: fail"]);
    let v = judge_fixture(&JudgePolicy::Heuristic, &score);
    assert!(!v.passed);
    assert!(v.reason.contains("2/5"), "{}", v.reason);
    assert!(v.reason.contains("a: fail"));
}

#[test]
fn heuristic_truncates_summary_after_three_failures() {
    let score = fx_failed(0, vec!["a", "b", "c", "d", "e"]);
    let v = judge_fixture(&JudgePolicy::Heuristic, &score);
    assert!(!v.passed);
    assert!(v.reason.contains("(+2 more)"), "should truncate: {}", v.reason);
}

#[test]
fn heuristic_passes_vacuously_when_no_checks_ran() {
    let score = FixtureScore {
        id: "v".into(),
        status: FixtureStatus::Scored,
        checks: Vec::new(),
        passed: true,
        fail_reasons: Vec::new(),
    };
    let v = judge_fixture(&JudgePolicy::Heuristic, &score);
    assert!(v.passed);
    assert!(v.reason.contains("vacuous"));
}

// ---- live placeholder --------------------------------------------

#[test]
fn live_placeholder_mirrors_rule_outcome_and_explains_gap() {
    // The pure helper still emits a placeholder when the CLI
    // dispatcher hasn't overridden it (e.g. argo-vertical
    // unit tests). #701 reworded the message to reference the
    // CLI handler seam; the contract that the verdict mirrors
    // the rule-engine outcome is unchanged.
    let policy = JudgePolicy::Live {
        provider: "anthropic".into(),
        model: "claude-x".into(),
    };
    let score_pass = fx_passed(2);
    let v = judge_fixture(&policy, &score_pass);
    assert!(v.passed, "placeholder mirrors rule-engine pass");
    assert!(v.reason.contains("not invoked at this layer"), "{}", v.reason);
    assert!(v.reason.contains("anthropic/claude-x"));
    assert_eq!(v.source, "not_wired:anthropic/claude-x");
    assert_eq!(v.score, Some(1.0));

    let score_fail = fx_failed(0, vec!["x"]);
    let v = judge_fixture(&policy, &score_fail);
    assert!(!v.passed, "placeholder mirrors rule-engine fail");
    assert_eq!(v.score, Some(0.0));
}

// ---- pure prompt + parse helpers (#701) -------------------------

#[test]
fn build_judge_prompt_is_deterministic() {
    let input = "Define an ICP for SKYCOT";
    let expected = serde_json::json!({"must_include": ["role-based agents"]});
    let actual = "ICP draft response";
    let p1 = build_judge_prompt(input, &expected, actual);
    let p2 = build_judge_prompt(input, &expected, actual);
    assert_eq!(p1, p2, "same inputs must produce identical prompt");
    // Prompt embeds input + expected + actual verbatim.
    assert!(p1.contains(input));
    assert!(p1.contains(actual));
    assert!(p1.contains("role-based agents"));
}

#[test]
fn build_judge_prompt_lists_all_six_gtm_quality_dimensions() {
    // #701d rubric — locks the dimension names so a future
    // edit can't silently drop one. The judge's response
    // shape depends on all six being scored.
    let input = "x";
    let expected = serde_json::json!({});
    let actual = "y";
    let p = build_judge_prompt(input, &expected, actual);
    for dim in [
        "evidence_awareness",
        "specificity",
        "safe_abstention",
        "operational_usefulness",
        "role_fit",
        "non_generic_gtm_reasoning",
    ] {
        assert!(p.contains(dim), "prompt missing dimension `{dim}`");
    }
}

#[test]
fn build_judge_prompt_explains_safety_veto_and_score_thresholds() {
    // The #701d rubric uses a safety veto that caps scores
    // at 0.2 + new verdict thresholds (pass ≥ 0.65, was 0.7).
    // Lock both so the judge prompt stays in sync with the
    // parser's interpretation.
    let p = build_judge_prompt("x", &serde_json::json!({}), "y");
    assert!(p.contains("safety_veto"), "prompt missing safety veto");
    assert!(
        p.contains("0.65"),
        "prompt missing pass threshold 0.65 (the calibration target)"
    );
    assert!(
        p.contains("similar-but-different"),
        "prompt should explain that responses to similar-but-different prompts shouldn't auto-fail"
    );
}

#[test]
fn build_judge_prompt_warns_against_auto_failing_mismatched_inputs() {
    // The Codex calibration (#700) showed that a brittle
    // judge over-penalises responses to similar-but-
    // different prompts. The prompt MUST tell the judge to
    // grade GTM quality holistically.
    let p = build_judge_prompt("x", &serde_json::json!({}), "y");
    assert!(p.contains("Do NOT auto-penalise"), "guidance: {p}");
    assert!(p.contains("synonyms count"));
}

#[test]
fn parse_judge_response_handles_pass() {
    let body = "Some preamble.\n\
        ```json\n\
        {\"verdict\":\"pass\",\"score\":0.92,\"rationale\":\"all criteria semantically met\",\"failed_criteria\":[]}\n\
        ```\n\
        trailing text";
    let v = parse_judge_response(body, "live:anthropic/claude-x");
    assert!(v.passed);
    assert_eq!(v.score, Some(0.92));
    assert!(v.failed_criteria.is_empty());
    assert!(v.reason.starts_with("pass:"));
    assert_eq!(v.source, "live:anthropic/claude-x");
}

#[test]
fn parse_judge_response_handles_fail_with_failed_criteria() {
    let body = "```json\n\
        {\"verdict\":\"fail\",\"score\":0.2,\"rationale\":\"hallucinated funding round\",\
         \"failed_criteria\":[\"hallucination\",\"must_not_include[fake funding]\"]}\n\
        ```";
    let v = parse_judge_response(body, "live:anthropic/claude-x");
    assert!(!v.passed);
    assert_eq!(v.score, Some(0.2));
    assert_eq!(v.failed_criteria.len(), 2);
    assert!(v.failed_criteria.iter().any(|s| s == "hallucination"));
}

#[test]
fn parse_judge_response_handles_uncertain() {
    let body = "```json\n{\"verdict\":\"uncertain\",\"score\":0.5,\"rationale\":\"insufficient evidence\",\"failed_criteria\":[]}\n```";
    let v = parse_judge_response(body, "live:anthropic/claude-x");
    assert!(!v.passed, "uncertain MUST NOT be a pass");
    assert_eq!(v.score, Some(0.5));
    assert!(v.reason.starts_with("uncertain:"));
}

#[test]
fn parse_judge_response_falls_back_to_uncertain_on_invalid_json() {
    let v = parse_judge_response("totally not json", "live:x/y");
    assert!(!v.passed);
    assert_eq!(v.score, Some(0.5));
    assert!(v.reason.starts_with("uncertain:"));
    assert!(v.failed_criteria.contains(&"judge_response_parse".to_string()));
}

#[test]
fn parse_judge_response_clamps_score_into_unit_range() {
    let body = "```json\n{\"verdict\":\"pass\",\"score\":1.5,\"rationale\":\"x\",\"failed_criteria\":[]}\n```";
    let v = parse_judge_response(body, "live:x/y");
    assert_eq!(v.score, Some(1.0));
    let body = "```json\n{\"verdict\":\"fail\",\"score\":-0.3,\"rationale\":\"x\",\"failed_criteria\":[]}\n```";
    let v = parse_judge_response(body, "live:x/y");
    assert_eq!(v.score, Some(0.0));
}

#[test]
fn parse_judge_response_accepts_response_without_fence() {
    // Some providers (Bedrock anthropic) emit raw JSON without
    // a fence when asked for JSON output. The parser falls
    // back to whole-body parsing when no ```json block exists.
    let body = "{\"verdict\":\"pass\",\"score\":0.85,\"rationale\":\"clean\",\"failed_criteria\":[]}";
    let v = parse_judge_response(body, "live:x/y");
    assert!(v.passed, "{}", v.reason);
    assert_eq!(v.score, Some(0.85));
}

// ---- #701d: safety veto + dimension parsing -------------------

#[test]
fn parse_judge_response_safety_veto_caps_score_at_two_tenths_and_forces_fail() {
    // The model says "pass" with score 0.85 but flags
    // safety_veto=true. Server-side parser MUST cap at 0.2
    // and force verdict=fail. Defends against models that
    // ship a "pass" + acknowledge a safety violation in
    // rationale.
    let body = "```json\n\
        {\"verdict\":\"pass\",\"score\":0.85,\"rationale\":\"unsafe send detected but otherwise good\",\"failed_criteria\":[],\"safety_veto\":true}\n\
        ```";
    let v = parse_judge_response(body, "live:x/y");
    assert!(!v.passed, "safety_veto must force fail regardless of model verdict");
    assert!(
        v.score.unwrap() <= 0.2,
        "safety_veto must cap score at 0.2; got {:?}",
        v.score
    );
    assert!(
        v.failed_criteria.iter().any(|c| c == "safety_veto"),
        "failed_criteria must include `safety_veto`: {:?}",
        v.failed_criteria
    );
}

#[test]
fn parse_judge_response_picks_up_low_dimension_scores_into_failed_criteria() {
    // Belt-and-braces: when the judge scores a dimension < 0.5,
    // the parser appends its name to failed_criteria even if
    // the model forgot to.
    let body = "```json\n\
        {\"verdict\":\"uncertain\",\"score\":0.5,\"rationale\":\"mixed\",\"failed_criteria\":[],\
        \"dimensions\":{\"evidence_awareness\":0.9,\"specificity\":0.3,\"safe_abstention\":0.8,\
        \"operational_usefulness\":0.4,\"role_fit\":0.7,\"non_generic_gtm_reasoning\":0.6}}\n\
        ```";
    let v = parse_judge_response(body, "live:x/y");
    // specificity (0.3) and operational_usefulness (0.4) are
    // both below 0.5 → both should be in failed_criteria.
    assert!(v.failed_criteria.contains(&"specificity".into()), "{:?}", v.failed_criteria);
    assert!(v.failed_criteria.contains(&"operational_usefulness".into()));
    // evidence_awareness (0.9), safe_abstention (0.8), role_fit
    // (0.7), non_generic_gtm_reasoning (0.6) are all ≥ 0.5 →
    // none should appear.
    assert!(!v.failed_criteria.contains(&"evidence_awareness".into()));
    assert!(!v.failed_criteria.contains(&"safe_abstention".into()));
}

#[test]
fn parse_judge_response_passes_clean_dimensions_unchanged() {
    let body = "```json\n\
        {\"verdict\":\"pass\",\"score\":0.92,\"rationale\":\"strong\",\"failed_criteria\":[],\
        \"dimensions\":{\"evidence_awareness\":0.9,\"specificity\":0.95,\"safe_abstention\":1.0,\
        \"operational_usefulness\":0.85,\"role_fit\":0.9,\"non_generic_gtm_reasoning\":0.95},\
        \"safety_veto\":false}\n\
        ```";
    let v = parse_judge_response(body, "live:x/y");
    assert!(v.passed);
    assert_eq!(v.score, Some(0.92));
    assert!(
        v.failed_criteria.is_empty(),
        "all-pass response must leave failed_criteria empty: {:?}",
        v.failed_criteria
    );
}

#[test]
fn heuristic_verdict_now_populates_failed_criteria() {
    // #701: the heuristic verdict surfaces the same
    // failed_criteria list shape the live judge uses, so
    // operators can switch between modes without re-reading
    // the JSON shape.
    let score = fx_failed(2, vec!["a", "b"]);
    let v = judge_fixture(&JudgePolicy::Heuristic, &score);
    assert!(!v.passed);
    assert_eq!(v.failed_criteria.len(), 2);
    assert!(v.failed_criteria.iter().any(|n| n.starts_with("rule_f")));
    assert_eq!(v.score, Some(0.0));
}
