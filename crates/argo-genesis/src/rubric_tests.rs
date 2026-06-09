//! Adversarial unit tests for the LLM-graded capability rubric (Genesis P7).
//!
//! Proves the three ULTRACODE properties on the PURE half (no provider):
//! 1. The rubric **cannot be gamed** — a thin/incoherent grade is blocked even
//!    when scope nouns are present; out-of-range / clamped scores can't inflate
//!    a pass.
//! 2. **Degraded mode fails safe** — a transport error OR an unparseable /
//!    incomplete response degrades to `Unavailable` → `passed_floor == false`,
//!    NEVER a silent pass.
//! 3. **Discrimination preserved** — a genuinely capable grade passes; a
//!    thin/incoherent one fails, through the IDENTICAL grader path.

use super::*;
use argo_types::{ArgoError, ArgoResult};

/// A mock grader that returns a canned response string verbatim — lets tests
/// exercise the full `grade_rubric → parse → outcome` path deterministically
/// with no provider, exactly like `genesis_judge_tests::CannedJudge`.
struct CannedGrader {
    response: String,
}

#[async_trait::async_trait]
impl RubricClient for CannedGrader {
    async fn grade(&self, _prompt: &str) -> ArgoResult<String> {
        Ok(self.response.clone())
    }
}

/// A mock grader whose transport always errors — exercises the degrade path.
struct FailingGrader;

#[async_trait::async_trait]
impl RubricClient for FailingGrader {
    async fn grade(&self, _prompt: &str) -> ArgoResult<String> {
        Err(ArgoError::Provider("simulated rubric transport crash".into()))
    }
}

fn sample_input() -> JudgeInput<'static> {
    JudgeInput {
        charter: "be my marketing intern, run weekly campaigns to cold leads",
        persona_description: "Maya, a diligent marketing intern.",
        action_vocabulary: vec!["draft_campaign".into(), "schedule_send".into()],
        action_approvals: None,
    }
}

async fn grade(response: &str) -> RubricOutcome {
    let grader = CannedGrader {
        response: response.to_string(),
    };
    let verdict = grade_rubric(&sample_input(), "behaviour contract text", &grader).await;
    RubricOutcome::from_verdict(verdict)
}

#[tokio::test]
async fn capable_teammate_passes_the_rubric() {
    // DISCRIMINATION (positive): a high grade across every dimension clears.
    let o = grade(
        r#"{"scope_clarity": 0.9, "action_coherence": 0.85, "governance_posture": 0.8, "role_fit": 0.9, "rationale": "clear scope, coherent actions, gated"}"#,
    )
    .await;
    assert!(o.applicable);
    assert!(!o.degraded);
    assert!(o.passed_floor, "a genuinely capable teammate must clear the rubric: {o:?}");
    assert!(o.weighted_score >= RUBRIC_PASS_FLOOR);
    assert!(o.detail.contains("clear scope"));
}

#[tokio::test]
async fn thin_incoherent_teammate_is_blocked_even_with_scope_present() {
    // CANNOT BE GAMED: a teammate whose scope NOUN is recognised (scope_clarity
    // high) but which has no coherent capability + no governance (the exact thin
    // shell the substring floor can't catch) is BLOCKED by the rubric. This is
    // the property that makes the rubric richer than the keyword floor.
    let o = grade(
        r#"{"scope_clarity": 0.9, "action_coherence": 0.2, "governance_posture": 0.1, "role_fit": 0.2, "rationale": "names the domain but no real capability"}"#,
    )
    .await;
    assert!(o.applicable);
    assert!(!o.degraded);
    assert!(
        !o.passed_floor,
        "a thin/incoherent teammate must FAIL the rubric even with a clear scope: {o:?}"
    );
    assert!(o.min_dimension < DIMENSION_FLOOR, "weakest facet must be surfaced");
}

#[tokio::test]
async fn one_failing_dimension_blocks_even_with_a_high_mean() {
    // CANNOT BE GAMED (per-dimension floor): three near-perfect dimensions
    // cannot carry one that is below the per-dimension bar. A teammate with no
    // governance posture is not production-ready regardless of how clear its
    // scope is. The weighted mean here is high (>= the mean floor), so ONLY the
    // per-dimension floor catches it — proving that floor is load-bearing.
    let o = grade(
        r#"{"scope_clarity": 1.0, "action_coherence": 1.0, "governance_posture": 0.3, "role_fit": 1.0, "rationale": "strong but ungoverned"}"#,
    )
    .await;
    assert!(
        o.weighted_score >= RUBRIC_PASS_FLOOR,
        "the weighted mean is above the mean floor here ({}), so only the \
         per-dimension floor can catch this teammate",
        o.weighted_score
    );
    assert!(
        !o.passed_floor,
        "a single sub-floor dimension (governance 0.3) must block even with a high mean: {o:?}"
    );
}

#[tokio::test]
async fn scraping_the_per_dimension_floor_still_fails_the_mean() {
    // CANNOT BE GAMED (mean floor): a teammate that scores EXACTLY at the
    // per-dimension floor on every axis clears every per-dimension check but its
    // weighted mean (0.5) is below the mean floor (0.6) → blocked. Proves the
    // mean floor is also load-bearing, not subsumed by the per-dimension one.
    let o = grade(
        r#"{"scope_clarity": 0.5, "action_coherence": 0.5, "governance_posture": 0.5, "role_fit": 0.5, "rationale": "mediocre across the board"}"#,
    )
    .await;
    assert!((o.min_dimension - 0.5).abs() < 1e-9, "every dimension is exactly at the floor");
    assert!(o.weighted_score < RUBRIC_PASS_FLOOR);
    assert!(!o.passed_floor, "scraping the per-dimension floor must still fail the mean: {o:?}");
}

#[tokio::test]
async fn out_of_range_scores_are_clamped_and_cannot_inflate_a_pass() {
    // CANNOT BE GAMED (clamping): a model that emits >1.0 scores cannot inflate
    // a grade. The genuinely failing governance dimension (clamped from a huge
    // value down to... no — a HIGH out-of-range value clamps to 1.0, so the
    // adversarial direction is a LOW dimension that a huge value elsewhere
    // can't rescue). Here governance is genuinely 0.1; the inflated 5.0 on the
    // others clamps to 1.0 but the per-dimension floor still blocks.
    let o = grade(
        r#"{"scope_clarity": 5.0, "action_coherence": 9.9, "governance_posture": 0.1, "role_fit": 8.0, "rationale": "inflated"}"#,
    )
    .await;
    assert!(o.weighted_score <= 1.0, "clamped scores never exceed 1.0: {}", o.weighted_score);
    assert!(!o.passed_floor, "an inflated grade can't rescue a genuinely failing dimension: {o:?}");
}

#[tokio::test]
async fn nan_score_collapses_conservatively_and_blocks() {
    // CANNOT BE GAMED (NaN): JSON can't carry NaN, but defend the clamp anyway —
    // a NaN dimension collapses to 0.0 (conservative) and blocks, never passes.
    let o = RubricOutcome::from_verdict(RubricVerdict::Graded {
        scope_clarity: 1.0,
        action_coherence: 1.0,
        governance_posture: f64::NAN,
        role_fit: 1.0,
        rationale: String::new(),
    });
    assert_eq!(o.min_dimension, 0.0, "NaN must collapse to 0.0");
    assert!(!o.passed_floor, "a NaN dimension must block: {o:?}");
}

#[tokio::test]
async fn transport_error_degrades_fail_safe_never_passes() {
    // DEGRADED MODE (transport): a provider crash → Unavailable → not a pass.
    let verdict = grade_rubric(&sample_input(), "contract", &FailingGrader).await;
    match &verdict {
        RubricVerdict::Unavailable { reason } => {
            assert!(reason.contains("transport error"), "got: {reason}");
        }
        other => panic!("a transport crash MUST degrade to Unavailable, got {other:?}"),
    }
    let o = RubricOutcome::from_verdict(verdict);
    assert!(o.applicable);
    assert!(o.degraded, "transport failure must mark the outcome degraded");
    assert!(!o.passed_floor, "a degraded rubric is NEVER a silent pass");
}

#[tokio::test]
async fn unparseable_response_degrades_fail_safe_never_passes() {
    // DEGRADED MODE (parse): a non-JSON response → Unavailable, not a pass.
    let o = grade("this is not json at all, just prose").await;
    assert!(o.degraded, "unparseable response must degrade");
    assert!(!o.passed_floor, "an unparseable grade is NEVER a silent pass: {o:?}");
}

#[tokio::test]
async fn missing_dimension_degrades_fail_safe_never_passes() {
    // DEGRADED MODE (incomplete): a JSON object missing a required dimension is
    // a parse failure → Unavailable. A model that omits governance_posture can
    // NOT be read as a clearance on the other three — that would be the exact
    // silent-pass-on-malformed-grade hole the design forbids.
    let o = grade(
        r#"{"scope_clarity": 0.9, "action_coherence": 0.9, "role_fit": 0.9, "rationale": "no governance score"}"#,
    )
    .await;
    assert!(o.degraded, "a missing dimension must degrade, not pass on the present ones");
    assert!(!o.passed_floor);
    assert!(o.detail.contains("governance_posture"), "the reason names the missing dim: {}", o.detail);
}

#[tokio::test]
async fn non_numeric_dimension_degrades_fail_safe() {
    // DEGRADED MODE (type): a dimension that is a string, not a number, is a
    // parse failure → Unavailable, never a pass.
    let o = grade(
        r#"{"scope_clarity": "excellent", "action_coherence": 0.9, "governance_posture": 0.9, "role_fit": 0.9}"#,
    )
    .await;
    assert!(o.degraded);
    assert!(!o.passed_floor);
}

#[tokio::test]
async fn prose_wrapped_json_still_parses() {
    // ROBUSTNESS: the grader tolerates a model that wraps the JSON in reasoning
    // prose (the claude-code-cli habit) — the first balanced object is parsed.
    let o = grade(
        "Here is my assessment of the teammate:\n\n{\"scope_clarity\": 0.8, \"action_coherence\": 0.8, \"governance_posture\": 0.7, \"role_fit\": 0.8, \"rationale\": \"solid\"}\n\nThat concludes my review.",
    )
    .await;
    assert!(!o.degraded, "prose-wrapped JSON must still grade: {o:?}");
    assert!(o.passed_floor);
}

#[test]
fn not_applicable_is_a_vacuous_pass() {
    // The flag-gated-off posture: the rubric is a vacuous pass, so the
    // deterministic floor + other axes gate unchanged.
    let o = RubricOutcome::not_applicable();
    assert!(!o.applicable);
    assert!(!o.degraded);
    assert!(o.passed_floor, "a not-applicable rubric must pass vacuously");
}

#[test]
fn prompt_includes_all_four_dimensions_and_the_role() {
    let prompt = build_rubric_prompt(&sample_input(), "the behaviour contract");
    for dim in RubricDimension::ALL {
        assert!(
            prompt.contains(dim.key()),
            "prompt must name dimension {}",
            dim.key()
        );
    }
    assert!(prompt.contains("be my marketing intern"), "prompt must carry the charter");
    assert!(prompt.contains("draft_campaign"), "prompt must carry the action vocabulary");
    assert!(prompt.contains("the behaviour contract"), "prompt must carry the contract excerpt");
}

#[test]
fn weights_sum_and_governance_role_fit_are_emphasised() {
    // The weighting is the design knob that makes the rubric catch what the
    // substring floor misses; pin it so a refactor can't silently flatten it.
    let total: f64 = RubricDimension::ALL.iter().map(|d| d.weight()).sum();
    assert!((total - 5.0).abs() < 1e-9, "weights sum to 5.0");
    assert!(RubricDimension::GovernancePosture.weight() > RubricDimension::ScopeClarity.weight());
    assert!(RubricDimension::RoleFit.weight() > RubricDimension::ActionCoherence.weight());
}
