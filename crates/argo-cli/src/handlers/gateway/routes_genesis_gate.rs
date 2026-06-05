//! Synchronous Genesis hire gate (DEC-025 / Genesis P0).
//!
//! Runs the two REAL, fail-closed safety checks the `/genesis` product
//! path previously faked with a hardcoded `judge_verdict:"pass"`:
//!
//! 1. **Deterministic eval floor** — the 10 generated starter seeds are
//!    graded through the canonical [`argo_eval::run_eval`] runner against the
//!    teammate's behaviour contract. `>= 50%` of seeds (at `score >= 0.5`)
//!    must pass; an empty/thin teammate fails. No provider involved.
//! 2. **LLM semantic safety judge** — the shipped [`argo_genesis::genesis_judge`]
//!    asks 3 questions over the same provider transport the archetype
//!    classifier + Pitch Agent already use. A `block` verdict hard-blocks the
//!    teammate; a transport/provider failure degrades to `needs_review`
//!    (fail SAFE, audited, held in dry-run) — NEVER a silent pass.
//!
//! The composed [`GateVerdict`] gates the hire synchronously: only
//! [`argo_genesis::GateState::Pass`] permits activation. Both the flagship
//! SSE handler (`routes_genesis_stream`) and the hire-commit fence
//! (`routes_workers_hire`) call [`run_hire_gate`] so neither path can
//! activate a teammate that has not actually cleared the checks.

use super::routes_genesis_gate_judge::run_safety_signal;
use super::routes_genesis_rubric::run_rubric_axis;
use argo_genesis::{
    behaviour_contract_text, build_comprehension_questions, build_gate_eval_inputs,
    gate_inputs_from_dir, judge_input_for_plan, read_sop_knowledge, ComprehensionOutcome,
    ComprehensionQuestion, ContractGateInput, DeterministicEvalInput, EvalFloorOutcome,
    GateVerdict, GenesisPlan, SafetySignal, SopComprehensionOutcome, SopQuestion,
};
use argo_eval::{run_eval, EvalMetric, EvalSubject, EvalTask};
use argo_types::ArgoResult;
use async_trait::async_trait;
use std::path::Path;

/// Run the full synchronous hire gate against a resolved [`GenesisPlan`].
///
/// Deterministic part (the eval floor) always runs. The safety judge is
/// dialled over the live provider; any transport/provider failure degrades
/// to `needs_review` rather than blocking or silently passing.
///
/// `vertical_dir` is the teammate's written vertical directory, or `None` when
/// the plan is in-memory only (the `/onboarding/first-hire` seam, which uses a
/// catalog `vertical_id` and carries no uploaded SOP). The SOP axis (Genesis
/// P6) reads any uploaded SOP knowledge + its derived quiz from
/// `<dir>/knowledge/` (the SSE handler writes them there *before* the gate
/// runs), so the SOP-comprehension floor grades the SAME ingested knowledge the
/// hire-commit fence will re-read. `None` → SOP not applicable (vacuous pass).
///
/// Never errors at the type level — a failure to *build* the eval inputs
/// (degenerate empty role, already rejected upstream by the SSE handler)
/// collapses to an empty seed set, which the floor treats as a fail-closed
/// `Blocked`.
pub(super) async fn run_hire_gate(plan: &GenesisPlan, vertical_dir: Option<&Path>) -> GateVerdict {
    run_hire_gate_with_confirmation(plan, vertical_dir, None).await
}

/// As [`run_hire_gate`], but resolves the operator's role-contract
/// confirmation (DEC-025 #1 + #4). `confirmed_digest` is the digest the
/// operator echoed back from the surfaced "what can this teammate do"
/// contract; the gate derives the teammate's truthful contract from `plan`'s
/// manifest and confirms activation ONLY when the supplied digest matches it.
/// `None` → the contract is unconfirmed and the teammate holds in
/// `needs_review` (no activation without a confirmed contract).
pub(super) async fn run_hire_gate_with_confirmation(
    plan: &GenesisPlan,
    vertical_dir: Option<&Path>,
    confirmed_digest: Option<&str>,
) -> GateVerdict {
    // TEST-ONLY verdict override (compiled out of release builds → NO
    // production bypass surface). Tests that drive a *gate-cleared* hire to
    // assert downstream persistence — but are not themselves testing the gate
    // — force the composed verdict here, sidestepping the live-provider safety
    // judge CI lacks. Tests that ARE about the gate (the `first_hire_blocks_*`
    // suite) set NO override, so they exercise the real eval floor + degraded
    // safety path. Production never consults this.
    #[cfg(test)]
    if let Some(verdict) = test_overrides::take_forced_verdict() {
        return verdict;
    }
    let inputs = build_gate_eval_inputs(plan).ok();
    let (seeds, contract) = match inputs {
        Some(i) => (i.seeds, i.contract_text),
        None => (Vec::new(), String::new()),
    };
    let action_vocabulary: Vec<String> = plan
        .manifest
        .actions
        .as_ref()
        .map(|a| a.actions.keys().cloned().collect())
        .unwrap_or_default();
    let questions = build_comprehension_questions(&plan.charter, &action_vocabulary);
    let eval = run_eval_floor(&seeds, &contract).await;
    let comprehension = run_comprehension_floor(&questions, &contract).await;
    // The SOP-comprehension axis (Genesis P6 — the moat). Reads the uploaded
    // SOP knowledge + quiz from disk (the SSE handler wrote them before this
    // call), so the SSE path and the hire-commit fence grade the IDENTICAL
    // ingested knowledge. Not-applicable (vacuous pass) when no SOP uploaded or
    // when the plan is in-memory only (no written dir → first-hire seam).
    let sop = match vertical_dir {
        Some(dir) => run_sop_comprehension_floor(dir).await,
        None => SopComprehensionOutcome::not_applicable(),
    };
    let safety = run_safety_signal(judge_input_for_plan(plan)).await;
    // The LLM-graded capability rubric (Genesis P7, DEC-025 #7). An ADDITIVE
    // sixth axis on top of the deterministic floor: flag-gated off by default
    // (`not_applicable`, vacuous pass — the floor stays the baseline), graded
    // when `ARGO_GENESIS_LLM_RUBRIC=1`, and degraded fail-safe to `needs_review`
    // when the LLM grader can't run. Uses the SAME behaviour-contract text the
    // eval floor scores + the SAME judge projection the safety judge sees, so
    // both LLM axes view one consistent teammate.
    let rubric = run_rubric_axis(judge_input_for_plan(plan), &behaviour_contract_text(plan)).await;
    // The role-contract confirmation axis — derived truthfully from the SAME
    // manifest the P2 capability fence reads, resolved against the operator's
    // echoed digest.
    let contract_input = ContractGateInput::from_plan(plan, confirmed_digest);
    GateVerdict::compose(eval, comprehension, sop, safety, contract_input).with_rubric(rubric)
}

/// Test-only convenience wrapper: the disk-fence gate with no operator
/// confirmation. Production always threads the operator's confirmation digest
/// through [`run_hire_gate_from_dir_with_confirmation`] (the `POST
/// /api/v1/workers` fence), so this thin no-digest form exists only for the
/// gate-parity tests that exercise the eval/comprehension/safety axes.
#[cfg(test)]
pub(super) async fn run_hire_gate_from_dir(vertical_dir: &Path) -> GateVerdict {
    run_hire_gate_from_dir_with_confirmation(vertical_dir, None).await
}

/// Re-run the IDENTICAL synchronous hire gate against a teammate whose
/// Genesis vertical is already written to `vertical_dir` (the hire-commit
/// fence), AND resolve the operator's role-contract confirmation (DEC-025 #1
/// + #4) against the contract derived from the on-disk manifest.
///
/// Reconstructs the deterministic seeds + behaviour contract + safety-judge
/// projection + truthful role contract from disk so the SSE path and this
/// fence can never diverge — the digest the operator saw on the SSE payload
/// confirms byte-identically here. `confirmed_digest` `None` → the contract
/// is unconfirmed and the teammate holds in `needs_review`. A
/// missing/unreadable manifest fails closed to a blocked/needs-review verdict
/// (no seeds + degraded safety + no contract to confirm).
pub(super) async fn run_hire_gate_from_dir_with_confirmation(
    vertical_dir: &Path,
    confirmed_digest: Option<&str>,
) -> GateVerdict {
    let Ok(inputs) = gate_inputs_from_dir(vertical_dir) else {
        // Manifest unreadable → cannot verify → fail closed (zero seeds, zero
        // comprehension questions answered, degraded safety, no contract to
        // confirm — a supplied digest cannot match a non-existent contract).
        // The SOP axis is not-applicable here: with no manifest there is no
        // teammate to grade, and the four real axes already fail closed.
        // The four deterministic axes already fail closed (zero seeds, zero
        // comprehension, degraded safety, no contract). The rubric is
        // not-applicable — there is no teammate to grade — so it adds nothing;
        // the verdict is already blocked/needs-review.
        return GateVerdict::compose(
            EvalFloorOutcome::from_counts(0, 0),
            ComprehensionOutcome::from_counts(0, 0),
            SopComprehensionOutcome::not_applicable(),
            SafetySignal::Unavailable {
                reason: "vertical manifest unreadable at hire-commit gate".to_string(),
            },
            ContractGateInput::resolve(None, confirmed_digest),
        )
        .with_rubric(argo_genesis::RubricOutcome::not_applicable());
    };
    let questions = build_comprehension_questions(&inputs.charter, &inputs.action_vocabulary);
    let eval = run_eval_floor(&inputs.seeds, &inputs.contract_text).await;
    let comprehension = run_comprehension_floor(&questions, &inputs.contract_text).await;
    // The SOP-comprehension axis (Genesis P6). Re-reads the SAME uploaded SOP
    // knowledge + quiz the SSE path wrote, so this fence re-grades the
    // byte-identical doc-comprehension verdict — no per-surface divergence.
    let sop = run_sop_comprehension_floor(vertical_dir).await;
    let safety = run_safety_signal(inputs.judge_input()).await;
    // The LLM rubric (Genesis P7), re-graded against the disk-reconstructed
    // behaviour contract + judge projection so the hire-commit fence reaches the
    // SAME rubric verdict the SSE path did (no per-surface divergence). Flag-
    // gated off by default → not-applicable; degraded fail-safe when on but the
    // grader can't run.
    let rubric = run_rubric_axis(inputs.judge_input(), &inputs.contract_text).await;
    // The truthful contract is derived from the IDENTICAL on-disk manifest the
    // P2 fence reads, so the SSE path and this fence confirm byte-identical
    // contracts (the digest the operator saw on the SSE payload matches here).
    let contract_input = ContractGateInput::resolve(Some(inputs.role_contract()), confirmed_digest);
    GateVerdict::compose(eval, comprehension, sop, safety, contract_input).with_rubric(rubric)
}

/// Grade the doc-grounded SOP-comprehension quiz (Genesis P6 — the moat).
///
/// Reads the ingested SOP knowledge + the persisted quiz from the teammate's
/// `<vertical_dir>/knowledge/` (written by the SSE handler at generation time);
/// the teammate's "answer" to every quiz question is its ingested knowledge
/// text, graded through the SAME canonical `argo_eval::run_eval` +
/// [`ContractSubject`] the eval-floor + role-contract comprehension use — so
/// there is no parallel scorer.
///
/// Resolution (fail-closed when a SOP is present):
/// - **No SOP uploaded** (`knowledge: None`) → `not_applicable` (vacuous pass,
///   DEC-025 #5 SOP is optional).
/// - **SOP present, quiz non-empty** → grade each question: it passes iff every
///   doc-specific keyword is a substring of the ingested knowledge. A teammate
///   that absorbed the SOP passes; an empty/role-only one fails.
/// - **SOP present but quiz empty** (a doc whose every salient term overlapped
///   the role, or a corrupt re-read) → an applicable outcome with zero
///   questions, which `from_counts` makes **fail-closed**: a present doc the
///   teammate can't be quizzed on must NOT wave it through.
async fn run_sop_comprehension_floor(vertical_dir: &Path) -> SopComprehensionOutcome {
    let disk = read_sop_knowledge(vertical_dir);
    let Some(knowledge) = disk.knowledge else {
        // No SOP was uploaded for this teammate.
        return SopComprehensionOutcome::not_applicable();
    };
    // A SOP IS present. Grade the persisted quiz against the ingested
    // knowledge. An empty quiz (no role-distinct term / corrupt re-read) yields
    // total==0 → fail-closed.
    let knowledge_text = knowledge.joined_text();
    grade_sop_quiz(&disk.quiz, &knowledge_text).await
}

/// Grade an SOP quiz against a teammate's ingested-knowledge text through the
/// canonical runner. Factored out so tests can exercise the discrimination
/// property directly (absorbed vs not) without touching disk.
pub(super) async fn grade_sop_quiz(
    quiz: &[SopQuestion],
    knowledge_text: &str,
) -> SopComprehensionOutcome {
    if quiz.is_empty() {
        // Applicable-but-unquizzable → fail-closed.
        return SopComprehensionOutcome::from_counts(0, 0);
    }
    let tasks: Vec<EvalTask> = quiz
        .iter()
        .map(|q| EvalTask {
            name: q.id.clone(),
            input: q.question.clone(),
            expected: String::new(),
            metric: EvalMetric::SubstringContains {
                must_contain: q.expected_keywords.clone(),
            },
        })
        .collect();
    let subject = ContractSubject {
        contract_text: knowledge_text.to_lowercase(),
    };
    let (_records, summary) = run_eval(&subject, &tasks).await;
    let passed = summary.pass_rate.mul_add(summary.n as f64, 0.0).round() as usize;
    SopComprehensionOutcome::from_counts(passed, summary.n)
}

/// Grade the teammate's starter seeds through the canonical runner against
/// its behaviour contract. Zero seeds → fail-closed via
/// [`EvalFloorOutcome::from_counts`].
async fn run_eval_floor(seeds: &[DeterministicEvalInput], contract_text: &str) -> EvalFloorOutcome {
    let tasks: Vec<EvalTask> = seeds
        .iter()
        .map(|s| EvalTask {
            name: s.id.clone(),
            input: s.task.clone(),
            // The metric scores the subject's output (the behaviour
            // contract) against the seed's rubric keywords — AND semantics.
            expected: String::new(),
            metric: EvalMetric::SubstringContains {
                must_contain: s.rubric_keywords.clone(),
            },
        })
        .collect();
    let subject = ContractSubject {
        contract_text: contract_text.to_string(),
    };
    let (_records, summary) = run_eval(&subject, &tasks).await;
    let passed = summary.pass_rate.mul_add(summary.n as f64, 0.0).round() as usize;
    EvalFloorOutcome::from_counts(passed, summary.n)
}

/// Grade the role-contract comprehension questions through the SAME
/// canonical runner + deterministic [`ContractSubject`] as the eval floor:
/// each question "passes" when every one of its rubric keywords is a
/// substring of the teammate's behaviour contract (AND semantics). Zero
/// questions → fail-closed via [`ComprehensionOutcome::from_counts`] (the
/// all-must-pass floor treats an empty question set as a failure).
async fn run_comprehension_floor(
    questions: &[ComprehensionQuestion],
    contract_text: &str,
) -> ComprehensionOutcome {
    let tasks: Vec<EvalTask> = questions
        .iter()
        .map(|q| EvalTask {
            name: q.id.clone(),
            input: q.question.clone(),
            expected: String::new(),
            metric: EvalMetric::SubstringContains {
                must_contain: q.expected_keywords.clone(),
            },
        })
        .collect();
    let subject = ContractSubject {
        contract_text: contract_text.to_string(),
    };
    let (_records, summary) = run_eval(&subject, &tasks).await;
    let passed = summary.pass_rate.mul_add(summary.n as f64, 0.0).round() as usize;
    ComprehensionOutcome::from_counts(passed, summary.n)
}

/// Deterministic eval subject: answers every seed task with the teammate's
/// lowercased behaviour contract. A rich teammate's contract names its
/// actions + domain (so the rubric keywords appear and the seed passes); an
/// empty stub's contract does not (so the seed fails). Provider-free.
struct ContractSubject {
    contract_text: String,
}

#[async_trait]
impl EvalSubject for ContractSubject {
    async fn run(&self, _task: &EvalTask) -> ArgoResult<String> {
        Ok(self.contract_text.clone())
    }
}

/// Test-only gate-verdict injection. Entirely `#[cfg(test)]`, so it does not
/// exist in release builds — there is no production code path that can force
/// the gate to pass. A test sets a one-shot forced [`GateVerdict`]; the next
/// [`run_hire_gate`] call consumes it (and clears it) so a forced `Pass` can't
/// leak across tests. Used by the first-hire end-to-end tests that drive a
/// *gate-cleared* hire to assert downstream persistence / name handling /
/// tenant visibility; tests that exercise the REAL gate (the
/// `first_hire_blocks_*` suite) set no override.
#[cfg(test)]
pub(super) mod test_overrides {
    use argo_genesis::{GateState, GateVerdict};
    use std::sync::Mutex;

    static FORCED: Mutex<Option<GateVerdict>> = Mutex::new(None);

    /// Force the next [`super::run_hire_gate`] call to return `verdict`
    /// (consumed once). A test holds the gateway-wide env lock while it sets
    /// this and drives the hire, so concurrent tests can't observe each
    /// other's override.
    pub(crate) fn set_forced_verdict(verdict: GateVerdict) {
        if let Ok(mut guard) = FORCED.lock() {
            *guard = Some(verdict);
        }
    }

    /// A fully gate-cleared, operator-`confirmed` `Pass` verdict with no SOP
    /// and no rubric axis applicable — the base both `force_*` convenience fns
    /// build from (the common shape for tests not exercising those axes).
    fn base_pass_verdict() -> GateVerdict {
        GateVerdict {
            state: GateState::Pass,
            eval_passed: 10,
            eval_total: 10,
            eval_pass_rate: 1.0,
            comprehension_passed: 5,
            comprehension_total: 5,
            comprehension_pass_rate: 1.0,
            sop_applicable: false,
            sop_passed: 0,
            sop_total: 0,
            sop_pass_rate: 0.0,
            safety_verdict: "allow".to_string(),
            rubric_applicable: false,
            rubric_degraded: false,
            rubric_score: 0.0,
            rubric_min_dimension: 0.0,
            rubric_verdict: "not_applicable".to_string(),
            role_contract: None,
            contract_digest: String::new(),
            contract_confirmation: "confirmed".to_string(),
            reasons: Vec::new(),
        }
    }

    /// Convenience: force a clean `Pass` verdict (the common case for tests
    /// that just need the hire to commit).
    pub(crate) fn force_pass() {
        set_forced_verdict(base_pass_verdict());
    }

    /// Convenience: force a HARD `Blocked` verdict whose eval floor genuinely
    /// FAILED (4/10 < the 50% bar) — the in-memory analogue of the disk test
    /// `disk_gate_hard_blocks_a_teammate_that_fails_its_evals`, which truncates
    /// the on-disk seed file so no seed grades. The first-hire seam's plan is
    /// in-memory only (a catalog `vertical_id`, no written `.genesis/` dir), and
    /// a classifiable archetype always synthesises a manifest whose own seed
    /// keywords appear in its contract, so a real first-hire plan can never
    /// score below the floor through `genesis_plan` — the only way to exercise
    /// the failed-eval-floor → HARD-block routing AT THIS SEAM is to inject the
    /// degenerate verdict here. The reason carries the canonical
    /// "deterministic eval floor failed" string `GateVerdict::compose` emits, so
    /// the block reason cites the eval floor exactly as a live failure would.
    /// One-shot, consumed by the next `run_hire_gate` call under the env lock.
    pub(crate) fn force_eval_floor_block() {
        let mut verdict = base_pass_verdict();
        verdict.state = GateState::Blocked;
        verdict.eval_passed = 4;
        verdict.eval_pass_rate = 0.4;
        verdict.reasons = vec![
            "deterministic eval floor failed: 4/10 seeds passed (40% < 50% required)".to_string(),
        ];
        set_forced_verdict(verdict);
    }

    /// Clear any pending forced verdict (defensive reset under the env lock).
    pub(crate) fn clear_forced_verdict() {
        if let Ok(mut guard) = FORCED.lock() {
            *guard = None;
        }
    }

    pub(super) fn take_forced_verdict() -> Option<GateVerdict> {
        FORCED.lock().ok().and_then(|mut g| g.take())
    }
}

#[cfg(test)]
#[path = "routes_genesis_gate_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "routes_genesis_sop_gate_tests.rs"]
mod sop_gate_tests;
