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

use super::routes_genesis_stream_llm::{complete_once, resolve_provider_id};
use argo_genesis::{
    build_comprehension_questions, build_gate_eval_inputs, gate_inputs_from_dir,
    judge_genesis_draft, judge_input_for_plan, ComprehensionOutcome, ComprehensionQuestion,
    DeterministicEvalInput, EvalFloorOutcome, GateVerdict, GenesisJudgeClient, GenesisJudgeVerdict,
    GenesisPlan, JudgeInput, SafetySignal,
};
use argo_eval::{run_eval, EvalMetric, EvalSubject, EvalTask};
use argo_types::ArgoResult;
use async_trait::async_trait;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

/// Run the full synchronous hire gate against a resolved [`GenesisPlan`].
///
/// Deterministic part (the eval floor) always runs. The safety judge is
/// dialled over the live provider; any transport/provider failure degrades
/// to `needs_review` rather than blocking or silently passing.
///
/// Never errors at the type level — a failure to *build* the eval inputs
/// (degenerate empty role, already rejected upstream by the SSE handler)
/// collapses to an empty seed set, which the floor treats as a fail-closed
/// `Blocked`.
pub(super) async fn run_hire_gate(plan: &GenesisPlan) -> GateVerdict {
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
    let safety = run_safety_signal(judge_input_for_plan(plan)).await;
    GateVerdict::compose(eval, comprehension, safety)
}

/// Re-run the IDENTICAL synchronous hire gate against a teammate whose
/// Genesis vertical is already written to `vertical_dir` (the hire-commit
/// fence). Reconstructs the deterministic seeds + behaviour contract +
/// safety-judge projection from disk so the SSE path and the hire fence
/// can never diverge. A missing/unreadable manifest fails closed to a
/// blocked verdict (no seeds + degraded safety).
pub(super) async fn run_hire_gate_from_dir(vertical_dir: &Path) -> GateVerdict {
    let Ok(inputs) = gate_inputs_from_dir(vertical_dir) else {
        // Manifest unreadable → cannot verify → fail closed (zero seeds, zero
        // comprehension questions answered, degraded safety).
        return GateVerdict::compose(
            EvalFloorOutcome::from_counts(0, 0),
            ComprehensionOutcome::from_counts(0, 0),
            SafetySignal::Unavailable {
                reason: "vertical manifest unreadable at hire-commit gate".to_string(),
            },
        );
    };
    let questions = build_comprehension_questions(&inputs.charter, &inputs.action_vocabulary);
    let eval = run_eval_floor(&inputs.seeds, &inputs.contract_text).await;
    let comprehension = run_comprehension_floor(&questions, &inputs.contract_text).await;
    let safety = run_safety_signal(inputs.judge_input()).await;
    GateVerdict::compose(eval, comprehension, safety)
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

/// Dial the LLM safety judge and normalise its outcome into a
/// [`SafetySignal`]. A transport/provider failure (the common CI / no-key
/// case) becomes [`SafetySignal::Unavailable`] (degraded → needs_review),
/// distinct from a real `block`.
async fn run_safety_signal(input: JudgeInput<'_>) -> SafetySignal {
    let provider_id = resolve_provider_id();
    let client = GateJudgeClient::new(provider_id);
    // `judge_genesis_draft` fail-closes to `Block` on transport error; we
    // want to tell "judge said no" apart from "judge couldn't run", so we
    // inspect the client's own transport-failure flag afterward.
    let verdict = match judge_genesis_draft(&input, &client).await {
        Ok(v) => v,
        Err(e) => {
            return SafetySignal::Unavailable {
                reason: format!("safety judge invocation error: {e}"),
            }
        }
    };
    if client.transport_failed() {
        // The judge fell through to its conservative `Block` because the
        // provider call errored — that is a *could-not-run*, so degrade to
        // needs_review (audited, held in dry-run) instead of a hard block.
        let reason = match verdict {
            GenesisJudgeVerdict::Block { reason } => reason,
            _ => "provider transport failed".to_string(),
        };
        return SafetySignal::Unavailable { reason };
    }
    match verdict {
        GenesisJudgeVerdict::Allow => SafetySignal::Allow,
        GenesisJudgeVerdict::AllowWithDisclaimer { reason } => {
            SafetySignal::AllowWithDisclaimer { reason }
        }
        GenesisJudgeVerdict::Block { reason } => SafetySignal::Block { reason },
    }
}

/// [`GenesisJudgeClient`] over the shared provider transport (same machinery
/// as the archetype classifier). Records whether the underlying provider
/// round-trip errored so the caller can distinguish a real `block` verdict
/// from a degraded could-not-run.
struct GateJudgeClient {
    provider_id: String,
    transport_failed: AtomicBool,
}

impl GateJudgeClient {
    fn new(provider_id: String) -> Self {
        Self {
            provider_id,
            transport_failed: AtomicBool::new(false),
        }
    }

    fn transport_failed(&self) -> bool {
        self.transport_failed.load(Ordering::Relaxed)
    }
}

#[async_trait]
impl GenesisJudgeClient for GateJudgeClient {
    async fn judge(&self, prompt: &str) -> ArgoResult<String> {
        match complete_once(&self.provider_id, prompt).await {
            Ok(text) => Ok(text),
            Err(e) => {
                self.transport_failed.store(true, Ordering::Relaxed);
                Err(e)
            }
        }
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

    /// Convenience: force a clean `Pass` verdict (the common case for tests
    /// that just need the hire to commit).
    pub(crate) fn force_pass() {
        set_forced_verdict(GateVerdict {
            state: GateState::Pass,
            eval_passed: 10,
            eval_total: 10,
            eval_pass_rate: 1.0,
            comprehension_passed: 5,
            comprehension_total: 5,
            comprehension_pass_rate: 1.0,
            safety_verdict: "allow".to_string(),
            reasons: Vec::new(),
        });
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
