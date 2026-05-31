//! Unit tests for the LLM rubric AXIS provider half (Genesis P7, DEC-025 #7).
//!
//! These cover the flag-gating + the degrade-fail-safe transport path of
//! [`run_rubric_axis`] in isolation. The FULL-gate composition proofs (a thin
//! teammate the floor let through is blocked; a degraded rubric on a valid
//! teammate is needs_review not pass; the floor still hard-blocks even with the
//! rubric on) live next to the gate fns in `routes_genesis_gate_tests.rs`, and
//! the pure per-dimension scoring proofs live in `argo-genesis`
//! (`rubric_tests.rs` + `hire_gate_rubric_tests.rs`).

use super::{rubric_enabled, run_rubric_axis, GENESIS_LLM_RUBRIC_ENV};
use crate::handlers::gateway::routes_genesis_gate as gate;
use argo_genesis::{plan, GateState, JudgeInput, RoleContract};

/// Holds the GATEWAY-WIDE env lock for its lifetime (the rubric reads
/// process-global `ARGO_GENESIS_LLM_RUBRIC` + `ARGO_GENESIS_LLM_PROVIDER`).
struct RubricEnvGuard {
    prev_flag: Option<String>,
    prev_provider: Option<String>,
    _lock: tokio::sync::OwnedMutexGuard<()>,
}

impl RubricEnvGuard {
    async fn on_with_unresolvable_provider() -> Self {
        let lock = crate::handlers::gateway::shared_env_lock().await;
        // The full-gate tests consume the gate's test-only forced-verdict
        // override; clear any stale one so they exercise the REAL gate.
        gate::test_overrides::clear_forced_verdict();
        let prev_flag = std::env::var(GENESIS_LLM_RUBRIC_ENV).ok();
        let prev_provider = std::env::var("ARGO_GENESIS_LLM_PROVIDER").ok();
        std::env::set_var(GENESIS_LLM_RUBRIC_ENV, "1");
        std::env::set_var(
            "ARGO_GENESIS_LLM_PROVIDER",
            "definitely-not-a-real-provider-xyz",
        );
        Self {
            prev_flag,
            prev_provider,
            _lock: lock,
        }
    }

    async fn off() -> Self {
        let lock = crate::handlers::gateway::shared_env_lock().await;
        let prev_flag = std::env::var(GENESIS_LLM_RUBRIC_ENV).ok();
        let prev_provider = std::env::var("ARGO_GENESIS_LLM_PROVIDER").ok();
        std::env::remove_var(GENESIS_LLM_RUBRIC_ENV);
        Self {
            prev_flag,
            prev_provider,
            _lock: lock,
        }
    }
}

impl Drop for RubricEnvGuard {
    fn drop(&mut self) {
        match &self.prev_flag {
            Some(v) => std::env::set_var(GENESIS_LLM_RUBRIC_ENV, v),
            None => std::env::remove_var(GENESIS_LLM_RUBRIC_ENV),
        }
        match &self.prev_provider {
            Some(v) => std::env::set_var("ARGO_GENESIS_LLM_PROVIDER", v),
            None => std::env::remove_var("ARGO_GENESIS_LLM_PROVIDER"),
        }
    }
}

fn sample_input() -> JudgeInput<'static> {
    JudgeInput {
        charter: "be my marketing intern",
        persona_description: "Maya",
        action_vocabulary: vec!["draft_campaign".into()],
    }
}

#[tokio::test]
async fn rubric_axis_is_not_applicable_when_the_flag_is_off() {
    // DEFAULT POSTURE: with the flag off, the rubric axis is a vacuous pass — no
    // provider dial-out, no effect on the verdict. The deterministic floor stays
    // the gate.
    let _g = RubricEnvGuard::off().await;
    let outcome = run_rubric_axis(sample_input(), "a behaviour contract").await;
    assert!(!outcome.applicable, "flag off → not applicable");
    assert!(!outcome.degraded);
    assert!(outcome.passed_floor, "not-applicable is a vacuous pass");
}

#[tokio::test]
async fn rubric_axis_degrades_fail_safe_without_a_live_provider() {
    // DEGRADE FAIL-SAFE: with the flag ON but no resolvable provider, the rubric
    // grader transport errors → the axis is degraded (applicable + degraded),
    // which the composer maps to needs_review — NEVER a silent pass.
    let _g = RubricEnvGuard::on_with_unresolvable_provider().await;
    let outcome = run_rubric_axis(sample_input(), "a behaviour contract").await;
    assert!(outcome.applicable, "flag on → applicable");
    assert!(outcome.degraded, "no provider → degraded");
    assert!(!outcome.passed_floor, "a degraded rubric is NEVER a silent pass");
    assert!(
        outcome.detail.contains("transport error"),
        "degraded reason should cite the transport error; got {:?}",
        outcome.detail
    );
}

#[test]
fn rubric_flag_parses_truthy_tokens_like_the_classifier() {
    // The opt-in discipline matches `ARGO_GENESIS_LLM_CLASSIFIER`: only
    // 1/true/on/yes (case-insensitive) enable it; everything else is off.
    let prev = std::env::var(GENESIS_LLM_RUBRIC_ENV).ok();
    for (val, want) in [
        ("1", true),
        ("true", true),
        ("ON", true),
        ("Yes", true),
        ("0", false),
        ("false", false),
        ("", false),
        ("maybe", false),
    ] {
        std::env::set_var(GENESIS_LLM_RUBRIC_ENV, val);
        assert_eq!(rubric_enabled(), want, "flag value {val:?} should be {want}");
    }
    std::env::remove_var(GENESIS_LLM_RUBRIC_ENV);
    assert!(!rubric_enabled(), "absent flag is off");
    if let Some(v) = prev {
        std::env::set_var(GENESIS_LLM_RUBRIC_ENV, v);
    }
}

#[tokio::test]
async fn full_gate_with_rubric_on_no_provider_is_needs_review_not_pass_not_block() {
    // END-TO-END (P7): a deterministically-VALID teammate (clears eval +
    // comprehension + a confirmed contract) with the rubric flag ON but no
    // provider must land NeedsReview — the degraded rubric (and the degraded
    // safety judge) hold it for review. NOT a silent Pass, and NOT a spurious
    // Block of a valid teammate (degrade != block).
    let _g = RubricEnvGuard::on_with_unresolvable_provider().await;
    let p = plan(
        "be my marketing intern, run weekly campaigns to cold leads and draft newsletters",
        "maya",
    )
    .expect("plan builds");
    let digest = RoleContract::derive_from_manifest(&p.manifest).digest();
    let verdict = gate::run_hire_gate_with_confirmation(&p, None, Some(&digest)).await;

    assert_eq!(verdict.eval_total, 10);
    assert!(
        verdict.eval_passed >= 5,
        "the deterministic eval floor must still clear on a valid teammate; got {}/{}",
        verdict.eval_passed,
        verdict.eval_total
    );
    assert!(verdict.rubric_applicable, "the rubric ran (flag on)");
    assert!(verdict.rubric_degraded, "no provider → rubric degraded");
    assert_eq!(verdict.rubric_verdict, "unavailable");
    assert_eq!(
        verdict.state,
        GateState::NeedsReview,
        "a degraded rubric must hold for review, not pass and not spuriously block; \
         reasons={:?}",
        verdict.reasons
    );
    assert!(!verdict.state.allows_hire());
    assert!(
        verdict.reasons.iter().any(|r| r.contains("rubric unavailable")),
        "expected a degraded-rubric reason; got {:?}",
        verdict.reasons
    );
    // CRITICAL: the rubric degradation must NOT have masked the deterministic
    // floor — the floor's pass is independent evidence the baseline still runs.
    assert!(
        !verdict.reasons.iter().any(|r| r.contains("eval floor failed")),
        "the deterministic floor must NOT be a block reason on a valid teammate; got {:?}",
        verdict.reasons
    );
}

#[tokio::test]
async fn rubric_on_no_provider_still_hard_blocks_a_failing_eval_floor() {
    // BASELINE HOLDS (no theater): even with the rubric ON (and degraded for no
    // provider), a teammate that FAILS the deterministic eval floor is HARD
    // BLOCKED — the degraded rubric (needs_review) can never soften a
    // deterministic block. The floor is the fail-closed baseline.
    let _g = RubricEnvGuard::on_with_unresolvable_provider().await;
    let p = plan("be my marketing intern, run weekly campaigns", "maya").expect("plan");
    let dir = tempfile::tempdir().expect("tempdir");
    let out = dir.path().join(&p.vertical_id);
    argo_genesis::write_vertical(&p, &out).expect("write vertical");
    // Empty the on-disk starter suite → the eval floor fails closed.
    std::fs::write(out.join("eval-seed.toml"), "").expect("truncate seeds");

    let verdict = gate::run_hire_gate_from_dir(&out).await;
    assert_eq!(verdict.eval_total, 0, "no seeds graded");
    assert_eq!(
        verdict.state,
        GateState::Blocked,
        "a failing eval floor must hard-block even with the rubric on; reasons={:?}",
        verdict.reasons
    );
    assert!(!verdict.state.allows_hire());
    assert!(
        verdict.reasons.iter().any(|r| r.contains("eval floor failed")),
        "the deterministic floor must still be the block reason; got {:?}",
        verdict.reasons
    );
}
