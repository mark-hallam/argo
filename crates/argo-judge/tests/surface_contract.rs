//! Public-surface contract pins for `argo-judge`.
//!
//! Authored by `eval-argo-judge-crate-surface` (claim-4/forge-judge
//! substrate audit, 2026-05-24). The crate is mission-relevant because
//! learned-skill review, honesty evals, and forge promotion gates depend
//! on this trait + rubric surface remaining stable.

#![allow(dead_code)]

use argo_judge::{
    load_fixtures, persist_judge_record, run_disagreement_eval, verdict_from_scores,
    BucketBreakdown, CaseMismatch, Check, DisagreementReport, DisagreementTag, GroundTruthCheck,
    Judge, JudgeDisagreementCase, JudgeEvidence, LlmJudge, ProposedArtifact, Rubric,
    VerdictThresholds, DEFAULT_FIXTURES_DIR, DEFAULT_RUBRIC_ID, DEFAULT_RUBRIC_VERSION,
};
use argo_provider::Transport;
use argo_types::{
    ArgoResult, JudgeRecord, JudgeRecordId, JudgeSubject, JudgeVerdict, ProviderModel, SessionId,
};
use async_trait::async_trait;
use chrono::Utc;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

// ---------------------------------------------------------------------------
// 1. `lib.rs` re-export inventory: compile-fence + count sentinel.
//
// Total inventory pinned: 20 names across 5 modules.
// ---------------------------------------------------------------------------

#[test]
fn pub_use_inventory_pins_count_at_20() {
    let inventory = [
        "load_fixtures",
        "run_disagreement_eval",
        "BucketBreakdown",
        "CaseMismatch",
        "DisagreementReport",
        "DisagreementTag",
        "JudgeDisagreementCase",
        "DEFAULT_FIXTURES_DIR",
        "GroundTruthCheck",
        "JudgeEvidence",
        "ProposedArtifact",
        "verdict_from_scores",
        "Judge",
        "LlmJudge",
        "persist_judge_record",
        "Check",
        "Rubric",
        "VerdictThresholds",
        "DEFAULT_RUBRIC_ID",
        "DEFAULT_RUBRIC_VERSION",
    ];
    assert_eq!(
        inventory.len(),
        20,
        "argo-judge pub-use inventory drifted; update this contract with lib.rs"
    );

    let _: fn(&Path) -> ArgoResult<Vec<JudgeDisagreementCase>> = load_fixtures;
    let _: fn(&Rubric, &HashMap<String, f64>) -> JudgeVerdict = verdict_from_scores;
    let _: fn(Arc<dyn Transport>, ProviderModel, Rubric) -> LlmJudge = LlmJudge::new;
    let _: fn(Arc<dyn Transport>, ProviderModel) -> LlmJudge = LlmJudge::with_default_rubric;
    let _: for<'a> fn(&'a LlmJudge) -> &'a Rubric = LlmJudge::rubric;
    assert!(DEFAULT_FIXTURES_DIR.ends_with("fixtures/disagreement"));
    assert_eq!(DEFAULT_RUBRIC_ID, "argo:default");
    assert_eq!(DEFAULT_RUBRIC_VERSION, "0.1.0");

    let _persist_fn = persist_judge_record;
}

// ---------------------------------------------------------------------------
// 2. Judge trait shape + verdict round-trips.
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct ScriptedJudge {
    verdicts_by_id: HashMap<String, JudgeVerdict>,
    fallback: JudgeVerdict,
}

impl ScriptedJudge {
    fn mirroring(cases: &[JudgeDisagreementCase]) -> Self {
        let verdicts_by_id = cases
            .iter()
            .map(|case| {
                (
                    case.id.clone(),
                    verdict_for_human_label(&case.human_verdict),
                )
            })
            .collect();
        Self {
            verdicts_by_id,
            fallback: JudgeVerdict::Success,
        }
    }
}

#[async_trait]
impl Judge for ScriptedJudge {
    async fn adjudicate(
        &self,
        session_id: SessionId,
        subject: JudgeSubject,
        evidence: JudgeEvidence,
    ) -> ArgoResult<JudgeRecord> {
        let id = match &subject {
            JudgeSubject::SkillCreate { name, .. } => name.clone(),
            _ => String::new(),
        };
        assert_eq!(evidence.evidence_hashes().len(), 6);
        let verdict = self
            .verdicts_by_id
            .get(&id)
            .cloned()
            .unwrap_or_else(|| self.fallback.clone());
        Ok(JudgeRecord {
            id: JudgeRecordId::new(),
            session_id,
            subject,
            verdict,
            judge_model: ProviderModel {
                provider: "surface-contract".into(),
                model: "scripted".into(),
            },
            rubric_id: DEFAULT_RUBRIC_ID.into(),
            rubric_version: DEFAULT_RUBRIC_VERSION.into(),
            evidence_hashes: evidence.evidence_hashes(),
            created_at: Utc::now(),
        })
    }
}

fn verdict_for_human_label(label: &str) -> JudgeVerdict {
    match label {
        "allow" => JudgeVerdict::Success,
        "block" => JudgeVerdict::Failure {
            reason: "fixture expects block".into(),
        },
        "partial" => JudgeVerdict::Partial {
            reason: "fixture expects partial".into(),
        },
        other => panic!("unknown fixture human_verdict label: {other}"),
    }
}

#[test]
fn judge_trait_is_object_safe() {
    fn assert_object_safe(_judge: &dyn Judge) {}
    let judge = ScriptedJudge {
        verdicts_by_id: HashMap::new(),
        fallback: JudgeVerdict::Success,
    };
    assert_object_safe(&judge);
}

#[tokio::test]
async fn judge_trait_shape_returns_persisted_record_result() {
    let judge = ScriptedJudge {
        verdicts_by_id: HashMap::new(),
        fallback: JudgeVerdict::Failure {
            reason: "scripted refusal".into(),
        },
    };
    let record = judge
        .adjudicate(
            SessionId::new(),
            JudgeSubject::DelegateOutcome {
                delegate_session: SessionId::new(),
            },
            sample_evidence(),
        )
        .await
        .expect("scripted judge returns a record");
    assert!(matches!(record.verdict, JudgeVerdict::Failure { .. }));
    assert_eq!(record.rubric_id, DEFAULT_RUBRIC_ID);
    assert_eq!(record.evidence_hashes.len(), 6);
}

#[test]
fn verdict_variants_round_trip_with_stable_serde_tags() {
    let cases = [
        JudgeVerdict::Success,
        JudgeVerdict::Partial {
            reason: "borderline evidence".into(),
        },
        JudgeVerdict::Failure {
            reason: "missing evidence".into(),
        },
    ];
    for verdict in cases {
        let json = serde_json::to_string(&verdict).expect("JudgeVerdict Serialize");
        let back: JudgeVerdict = serde_json::from_str(&json).expect("JudgeVerdict Deserialize");
        match (verdict, back) {
            (JudgeVerdict::Success, JudgeVerdict::Success) => {}
            (JudgeVerdict::Partial { reason: a }, JudgeVerdict::Partial { reason: b }) => {
                assert_eq!(a, b);
                assert!(json.contains("partial"));
            }
            (JudgeVerdict::Failure { reason: a }, JudgeVerdict::Failure { reason: b }) => {
                assert_eq!(a, b);
                assert!(json.contains("failure"));
            }
            (expected, actual) => panic!("serde changed verdict shape: {expected:?} -> {actual:?}"),
        }
    }
}

// ---------------------------------------------------------------------------
// 3. Rubric + evidence surface.
// ---------------------------------------------------------------------------

#[test]
fn default_rubric_shape_and_thresholds_are_pinned() {
    let rubric = Rubric::default_rubric();
    rubric.validate().expect("default rubric validates");
    assert_eq!(rubric.id, DEFAULT_RUBRIC_ID);
    assert_eq!(rubric.version, DEFAULT_RUBRIC_VERSION);
    assert_eq!(
        rubric
            .checks
            .iter()
            .map(|check| check.id.as_str())
            .collect::<Vec<_>>(),
        [
            "task_completed",
            "tool_calls_grounded",
            "skill_quality",
            "side_effects",
        ]
    );
    assert_eq!(rubric.checks.len(), 4);
    assert!((rubric.verdict_thresholds.success - 0.8).abs() < f64::EPSILON);
    assert!((rubric.verdict_thresholds.partial - 0.5).abs() < f64::EPSILON);

    let json = serde_json::to_string(&rubric).expect("Rubric Serialize");
    let back: Rubric = serde_json::from_str(&json).expect("Rubric Deserialize");
    assert_eq!(back.id, rubric.id);
    assert_eq!(back.version, rubric.version);
    assert_eq!(back.checks.len(), rubric.checks.len());
    assert_eq!(
        back.verdict_thresholds.success,
        rubric.verdict_thresholds.success
    );
}

#[test]
fn rubric_thresholds_map_scores_to_success_partial_and_failure() {
    let rubric = Rubric::default_rubric();
    let success = scores(&[
        ("task_completed", 1.0),
        ("tool_calls_grounded", 1.0),
        ("skill_quality", 0.9),
        ("side_effects", 1.0),
    ]);
    assert!(matches!(
        verdict_from_scores(&rubric, &success),
        JudgeVerdict::Success
    ));

    let partial = scores(&[
        ("task_completed", 0.6),
        ("tool_calls_grounded", 0.6),
        ("skill_quality", 0.6),
        ("side_effects", 0.6),
    ]);
    assert!(matches!(
        verdict_from_scores(&rubric, &partial),
        JudgeVerdict::Partial { .. }
    ));

    let missing = scores(&[
        ("task_completed", 1.0),
        ("tool_calls_grounded", 1.0),
        ("skill_quality", 1.0),
    ]);
    let verdict = verdict_from_scores(&rubric, &missing);
    let JudgeVerdict::Failure { reason } = verdict else {
        panic!("missing rubric check must fail closed");
    };
    assert!(reason.contains("side_effects"));
}

#[test]
fn rubric_struct_literal_and_yaml_loader_surface_are_stable() {
    let rubric = Rubric {
        id: "custom".into(),
        version: "1.2.3".into(),
        checks: vec![Check {
            id: "grounded".into(),
            weight: 1.0,
            prompt: "Is the answer grounded?".into(),
        }],
        verdict_thresholds: VerdictThresholds {
            success: 0.9,
            partial: 0.4,
        },
    };
    rubric.validate().expect("custom rubric validates");
    let yaml = serde_yaml::to_string(&rubric).expect("Rubric YAML serialize");
    let back = Rubric::from_yaml(&yaml).expect("Rubric::from_yaml");
    assert_eq!(back.id, "custom");
    assert_eq!(back.checks[0].id, "grounded");
}

#[test]
fn judge_evidence_and_proposed_artifact_round_trip() {
    let evidence = sample_evidence();
    let json = serde_json::to_string(&evidence).expect("JudgeEvidence Serialize");
    assert!(json.contains("skill_create"));
    assert!(json.contains("cargo_test"));
    let back: JudgeEvidence = serde_json::from_str(&json).expect("JudgeEvidence Deserialize");
    assert_eq!(back.ground_truth.len(), 1);
    assert_eq!(back.evidence_hashes(), evidence.evidence_hashes());
}

fn sample_evidence() -> JudgeEvidence {
    JudgeEvidence {
        task_statement: "create a reusable review skill".into(),
        agent_output: "wrote the skill and tests".into(),
        tool_calls: Vec::new(),
        tool_results: Vec::new(),
        proposed_artifact: ProposedArtifact::SkillCreate {
            name: "review-pr".into(),
            body: "# Skill\n\nCheck diffs and tests.".into(),
        },
        ground_truth: vec![GroundTruthCheck {
            id: "cargo_test".into(),
            passed: true,
            note: "targeted tests passed".into(),
        }],
    }
}

fn scores(items: &[(&str, f64)]) -> HashMap<String, f64> {
    items
        .iter()
        .map(|(key, value)| ((*key).to_string(), *value))
        .collect()
}

// ---------------------------------------------------------------------------
// 4. Fixture-backed disagreement/evidence path.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn real_disagreement_fixtures_run_through_judge_trait_surface() {
    let fixture_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/disagreement");
    let cases = load_fixtures(&fixture_dir).expect("load real disagreement fixtures");
    assert_eq!(cases.len(), 30, "fixture count drifted");
    assert!(
        cases
            .iter()
            .any(|case| case.tag == DisagreementTag::ClearAllow),
        "clear-allow bucket missing"
    );
    assert!(
        cases
            .iter()
            .any(|case| case.tag == DisagreementTag::ClearBlock),
        "clear-block bucket missing"
    );
    assert!(
        cases
            .iter()
            .any(|case| case.tag == DisagreementTag::Contentious),
        "contentious bucket missing"
    );

    let judge = ScriptedJudge::mirroring(&cases);
    let report = run_disagreement_eval(&judge, &cases)
        .await
        .expect("run fixture-backed disagreement eval");
    let _: &DisagreementReport = &report;
    let _: Vec<CaseMismatch> = report.mismatches.clone();
    assert_eq!(report.total, 30);
    assert_eq!(report.agreed, 30);
    assert_eq!(report.disagreed, 0);
    assert!(report.mismatches.is_empty());
    assert!(report.passes_threshold(1.0));

    let clear_allow: &BucketBreakdown = report
        .buckets
        .get("clear-allow")
        .expect("clear-allow bucket present");
    assert!(clear_allow.total > 0);
    assert_eq!(clear_allow.total, clear_allow.agreed);
}
