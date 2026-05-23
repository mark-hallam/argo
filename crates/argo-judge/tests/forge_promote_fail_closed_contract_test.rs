//! Contract pins for the `forge promote` judge fail-CLOSED wire.
//!
//! Per the gap manifest at
//! `docs/skycot/evals/scheduled/2026-05-23/mission-vs-code-gap-manifest.md`
//! (claim-2 / claim-4 row, `argo-judge`):
//!
//! > Used by skill creation (fail-open per #936) + forge promote
//! > (fail-closed per #940).
//!
//! And per MISSION.md ("judge is consulted by forge promote
//! (fail-CLOSED — promotion blast-radius is larger)").
//!
//! The production gate lives in
//! `crates/argo-cli/src/handlers/forge_optimize_judge.rs` —
//! outside this brief's may-touch. What this file pins is the
//! **substrate contract** the gate depends on: `Judge::adjudicate`
//! returns `ArgoResult` so errors surface (fail-CLOSED signal);
//! `JudgeVerdict` has exactly three variants so the gate `match`
//! stays exhaustive; the default rubric's success/partial
//! thresholds (0.8 / 0.5) determine which borderline aggregate
//! scores land at `Partial` (rejected at the strict forge floor);
//! `tokio::time::timeout` is the timeout primitive the gate uses;
//! `JudgeSubject::SkillRefine` is the subject forge promote hands
//! to the judge and survives the trait surface unchanged.
//!
//! FINDINGS (`#[ignore]`'d below): every production caller of
//! `forge_optimize::run_with_events_and_run_id` passes
//! `judge: None`. The judge gate landed via PR #940 as an opt-in
//! seam; no follow-up brief threaded a real `LlmJudge` through
//! the three entry points. Fix-brief named in the ignored test.

use argo_judge::{Judge, Rubric};
use argo_types::{
    ArgoError, ArgoResult, JudgeRecord, JudgeRecordId, JudgeSubject, JudgeVerdict, ProviderModel,
    SessionId,
};
use async_trait::async_trait;
use chrono::Utc;
use std::collections::HashMap;
use std::time::Duration;

/// Stub that always errors on `adjudicate`. Mirrors a degraded
/// production judge (network timeout, provider 5xx, malformed
/// response). The fail-CLOSED contract: callers MUST see the
/// `Err` and refuse the downstream promote.
#[derive(Clone)]
struct FailingJudge {
    reason: &'static str,
}

#[async_trait]
impl Judge for FailingJudge {
    async fn adjudicate(
        &self,
        _session_id: SessionId,
        _subject: JudgeSubject,
        _evidence: argo_judge::JudgeEvidence,
    ) -> ArgoResult<JudgeRecord> {
        Err(ArgoError::Provider(self.reason.to_string()))
    }
}

/// Stub that returns a fixed verdict. Lets us pin the per-variant
/// mapping the production gate relies on.
#[derive(Clone)]
struct ScriptedJudge {
    verdict: JudgeVerdict,
}

#[async_trait]
impl Judge for ScriptedJudge {
    async fn adjudicate(
        &self,
        session_id: SessionId,
        subject: JudgeSubject,
        _evidence: argo_judge::JudgeEvidence,
    ) -> ArgoResult<JudgeRecord> {
        Ok(JudgeRecord {
            id: JudgeRecordId::new(),
            session_id,
            subject,
            verdict: self.verdict.clone(),
            judge_model: ProviderModel {
                provider: "stub".into(),
                model: "scripted-judge".into(),
            },
            rubric_id: "stub".into(),
            rubric_version: "0".into(),
            evidence_hashes: Vec::new(),
            created_at: Utc::now(),
        })
    }
}

/// Stub that never resolves — used to drive `tokio::time::timeout`
/// against the trait surface (the primitive the production gate
/// uses to enforce `ARGO_JUDGE_FORGE_TIMEOUT_MS`).
#[derive(Clone, Default)]
struct PendingJudge;

#[async_trait]
impl Judge for PendingJudge {
    async fn adjudicate(
        &self,
        _session_id: SessionId,
        _subject: JudgeSubject,
        _evidence: argo_judge::JudgeEvidence,
    ) -> ArgoResult<JudgeRecord> {
        std::future::pending().await
    }
}

fn empty_evidence() -> argo_judge::JudgeEvidence {
    argo_judge::JudgeEvidence {
        task_statement: "forge promote".into(),
        agent_output: String::new(),
        tool_calls: Vec::new(),
        tool_results: Vec::new(),
        proposed_artifact: argo_judge::ProposedArtifact::SkillCreate {
            name: "alpha".into(),
            body: "stub body".into(),
        },
        ground_truth: Vec::new(),
    }
}

fn skill_refine_subject() -> JudgeSubject {
    JudgeSubject::SkillRefine {
        name: "alpha".into(),
        version_from: "0.1".into(),
        version_to_hash: [0u8; 32],
    }
}

// ──────────────────────────────────────────────────────────────
// PASS 1 — verdict-variant contract
// ──────────────────────────────────────────────────────────────

#[test]
fn judge_verdict_has_exactly_three_variants_so_gate_match_is_exhaustive() {
    // Production gate's `classify_verdict` does an exhaustive
    // match. Adding a fourth variant without touching the gate
    // would refuse to compile — this test pins the variant set
    // so any future variant addition surfaces at the brief
    // level, not at first-deploy.
    fn assert_exhaustive(v: &JudgeVerdict) -> &'static str {
        match v {
            JudgeVerdict::Success => "allow",
            JudgeVerdict::Partial { .. } => "partial",
            JudgeVerdict::Failure { .. } => "reject",
        }
    }
    assert_eq!(assert_exhaustive(&JudgeVerdict::Success), "allow");
    assert_eq!(
        assert_exhaustive(&JudgeVerdict::Partial { reason: "x".into() }),
        "partial",
    );
    assert_eq!(
        assert_exhaustive(&JudgeVerdict::Failure { reason: "y".into() }),
        "reject",
    );
}

// ──────────────────────────────────────────────────────────────
// PASS 2 — Judge trait surface contract (fail-CLOSED substrate)
// ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn failing_judge_surfaces_err_not_silent_allow() {
    // The fail-CLOSED substrate: the trait method returns
    // `ArgoResult`, so a transport / provider error surfaces
    // as `Err`. The production gate maps this to
    // `PromoteDecision::JudgeUnavailable` (refuses promotion).
    // If `Judge::adjudicate` ever changed to return `JudgeRecord`
    // (no Result), a degraded judge would have to fall back to
    // a default verdict — likely Success — and silently allow.
    let judge = FailingJudge {
        reason: "network down",
    };
    let result = judge
        .adjudicate(SessionId::new(), skill_refine_subject(), empty_evidence())
        .await;
    let err = result.expect_err("FailingJudge must surface Err to its caller");
    assert!(
        format!("{err}").contains("network down"),
        "err must carry the underlying reason for audit-trail attribution: {err}",
    );
}

#[tokio::test]
async fn pending_judge_under_tokio_timeout_returns_err_elapsed() {
    // The production gate at
    // `crates/argo-cli/src/handlers/forge_optimize_judge.rs`
    // wraps `judge.adjudicate(...)` in `tokio::time::timeout` and
    // routes `Err(Elapsed)` to `JudgeUnavailable` (fail-CLOSED).
    // Pin the primitive: a hung judge IS observable as a timeout.
    let judge = PendingJudge;
    let outcome = tokio::time::timeout(
        Duration::from_millis(50),
        judge.adjudicate(SessionId::new(), skill_refine_subject(), empty_evidence()),
    )
    .await;
    assert!(
        outcome.is_err(),
        "tokio::time::timeout must surface Elapsed for hung adjudicate",
    );
}

// ──────────────────────────────────────────────────────────────
// PASS 3 — verdict → outcome mapping the gate depends on
// ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn scripted_judge_success_is_unconditional_allow() {
    // Production gate at strict floor (0.7): Success → Allow.
    // Pin via the trait surface so the substrate contract
    // (`Success` carries no qualifier; cannot be "downgraded"
    // by a stricter floor) holds independent of gate code.
    let judge = ScriptedJudge {
        verdict: JudgeVerdict::Success,
    };
    let record = judge
        .adjudicate(SessionId::new(), skill_refine_subject(), empty_evidence())
        .await
        .expect("scripted Success must Ok");
    assert!(matches!(record.verdict, JudgeVerdict::Success));
}

#[tokio::test]
async fn scripted_judge_failure_is_unconditional_reject() {
    // Failure always reduces to reject — no floor configuration
    // can override. The reason string is preserved so the
    // refused-promote audit row carries WHY.
    let judge = ScriptedJudge {
        verdict: JudgeVerdict::Failure {
            reason: "regressed on golden eval set".into(),
        },
    };
    let record = judge
        .adjudicate(SessionId::new(), skill_refine_subject(), empty_evidence())
        .await
        .expect("scripted Failure must Ok with Failure verdict");
    let JudgeVerdict::Failure { reason } = &record.verdict else {
        panic!("expected Failure, got {:?}", record.verdict);
    };
    assert!(reason.contains("regressed"), "reason={reason}");
}

#[tokio::test]
async fn scripted_judge_partial_is_strict_floor_rejected() {
    // Partial is the discriminator between forge promote
    // (strict floor 0.7 → reject) and skill creation (lenient
    // floor 0.6 → allow as draft). Pin Partial as the verdict
    // a STRICT consumer sees and rejects. The gate's mapping
    // logic lives in argo-cli; here we pin that the substrate
    // carries enough information (reason string) for an audit
    // row to record the rejection cause.
    let judge = ScriptedJudge {
        verdict: JudgeVerdict::Partial {
            reason: "ambiguous on 2/10 cases".into(),
        },
    };
    let record = judge
        .adjudicate(SessionId::new(), skill_refine_subject(), empty_evidence())
        .await
        .expect("scripted Partial must Ok with Partial verdict");
    let JudgeVerdict::Partial { reason } = &record.verdict else {
        panic!("expected Partial, got {:?}", record.verdict);
    };
    assert!(reason.contains("ambiguous"), "reason={reason}");
}

// ──────────────────────────────────────────────────────────────
// PASS 4 — default rubric thresholds (LlmJudge backbone)
// ──────────────────────────────────────────────────────────────

#[test]
fn default_rubric_thresholds_pin_promote_bar() {
    // `LlmJudge::with_default_rubric` calls `Rubric::default_rubric()`
    // which encodes the §25.2 thresholds: `success ≥ 0.8`,
    // `partial ≥ 0.5`, otherwise `Failure`. The forge gate is
    // forced strict (Partial→Reject) at the gate layer, but
    // these thresholds determine WHICH aggregate score lands a
    // borderline candidate at `Partial` (rejected) vs `Success`
    // (allowed). Pin them so a downgrade (e.g. `success: 0.6`)
    // surfaces as a test failure at the substrate, not as a
    // silent loosening of the forge promote bar.
    let r = Rubric::default_rubric();
    assert!(
        (r.verdict_thresholds.success - 0.8).abs() < 1e-9,
        "default success threshold drifted: {}",
        r.verdict_thresholds.success,
    );
    assert!(
        (r.verdict_thresholds.partial - 0.5).abs() < 1e-9,
        "default partial threshold drifted: {}",
        r.verdict_thresholds.partial,
    );
    // Verdicts must be ordered: partial ≤ success. The validator
    // enforces this; pin it here too so downstream consumers
    // don't have to re-derive the invariant.
    assert!(r.verdict_thresholds.partial <= r.verdict_thresholds.success);
}

#[test]
fn default_rubric_aggregates_borderline_score_to_partial_not_success() {
    // Borderline aggregate score 0.6 sits in `partial..success`
    // (0.5..0.8). At the strict forge floor this becomes a
    // promote refusal. Pin the verdict-for-score mapping so
    // the threshold ceiling can't drift down silently.
    let r = Rubric::default_rubric();
    // Hit a `task_completed` 0.6, weight 0.4 plus enough on
    // other checks to land aggregate exactly at 0.6.
    let scores: HashMap<String, f64> = [
        ("task_completed".to_string(), 0.6),
        ("tool_calls_grounded".to_string(), 0.6),
        ("skill_quality".to_string(), 0.6),
        ("side_effects".to_string(), 0.6),
    ]
    .into_iter()
    .collect();
    let (agg, verdict) = r.aggregate(&scores).expect("aggregate ok");
    assert!((agg - 0.6).abs() < 1e-9, "aggregate drifted: {agg}");
    assert!(
        matches!(verdict, JudgeVerdict::Partial { .. }),
        "0.6 must land at Partial (rejected at strict forge floor), got {verdict:?}",
    );
}

#[test]
fn default_rubric_aggregates_high_score_to_success() {
    let r = Rubric::default_rubric();
    let scores: HashMap<String, f64> = [
        ("task_completed".to_string(), 0.95),
        ("tool_calls_grounded".to_string(), 0.9),
        ("skill_quality".to_string(), 0.9),
        ("side_effects".to_string(), 1.0),
    ]
    .into_iter()
    .collect();
    let (_agg, verdict) = r.aggregate(&scores).expect("aggregate ok");
    assert!(
        matches!(verdict, JudgeVerdict::Success),
        "high scores must allow at strict forge floor, got {verdict:?}",
    );
}

#[test]
fn default_rubric_aggregates_low_score_to_failure() {
    let r = Rubric::default_rubric();
    let scores: HashMap<String, f64> = [
        ("task_completed".to_string(), 0.2),
        ("tool_calls_grounded".to_string(), 0.3),
        ("skill_quality".to_string(), 0.4),
        ("side_effects".to_string(), 0.5),
    ]
    .into_iter()
    .collect();
    let (_agg, verdict) = r.aggregate(&scores).expect("aggregate ok");
    assert!(
        matches!(verdict, JudgeVerdict::Failure { .. }),
        "low scores must reject unconditionally, got {verdict:?}",
    );
}

#[test]
fn default_rubric_missing_check_aggregates_to_failure_not_silent_allow() {
    // If the judge response is malformed and one of the four
    // checks is missing, the rubric must NOT default the
    // missing weight to 1.0 and round up to Success. It must
    // refuse — the production gate then writes a `judge_blocked`
    // audit row. This is the second fail-CLOSED axis (the first
    // is transport error).
    let r = Rubric::default_rubric();
    let mut scores: HashMap<String, f64> = HashMap::new();
    scores.insert("task_completed".into(), 1.0);
    scores.insert("tool_calls_grounded".into(), 1.0);
    scores.insert("skill_quality".into(), 1.0);
    // `side_effects` deliberately missing.
    let (_agg, verdict) = r.aggregate(&scores).expect("aggregate ok");
    let JudgeVerdict::Failure { reason } = &verdict else {
        panic!("missing check must reduce to Failure, got {verdict:?}");
    };
    assert!(
        reason.contains("missing checks"),
        "Failure must name the missing check for audit attribution: {reason}",
    );
    assert!(reason.contains("side_effects"));
}

// ──────────────────────────────────────────────────────────────
// PASS 5 — SkillRefine subject round-trip (forge promote's subject)
// ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn skill_refine_subject_round_trips_through_judge_record() {
    // Forge promote hands `JudgeSubject::SkillRefine` (per
    // `crates/argo-cli/src/handlers/forge_optimize_judge.rs::adjudicate_with_timeout`).
    // Pin that the field set (`name`, `version_from`,
    // `version_to_hash`) survives the trait surface unchanged
    // so the persisted judge_record row carries the promote
    // attribution.
    let subject = JudgeSubject::SkillRefine {
        name: "alpha".into(),
        version_from: "0.1".into(),
        version_to_hash: [42u8; 32],
    };
    let judge = ScriptedJudge {
        verdict: JudgeVerdict::Success,
    };
    let record = judge
        .adjudicate(SessionId::new(), subject.clone(), empty_evidence())
        .await
        .expect("Ok");
    let JudgeSubject::SkillRefine {
        name,
        version_from,
        version_to_hash,
    } = &record.subject
    else {
        panic!("subject variant changed: {:?}", record.subject);
    };
    assert_eq!(name, "alpha");
    assert_eq!(version_from, "0.1");
    assert_eq!(version_to_hash, &[42u8; 32]);
}

#[test]
fn skill_refine_subject_serializes_with_kind_tag_for_audit_filter() {
    // Audit-feed consumers filter judge_record rows by `kind`
    // (e.g. forge_promote dashboard pulls all `skill_refine`
    // subjects). Pin the serde tag so a rename can't silently
    // break dashboards.
    let subject = JudgeSubject::SkillRefine {
        name: "alpha".into(),
        version_from: "0.1".into(),
        version_to_hash: [0u8; 32],
    };
    let json = serde_json::to_string(&subject).expect("serialize");
    assert!(json.contains("skill_refine"), "serde tag drifted: {json}");
    assert!(json.contains("version_from"));
    assert!(json.contains("version_to_hash"));
}

// ──────────────────────────────────────────────────────────────
// FINDINGS — ignored test marking the gap surfaced this round
// ──────────────────────────────────────────────────────────────

/// 🟡 **FINDING**: every production caller of
/// `forge_optimize::run_with_events_and_run_id` passes
/// `judge: None`. The judge gate landed via PR #940
/// (`mission-wire-judge-on-forge-promote`) as an opt-in seam,
/// but no follow-up brief threaded a real `LlmJudge` through
/// the three production entry points
/// (`run`, `run_with_events`, `run_claimed`) at
/// `crates/argo-cli/src/handlers/forge_optimize.rs:32, 57, 83`.
///
/// Net effect today: `evaluate_promote_gates` is reached only
/// with `judge: None`, which the function documents as the
/// "back-compat path" that gives "the threshold gate only". The
/// fail-CLOSED judge gate is **wired but unconsumed in
/// production**.
///
/// **Fix-brief to queue**: `mission-forge-optimize-thread-real-judge`
/// — construct `LlmJudge::with_default_rubric` against the
/// session's resolved transport + judge model, thread it
/// through `run_with_events_and_run_id`, gate behind an env
/// flag (`ARGO_FORGE_JUDGE=on`) for v1.x rollout, and remove
/// `#[ignore]` from this test once the production wire is live.
///
/// This test is FINDINGS-ONLY: it cannot reach into `argo-cli`
/// (outside the may-touch list), so it deliberately fails when
/// run to keep the gap visible. The `#[ignore]` attribute
/// suppresses it from the default `cargo test` set.
#[test]
#[ignore = "FINDINGS-ONLY — gap is in argo-cli (outside may-touch); see doc-comment + fix-brief mission-forge-optimize-thread-real-judge"]
fn findings_forge_optimize_production_callers_pass_judge_none() {
    panic!(
        "FINDING: forge_optimize entry points all pass judge: None — \
         judge fail-CLOSED gate is wired (#940) but never reached in \
         production. Fix-brief: mission-forge-optimize-thread-real-judge.",
    );
}
