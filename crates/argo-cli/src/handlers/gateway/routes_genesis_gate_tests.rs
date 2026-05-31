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
///
/// Holds the GATEWAY-WIDE env lock for its whole lifetime: it mutates the
/// process-global `ARGO_GENESIS_LLM_PROVIDER` AND, because `run_hire_gate`
/// consumes the test-only forced-verdict override, it must not run
/// concurrently with the first-hire tests that set that override. Clears any
/// stale forced verdict on acquisition so a `run_hire_gate` call here always
/// exercises the REAL gate.
struct ProviderGuard {
    prev: Option<String>,
    _lock: tokio::sync::OwnedMutexGuard<()>,
}
impl ProviderGuard {
    async fn unresolvable() -> Self {
        let lock = crate::handlers::gateway::shared_env_lock().await;
        test_overrides::clear_forced_verdict();
        let prev = std::env::var("ARGO_GENESIS_LLM_PROVIDER").ok();
        std::env::set_var(
            "ARGO_GENESIS_LLM_PROVIDER",
            "definitely-not-a-real-provider-xyz",
        );
        Self { prev, _lock: lock }
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

/// A legitimate from-scratch teammate generated through the SAME
/// `BroadRoleDraft → CapabilitySpec → materialize_spec` seam the flagship
/// `POST /api/v1/genesis` SSE page uses (the `broad` / LLM path). Built
/// in-test (no provider) so the eval-floor assertions are deterministic.
fn broad_path_plan() -> GenesisPlan {
    use argo_genesis::{materialize_spec, Archetype, BroadRoleDraft, CapabilitySpec};
    let draft = BroadRoleDraft {
        charter: "a vineyard logistics coordinator who schedules harvests, \
                  drafts trucking plans, and dispatches haulage to the winery"
            .into(),
        persona: "vinny".into(),
        persona_description: "Coordinates vineyard harvest scheduling and haulage logistics."
            .into(),
        action_vocabulary: vec![
            "schedule_harvest".into(),
            "draft_truck_plan".into(),
            "send_dispatch_notice".into(),
        ],
        success_metrics: vec!["zero missed harvest windows".into()],
        onboarding_script: "Hi, I keep the harvest calendar tight.".into(),
    };
    let spec = CapabilitySpec::from_broad_draft(draft, Archetype::CalendarCoordinator);
    materialize_spec(&spec).expect("broad-path teammate materializes")
}

#[tokio::test]
async fn broad_llm_path_teammate_clears_the_eval_floor_through_the_real_runner() {
    // REGRESSION for the flagship `/genesis` SSE 0/10 bug: a legitimate
    // broad/LLM-path teammate previously scored 0/10 on the deterministic floor
    // (eval-seed keyword tokenizer deleted `-`/`_` from the boilerplate
    // description; the contract kept them → no keyword could ever match) and
    // was permanently `blocked`. Drive the ACTUAL canonical
    // `argo_eval::run_eval` scorer via `run_eval_floor` and assert it now
    // clears its own >=50% floor.
    let p = broad_path_plan();
    let inputs = build_gate_eval_inputs(&p).expect("broad-path inputs");
    let eval = run_eval_floor(&inputs.seeds, &inputs.contract_text).await;
    assert_eq!(eval.total, 10, "ten starter seeds graded");
    assert!(
        eval.passed_floor,
        "a legitimate broad/LLM-path teammate must clear its own starter floor \
         through the real runner; got {}/{} ({:.0}%)",
        eval.passed,
        eval.total,
        eval.pass_rate * 100.0
    );
}

#[tokio::test]
async fn broad_path_degenerate_contract_still_fails_the_floor() {
    // DISCRIMINATION (no theater on the safety gate): the SAME seeds the
    // legitimate broad-path teammate clears must FAIL when scored against a
    // thin/degenerate contract (the empty-worker case the floor exists to
    // catch). Proves the fix did not make the floor pass unconditionally.
    let p = broad_path_plan();
    let inputs = build_gate_eval_inputs(&p).expect("broad-path inputs");
    let eval = run_eval_floor(&inputs.seeds, "i am a helpful assistant.").await;
    assert_eq!(eval.total, 10);
    assert!(
        !eval.passed_floor,
        "a thin/degenerate teammate must STILL fail the floor; got {}/{} ({:.0}%)",
        eval.passed,
        eval.total,
        eval.pass_rate * 100.0
    );
    assert_eq!(eval.passed, 0, "no seed should match an empty-stub contract");
}

#[tokio::test]
async fn broad_path_full_gate_is_needs_review_not_blocked_without_a_provider() {
    // END-TO-END: the full hire gate on a legitimate broad-path teammate, with
    // no live safety-judge provider, must land NeedsReview (degraded safety) —
    // NEVER a hard Blocked from a failing eval floor (the bug) and never a
    // silent Pass. This is the user-visible promise: an SSE-generated teammate
    // can become hireable once the operator confirms + a provider is present.
    let _g = ProviderGuard::unresolvable().await;
    let p = broad_path_plan();
    let digest = argo_genesis::RoleContract::derive_from_manifest(&p.manifest).digest();
    let verdict = run_hire_gate_with_confirmation(&p, None, Some(&digest)).await;
    assert_eq!(verdict.eval_total, 10);
    assert!(
        verdict.eval_passed >= 5,
        "the eval floor must clear on a legit broad-path teammate; got {}/{} reasons={:?}",
        verdict.eval_passed,
        verdict.eval_total,
        verdict.reasons
    );
    assert!(
        !verdict.reasons.iter().any(|r| r.contains("eval floor failed")),
        "the eval floor must NOT be a block reason on a legit teammate; got {:?}",
        verdict.reasons
    );
    assert_eq!(
        verdict.state,
        GateState::NeedsReview,
        "degraded (no-provider) broad-path gate is needs_review, not blocked; got {:?} {:?}",
        verdict.state,
        verdict.reasons
    );
}

#[tokio::test]
async fn safety_judge_degrades_to_needs_review_without_a_live_provider() {
    let _g = ProviderGuard::unresolvable().await;
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
    let _g = ProviderGuard::unresolvable().await;
    let p = plan(
        "be my marketing intern, run weekly campaigns to cold leads and draft newsletters",
        "maya",
    )
    .expect("plan builds");
    let verdict = run_hire_gate(&p, None).await;
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
    let _g = ProviderGuard::unresolvable().await;
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

// --- Role-contract operator-confirmation adversarial proofs (DEC-025 #1 + #4) ---

/// The truthful contract digest for a plan — what the operator must echo back.
fn truthful_digest(p: &GenesisPlan) -> String {
    argo_genesis::RoleContract::derive_from_manifest(&p.manifest).digest()
}

#[tokio::test]
async fn plan_gate_holds_for_review_without_a_confirmed_contract() {
    // ADVERSARIAL (no-activation-without-confirmation): even with a working
    // eval floor, a teammate with NO operator confirmation of its role
    // contract cannot reach Pass — it holds in needs_review. Force the safety
    // judge to allow via an unresolvable... no: instead, prove it deterministically
    // by checking the contract axis directly through the public gate fn with a
    // GOOD digest vs NONE, isolating the confirmation effect from the (degraded)
    // safety axis.
    let _g = ProviderGuard::unresolvable().await;
    let p = plan(
        "be my marketing intern, run weekly campaigns to cold leads and draft newsletters",
        "maya",
    )
    .expect("plan builds");

    // No digest → unconfirmed → not hireable (one of the block reasons is the
    // awaiting-confirmation hold).
    let unconfirmed = run_hire_gate_with_confirmation(&p, None, None).await;
    assert!(!unconfirmed.state.allows_hire());
    assert_eq!(unconfirmed.contract_confirmation, "unconfirmed");
    assert!(
        unconfirmed
            .reasons
            .iter()
            .any(|r| r.contains("awaiting operator confirmation")),
        "expected awaiting-confirmation reason; got {:?}",
        unconfirmed.reasons
    );
    // The truthful contract is surfaced for the operator to review + confirm.
    assert!(unconfirmed.role_contract.is_some());
    assert_eq!(unconfirmed.contract_digest.len(), 64);
}

#[tokio::test]
async fn plan_gate_with_correct_digest_resolves_confirmed_leaving_only_safety_to_degrade() {
    // ADVERSARIAL (digest plumbing): when the operator echoes the CORRECT
    // digest, the confirmation axis resolves to `confirmed` — proving the
    // confirmation is genuinely checked against the teammate's truthful
    // contract. With no live provider the only residual blocker is the
    // degraded safety judge (needs_review), NOT the contract — so the gate is
    // needs_review WITHOUT an awaiting-confirmation reason.
    let _g = ProviderGuard::unresolvable().await;
    let p = plan(
        "be my marketing intern, run weekly campaigns to cold leads and draft newsletters",
        "maya",
    )
    .expect("plan builds");
    let digest = truthful_digest(&p);

    let verdict = run_hire_gate_with_confirmation(&p, None, Some(&digest)).await;
    assert_eq!(verdict.contract_confirmation, "confirmed");
    assert!(
        !verdict
            .reasons
            .iter()
            .any(|r| r.contains("awaiting operator confirmation")),
        "a confirmed contract must NOT add an awaiting-confirmation reason; got {:?}",
        verdict.reasons
    );
    // Residual block is purely the degraded safety judge.
    assert_eq!(verdict.state, GateState::NeedsReview);
    assert_eq!(verdict.safety_verdict, "unavailable");
}

#[tokio::test]
async fn plan_gate_hard_blocks_a_forged_or_stale_contract_confirmation() {
    // ADVERSARIAL (forgery/staleness): an operator-supplied digest that does
    // NOT match the teammate's truthful contract is a HARD block — a stale or
    // forged confirmation must never activate a teammate the operator never
    // actually saw. This is a Blocked, outranking the degraded safety signal.
    let _g = ProviderGuard::unresolvable().await;
    let p = plan("be my marketing intern, run weekly campaigns", "maya").expect("plan");
    let verdict = run_hire_gate_with_confirmation(&p, None, Some("not-the-real-digest")).await;
    assert_eq!(verdict.state, GateState::Blocked);
    assert!(!verdict.state.allows_hire());
    assert_eq!(verdict.contract_confirmation, "mismatch");
    assert!(
        verdict
            .reasons
            .iter()
            .any(|r| r.contains("confirmation mismatch")),
        "expected a mismatch reason; got {:?}",
        verdict.reasons
    );
}

#[tokio::test]
async fn disk_fence_confirms_the_same_digest_the_sse_payload_surfaced() {
    // ADVERSARIAL (cross-seam parity): the digest the SSE path surfaces (from
    // the in-memory plan) MUST equal the digest the hire-commit fence derives
    // (from the on-disk manifest), so a confirmation made against the SSE
    // payload validates at the `POST /api/v1/workers` fence — no per-surface
    // divergence in WHAT the operator confirms.
    let _g = ProviderGuard::unresolvable().await;
    let p = plan(
        "be my marketing intern, run weekly campaigns to cold leads and draft newsletters",
        "maya",
    )
    .expect("plan builds");
    let dir = tempfile::tempdir().expect("tempdir");
    let out = dir.path().join(&p.vertical_id);
    write_vertical(&p, &out).expect("write vertical");

    let plan_digest = truthful_digest(&p);

    // The fence, given the SSE-surfaced digest, resolves `confirmed` (the
    // on-disk contract matches the in-memory one).
    let confirmed = run_hire_gate_from_dir_with_confirmation(&out, Some(&plan_digest)).await;
    assert_eq!(
        confirmed.contract_confirmation, "confirmed",
        "the SSE-surfaced digest must confirm at the disk fence (parity)"
    );
    assert_eq!(confirmed.contract_digest, plan_digest);

    // And with no digest the fence holds for review (no activation w/o confirm).
    let unconfirmed = run_hire_gate_from_dir_with_confirmation(&out, None).await;
    assert_eq!(unconfirmed.contract_confirmation, "unconfirmed");
    assert!(!unconfirmed.state.allows_hire());
}

#[tokio::test]
async fn disk_fence_truthful_contract_reflects_bound_approval_gated_actions() {
    // ADVERSARIAL (truthfulness): the contract the fence derives names the
    // teammate's ACTUAL approval-gated actions — a teammate cannot present a
    // benign contract while its manifest binds an approval-gated sensitive
    // verb. The marketing intern's `post_campaign`/`send_followup` are
    // sensitive (forced approval by the safety judge) and MUST appear in the
    // contract's approval_required set, never as autonomous.
    let _g = ProviderGuard::unresolvable().await;
    let p = plan(
        "be my marketing intern, run weekly campaigns to cold leads and draft newsletters",
        "maya",
    )
    .expect("plan builds");
    let dir = tempfile::tempdir().expect("tempdir");
    let out = dir.path().join(&p.vertical_id);
    write_vertical(&p, &out).expect("write vertical");

    let inputs = argo_genesis::gate_inputs_from_dir(&out).expect("disk inputs");
    let contract = inputs.role_contract();
    assert!(
        contract.approval_required.iter().any(|a| a == "post_campaign"),
        "post_campaign must be approval-gated in the truthful contract: {:?}",
        contract.approval_required
    );
    assert!(
        !contract.autonomous.iter().any(|a| a == "post_campaign"),
        "a sensitive verb must NEVER appear as autonomous: {:?}",
        contract.autonomous
    );
    // The contract is self-consistent with the manifest it was derived from.
    assert!(argo_genesis::verify_contract_matches(&contract, &inputs.manifest));
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
