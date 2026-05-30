//! Tests for the synchronous Genesis hire gate.
//!
//! These exercise the REAL eval floor against a real `GenesisPlan` and the
//! degraded safety path. The safety judge's provider is `claude-code-cli`
//! by default; in the test environment that subprocess is not a usable
//! provider, so the judge degrades to `needs_review` (fail SAFE) rather than
//! a silent pass — which is exactly the contract we assert.

use super::*;
use argo_genesis::{build_gate_eval_inputs, plan, write_vertical, GateState};

/// Force an unresolvable provider id so the safety judge's transport call
/// errors deterministically regardless of the host's env / keys, exercising
/// the degraded → needs_review path. Returns a guard that restores the env.
struct ProviderGuard {
    prev: Option<String>,
}
impl ProviderGuard {
    fn unresolvable() -> Self {
        let prev = std::env::var("ARGO_GENESIS_LLM_PROVIDER").ok();
        std::env::set_var(
            "ARGO_GENESIS_LLM_PROVIDER",
            "definitely-not-a-real-provider-xyz",
        );
        Self { prev }
    }
}
impl Drop for ProviderGuard {
    fn drop(&mut self) {
        match &self.prev {
            Some(v) => std::env::set_var("ARGO_GENESIS_LLM_PROVIDER", v),
            None => std::env::remove_var("ARGO_GENESIS_LLM_PROVIDER"),
        }
    }
}

#[tokio::test]
async fn rich_plan_clears_the_deterministic_eval_floor() {
    let p = plan(
        "be my marketing intern, run weekly campaigns to cold leads and draft newsletters",
        "maya",
    )
    .expect("plan builds");
    let inputs = build_gate_eval_inputs(&p).expect("inputs");
    let eval = run_eval_floor(&inputs.seeds, &inputs.contract_text).await;
    assert_eq!(eval.total, 10, "ten starter seeds graded");
    assert!(
        eval.passed_floor,
        "a real teammate must clear its own starter floor; got {}/{} ({:.0}%)",
        eval.passed,
        eval.total,
        eval.pass_rate * 100.0
    );
}

#[tokio::test]
async fn empty_seed_set_fails_the_floor_closed() {
    // No seeds → the floor is a fail-closed Blocked (a teammate with no
    // starter suite is the thin worker the gate exists to catch).
    let eval = run_eval_floor(&[], "any contract text").await;
    assert_eq!(eval.total, 0);
    assert!(!eval.passed_floor, "zero seeds must fail the floor");
}

#[tokio::test]
async fn safety_judge_degrades_to_needs_review_without_a_live_provider() {
    let _g = ProviderGuard::unresolvable();
    let p = plan("be my marketing intern, run weekly campaigns", "maya").expect("plan builds");
    let signal = run_safety_signal(judge_input_for_plan(&p)).await;
    assert!(
        matches!(signal, SafetySignal::Unavailable { .. }),
        "expected degraded Unavailable signal; got {signal:?}"
    );
}

#[tokio::test]
async fn full_gate_on_rich_plan_is_needs_review_without_provider_never_blocked() {
    // End-to-end: a rich plan with a working eval floor but no provider must
    // land NeedsReview (degraded) — never a silent Pass, never a spurious
    // Block of a valid teammate.
    let _g = ProviderGuard::unresolvable();
    let p = plan(
        "be my marketing intern, run weekly campaigns to cold leads and draft newsletters",
        "maya",
    )
    .expect("plan builds");
    let verdict = run_hire_gate(&p).await;
    assert_eq!(verdict.eval_total, 10);
    assert!(verdict.eval_passed >= 5, "eval floor cleared on a rich plan");
    assert_eq!(
        verdict.state,
        GateState::NeedsReview,
        "no-provider rich plan must be needs_review (fail-safe); got {:?} reasons={:?}",
        verdict.state,
        verdict.reasons
    );
    assert!(!verdict.state.allows_hire());
}

#[tokio::test]
async fn disk_gate_matches_plan_gate_for_the_same_vertical() {
    // The hire-commit fence reconstructs the gate from the written vertical.
    // Its eval-floor outcome must match the in-memory plan's (same seeds,
    // same contract) — parity, no per-surface divergence.
    let _g = ProviderGuard::unresolvable();
    let p = plan(
        "be my marketing intern, run weekly campaigns to cold leads and draft newsletters",
        "maya",
    )
    .expect("plan builds");
    let dir = tempfile::tempdir().expect("tempdir");
    let out = dir.path().join(&p.vertical_id);
    write_vertical(&p, &out).expect("write vertical");

    let from_dir = run_hire_gate_from_dir(&out).await;
    assert_eq!(from_dir.eval_total, 10, "disk gate grades the 10 on-disk seeds");
    assert!(
        from_dir.eval_passed >= 5,
        "disk-reconstructed contract clears its own floor; got {}/{}",
        from_dir.eval_passed,
        from_dir.eval_total
    );
    // Degraded (no provider) → needs_review, fail-safe, not hireable.
    assert_eq!(from_dir.state, GateState::NeedsReview);
    assert!(!from_dir.state.allows_hire());
}

#[tokio::test]
async fn disk_gate_hard_blocks_a_teammate_that_fails_its_evals() {
    // Write a real vertical, then truncate its eval-seed file so NO seed
    // grades → the deterministic eval floor fails → the composed gate is a
    // HARD Blocked (a failing-evals teammate is refused activation), not a
    // degraded needs_review. This is the core P0 guarantee.
    let p = plan("be my marketing intern, run weekly campaigns", "maya").expect("plan builds");
    let dir = tempfile::tempdir().expect("tempdir");
    let out = dir.path().join(&p.vertical_id);
    write_vertical(&p, &out).expect("write vertical");
    // Empty the on-disk starter suite (simulates a thin/empty teammate).
    std::fs::write(out.join("eval-seed.toml"), "").expect("truncate seeds");

    let verdict = run_hire_gate_from_dir(&out).await;
    assert_eq!(verdict.eval_total, 0, "no seeds graded");
    assert!(!verdict.eval_pass_rate.is_nan());
    assert_eq!(
        verdict.state,
        GateState::Blocked,
        "a teammate that fails its evals must be hard-blocked; got {:?} reasons={:?}",
        verdict.state,
        verdict.reasons
    );
    assert!(!verdict.state.allows_hire(), "blocked teammate is NOT hireable");
    assert!(
        verdict.reasons.iter().any(|r| r.contains("eval floor failed")),
        "blocked reason should cite the eval floor; got {:?}",
        verdict.reasons
    );
}

#[tokio::test]
async fn disk_gate_fails_closed_on_missing_manifest() {
    // Pointing the hire fence at an empty dir (no vertical.toml) must fail
    // closed — never a pass.
    let dir = tempfile::tempdir().expect("tempdir");
    let verdict = run_hire_gate_from_dir(dir.path()).await;
    assert_ne!(verdict.state, GateState::Pass, "unreadable manifest must not pass");
    assert!(!verdict.state.allows_hire());
}

#[test]
fn contract_subject_returns_the_contract_for_any_task() {
    use argo_eval::EvalSubject;
    let subject = ContractSubject {
        contract_text: "marketing intern drafts campaigns".to_string(),
    };
    let task = EvalTask {
        name: "t".into(),
        input: "anything".into(),
        expected: String::new(),
        metric: EvalMetric::SubstringContains {
            must_contain: vec!["marketing".into()],
        },
    };
    let out = tokio_test_block(subject.run(&task));
    assert_eq!(out.unwrap(), "marketing intern drafts campaigns");
}

fn tokio_test_block<F: std::future::Future>(fut: F) -> F::Output {
    tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("rt")
        .block_on(fut)
}
