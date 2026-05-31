//! Synchronous hire-gate primitives (DEC-025 / Genesis P0).
//!
//! Before a Genesis-generated teammate may be hired into a paying
//! customer's business it MUST pass two real, fail-closed checks:
//!
//! 1. **Deterministic eval floor** — the 10 generated starter seeds are
//!    graded against the teammate's *behaviour contract* (system prompt +
//!    action vocabulary). A thin/empty worker — no actions, a stub prompt —
//!    fails because the seed's rubric keywords never appear. This is a floor
//!    against thin workers, not a competence proof (the LLM-graded rubric is
//!    a documented fast-follow), but it is REAL: it really runs and really
//!    blocks. The actual scoring is delegated to the canonical
//!    `argo_eval::run_eval` runner by the gateway orchestrator — this module
//!    only builds the deterministic inputs so there is no parallel scorer.
//! 2. **LLM semantic safety judge** — [`crate::genesis_judge`], fail-closed
//!    to `Block` on any transport/parse error, run by the orchestrator.
//!
//! This module is the *pure* half: it turns a [`GenesisPlan`] into the
//! deterministic eval inputs and the safety-judge projection, and composes
//! the final [`GateVerdict`]. The provider-touching half (running the eval
//! subject + dialling the judge) lives in the gateway handler that already
//! holds the `argo-eval` + transport dependencies, so `argo-genesis` gains
//! no new crate edge.
//!
//! ## Why the behaviour-contract text is the eval subject
//!
//! The deterministic floor has no provider. A seed "passes" when the
//! teammate's rendered behaviour contract contains the seed's rubric keywords
//! (derived from the role + its actions): a richly-specified teammate's
//! contract names its actions and domain, an empty stub's does not — the
//! "gameable by keyword-parroting, but a true floor against empty workers"
//! property the design doc calls for. The LLM rubric (P7, [`crate::rubric`])
//! is the additive axis that closes that gameability gap.

use crate::eval_seed::{self, EvalSeed};
use crate::hire_gate_contract::{ContractConfirmation, ContractGateInput};
use crate::pipeline::GenesisPlan;
use crate::prompt_writer::build_system_prompt;

/// Minimum fraction of starter seeds that must pass for the deterministic
/// eval floor to clear. The design doc fixes the per-seed bar at
/// `score >= 0.5`; a teammate that cannot clear at least this fraction of
/// its *own* starter suite is blocked from normal operation.
pub const EVAL_PASS_RATE_FLOOR: f64 = 0.5;

/// One deterministic eval input: the task text the runner feeds the
/// subject, plus the rubric keywords that the subject's output must
/// contain (AND semantics — the `SubstringContains` metric).
///
/// Kept transport- and `argo-eval`-free so this crate owns no parallel
/// eval types; the gateway maps each entry onto an `argo_eval::EvalTask`
/// with `EvalMetric::SubstringContains { must_contain: rubric_keywords }`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeterministicEvalInput {
    /// Stable seed id (becomes the `EvalTask` name).
    pub id: String,
    /// The seed task prompt.
    pub task: String,
    /// Lowercased rubric keywords; ALL must appear in the subject output.
    pub rubric_keywords: Vec<String>,
}

/// The teammate's full eval inputs: the deterministic seed tasks plus the
/// behaviour-contract text the deterministic subject answers every task
/// with. Built straight from the [`GenesisPlan`] so the SSE path and the
/// hire-commit fence derive byte-identical inputs from the same plan.
#[derive(Debug, Clone)]
pub struct GateEvalInputs {
    /// The deterministic seed tasks.
    pub seeds: Vec<DeterministicEvalInput>,
    /// The behaviour contract the deterministic subject returns for every
    /// task. Lowercased once here so substring matching is case-insensitive.
    pub contract_text: String,
}

/// Where a gate verdict came down. Honest about the three distinct
/// operational states the design doc requires — never a silent "pass".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GateState {
    /// Every check passed; the teammate may be hired/activated.
    Pass,
    /// A check said *no* (evals failed below the floor, or the safety judge
    /// returned `Block`). Hard fail — surface the reason; do NOT hire.
    Blocked,
    /// A check could not *run* (judge/provider unavailable or errored). Fail
    /// SAFE: never auto-activate, require human review, hold in dry-run, and
    /// audit the degradation. NOT a pass.
    NeedsReview,
}

impl GateState {
    /// The lowercase wire token surfaced in the SSE payload + audit row.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pass => "pass",
            Self::Blocked => "blocked",
            Self::NeedsReview => "needs_review",
        }
    }

    /// `true` only for [`GateState::Pass`] — the single state in which a
    /// teammate may be hired/activated. Both `Blocked` and `NeedsReview`
    /// return `false`, so the hire fence is fail-closed by construction.
    #[must_use]
    pub fn allows_hire(self) -> bool {
        matches!(self, Self::Pass)
    }
}

/// Composite outcome of the synchronous hire gate. Carries the genuine
/// pass/fail signal plus the evidence the operator (and the audit ledger)
/// needs to understand *why*.
#[derive(Debug, Clone)]
pub struct GateVerdict {
    /// Pass / Blocked / NeedsReview.
    pub state: GateState,
    /// Honest count of starter seeds that scored `>= 0.5`.
    pub eval_passed: usize,
    /// Total starter seeds run.
    pub eval_total: usize,
    /// Observed pass rate (`eval_passed / eval_total`, 0.0 when no seeds).
    pub eval_pass_rate: f64,
    /// Honest count of role-contract comprehension questions the teammate's
    /// contract answered (all keywords present).
    pub comprehension_passed: usize,
    /// Total comprehension questions asked.
    pub comprehension_total: usize,
    /// Observed comprehension pass rate (0.0 when no questions).
    pub comprehension_pass_rate: f64,
    /// Whether an SOP document was uploaded at onboarding (Genesis P6,
    /// DEC-025 #4 + #5). When `false` the SOP axis is *not applicable* — it
    /// passes vacuously and the other four axes still gate.
    pub sop_applicable: bool,
    /// Honest count of doc-grounded SOP-comprehension questions the teammate's
    /// ingested knowledge answered (all doc-specific keywords present).
    pub sop_passed: usize,
    /// Total SOP-comprehension questions asked (0 when no SOP uploaded).
    pub sop_total: usize,
    /// Observed SOP-comprehension pass rate (0.0 when no questions).
    pub sop_pass_rate: f64,
    /// Safety-judge token: `allow` / `allow_with_disclaimer` / `block` / `unavailable`.
    pub safety_verdict: String,
    /// LLM-graded capability rubric (Genesis P7, DEC-025 #7) axis — see
    /// [`crate::rubric`] + [`Self::with_rubric`]. `false` when flag-gated off
    /// (vacuous pass; the deterministic floor stays the baseline).
    pub rubric_applicable: bool,
    /// The rubric was applicable but DEGRADED (grader could not run / returned an
    /// unusable grade) → holds in `needs_review` (fail-safe), never a silent pass.
    pub rubric_degraded: bool,
    /// The rubric's weighted-mean capability score (`0.0` when n/a or degraded).
    pub rubric_score: f64,
    /// The rubric's lowest single-dimension score — the weakest facet found.
    pub rubric_min_dimension: f64,
    /// Wire token: `not_applicable` / `pass` / `block` / `unavailable`.
    pub rubric_verdict: String,
    /// The teammate's truthful, operator-confirmable role contract — derived
    /// from its actual bound capability profile (DEC-025 #4). Surfaced for
    /// operator review before activation. `None` only when no plan/manifest
    /// was available to derive it (the gate then has nothing to confirm and
    /// cannot pass).
    pub role_contract: Option<crate::role_contract::RoleContract>,
    /// SHA-256 of [`Self::role_contract`]'s truthful capability projection.
    /// The operator echoes this back to confirm; the wire surface for the
    /// "what can this teammate do" review screen. Empty when no contract.
    pub contract_digest: String,
    /// The operator-confirmation state for the role contract (`confirmed` /
    /// `unconfirmed` / `mismatch`). Honest on the wire so the SPA can render
    /// "awaiting your confirmation" vs "confirmation out of date".
    pub contract_confirmation: String,
    /// Human-readable reasons for a non-pass (empty on pass). One per line
    /// is the gateway's join convention.
    pub reasons: Vec<String>,
}

impl GateVerdict {
    /// Compose the final verdict from the five independent signals: the
    /// deterministic eval floor, the role-contract comprehension gate, the
    /// **SOP-comprehension gate** (Genesis P6), the semantic safety judge, and
    /// the **operator confirmation of the role contract** (DEC-025 #1 + #4 +
    /// #5).
    ///
    /// `contract` is the teammate's truthful role contract (derived from its
    /// actual bound capability profile) plus the operator's confirmation
    /// state for it. `sop` is the doc-grounded comprehension outcome — *not
    /// applicable* (vacuous pass) when no SOP was uploaded, fail-closed when
    /// one was.
    ///
    /// Precedence (fail-closed):
    /// 1. A hard `Block` from any check that *ran* → [`GateState::Blocked`]:
    ///    eval floor below the bar, a comprehension governance miss, **an
    ///    uploaded SOP the teammate failed to comprehend**, a safety-judge
    ///    `Block`, **or a contract-confirmation `Mismatch`** (a forged/stale
    ///    confirmation must never activate a teammate).
    /// 2. Else a check that could not run / has not yet happened →
    ///    [`GateState::NeedsReview`]: a degraded safety signal, **or an
    ///    `Unconfirmed` contract** (human-review-before-activation is ON by
    ///    default, so the teammate holds for review until the operator
    ///    confirms the surfaced contract).
    /// 3. Else [`GateState::Pass`].
    ///
    /// Comprehension + SOP-comprehension are all-must-pass invariant gates, so
    /// any miss is a hard block. Operator confirmation is required for
    /// activation, so an unconfirmed contract can NEVER reach `Pass`. The
    /// **LLM rubric** (Genesis P7) is an ADDITIVE sixth axis folded in by
    /// [`Self::with_rubric`] AFTER this call (it can only tighten the verdict).
    #[must_use]
    pub fn compose(
        eval: EvalFloorOutcome,
        comprehension: crate::comprehension::ComprehensionOutcome,
        sop: crate::sop::SopComprehensionOutcome,
        safety: SafetySignal,
        contract: ContractGateInput,
    ) -> Self {
        let mut reasons = Vec::new();
        let mut blocked = false;
        let mut needs_review = false;

        if !eval.passed_floor {
            blocked = true;
            reasons.push(format!(
                "deterministic eval floor failed: {}/{} seeds passed ({:.0}% < {:.0}% required)",
                eval.passed,
                eval.total,
                eval.pass_rate * 100.0,
                EVAL_PASS_RATE_FLOOR * 100.0
            ));
        }

        if !comprehension.passed_floor {
            blocked = true;
            reasons.push(format!(
                "role-contract comprehension failed: {}/{} questions answered \
                 (all required — the contract did not demonstrate a scope or \
                 governance boundary)",
                comprehension.passed, comprehension.total,
            ));
        }

        // The SOP-comprehension axis (Genesis P6 — the moat). Only gates when a
        // SOP was actually uploaded (`applicable`); otherwise it is a vacuous
        // pass (DEC-025 #5, SOP is optional). When applicable, it is
        // all-must-pass: a teammate that did not demonstrably absorb the
        // uploaded document is hard-blocked — no hire without comprehension of
        // the SPECIFIC SOP.
        if sop.applicable && !sop.passed_floor {
            blocked = true;
            reasons.push(format!(
                "SOP-comprehension failed: {}/{} doc-grounded questions answered \
                 (all required — the teammate did not demonstrate it absorbed the \
                 uploaded procedure document)",
                sop.passed, sop.total,
            ));
        }

        let safety_verdict = match &safety {
            SafetySignal::Allow => "allow".to_string(),
            SafetySignal::AllowWithDisclaimer { reason } => {
                reasons.push(format!("safety judge attached a disclaimer: {reason}"));
                "allow_with_disclaimer".to_string()
            }
            SafetySignal::Block { reason } => {
                blocked = true;
                reasons.push(format!("safety judge blocked the teammate: {reason}"));
                "block".to_string()
            }
            SafetySignal::Unavailable { reason } => {
                needs_review = true;
                reasons.push(format!(
                    "safety judge unavailable (degraded → needs_review, held in dry-run): {reason}"
                ));
                "unavailable".to_string()
            }
        };

        // The role-contract confirmation axis (DEC-025 #1 + #4). A truthful
        // contract that the operator has not yet confirmed holds the teammate
        // in needs_review; a confirmation that does not match the truthful
        // contract is a hard block.
        let (contract_digest, confirmation_token) = match &contract.confirmation {
            ContractConfirmation::Confirmed => {
                (contract.digest(), "confirmed".to_string())
            }
            ContractConfirmation::Unconfirmed => {
                needs_review = true;
                reasons.push(
                    "role contract awaiting operator confirmation (human-review-before-\
                     activation is on): the teammate stays held until the operator \
                     confirms the surfaced 'what can this teammate do' contract"
                        .to_string(),
                );
                (contract.digest(), "unconfirmed".to_string())
            }
            ContractConfirmation::Mismatch { supplied, expected } => {
                blocked = true;
                reasons.push(format!(
                    "role-contract confirmation mismatch: operator confirmed digest \
                     {supplied} but the teammate's actual contract is {expected} — the \
                     confirmation is stale or forged; refusing to activate"
                ));
                (expected.clone(), "mismatch".to_string())
            }
        };

        // Belt-and-braces: a missing contract (no manifest to derive from)
        // means there is nothing to confirm, so it can never pass.
        if contract.role_contract.is_none() {
            needs_review = true;
            reasons.push(
                "no role contract could be derived (manifest unavailable) — cannot \
                 confirm scope, holding for review"
                    .to_string(),
            );
        }

        let state = if blocked {
            GateState::Blocked
        } else if needs_review {
            GateState::NeedsReview
        } else {
            GateState::Pass
        };

        Self {
            state,
            eval_passed: eval.passed,
            eval_total: eval.total,
            eval_pass_rate: eval.pass_rate,
            comprehension_passed: comprehension.passed,
            comprehension_total: comprehension.total,
            comprehension_pass_rate: comprehension.pass_rate,
            sop_applicable: sop.applicable,
            sop_passed: sop.passed,
            sop_total: sop.total,
            sop_pass_rate: sop.pass_rate,
            safety_verdict,
            // The LLM rubric (P7) is folded in by `with_rubric`; default n/a.
            rubric_applicable: false,
            rubric_degraded: false,
            rubric_score: 0.0,
            rubric_min_dimension: 0.0,
            rubric_verdict: "not_applicable".to_string(),
            role_contract: contract.role_contract,
            contract_digest,
            contract_confirmation: confirmation_token,
            reasons,
        }
    }
}

/// Result of running the deterministic eval floor (computed by the
/// orchestrator from the canonical runner's summary).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EvalFloorOutcome {
    /// Seeds that scored `>= 0.5`.
    pub passed: usize,
    /// Total seeds run.
    pub total: usize,
    /// `passed / total` (0.0 when `total == 0`).
    pub pass_rate: f64,
    /// `true` iff `pass_rate >= EVAL_PASS_RATE_FLOOR` AND at least one seed
    /// ran. Zero seeds is itself a floor failure — a teammate with no
    /// starter suite is exactly the thin worker the floor exists to catch.
    pub passed_floor: bool,
}

impl EvalFloorOutcome {
    /// Build from a raw pass/total count, deriving the rate + floor verdict.
    #[must_use]
    pub fn from_counts(passed: usize, total: usize) -> Self {
        let pass_rate = if total == 0 {
            0.0
        } else {
            passed as f64 / total as f64
        };
        let passed_floor = total > 0 && pass_rate >= EVAL_PASS_RATE_FLOOR;
        Self {
            passed,
            total,
            pass_rate,
            passed_floor,
        }
    }
}

/// The semantic safety-judge signal, normalised for [`GateVerdict::compose`].
///
/// Distinguishes a judge that said *no* ([`SafetySignal::Block`], a hard
/// fail) from a judge that could not *run* ([`SafetySignal::Unavailable`], a
/// degraded → needs-review). The gateway maps the LLM judge's
/// [`crate::genesis_judge::GenesisJudgeVerdict`] onto this, and uses
/// `Unavailable` for the degraded path (provider missing / quota-out).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SafetySignal {
    /// Judge ran and allowed.
    Allow,
    /// Judge ran and allowed with a disclaimer.
    AllowWithDisclaimer {
        /// The disclaimer text.
        reason: String,
    },
    /// Judge ran and blocked. Hard fail.
    Block {
        /// Why the judge blocked.
        reason: String,
    },
    /// Judge could not run (degraded). Fail SAFE → needs review.
    Unavailable {
        /// Why the judge could not run.
        reason: String,
    },
}

/// Build the deterministic eval inputs for a plan: the seed tasks (from the
/// canonical [`eval_seed::generate`], so the floor grades the SAME 10 seeds
/// written to `eval-seed.toml`) plus the behaviour-contract text the
/// deterministic subject answers every task with.
///
/// # Errors
///
/// Propagates [`eval_seed::generate`] failures (only on a degenerate empty
/// role, which the SSE handler already rejects upstream).
///
/// # Examples
///
/// ```
/// # fn demo() -> Result<(), argo_genesis::GenesisError> {
/// let plan = argo_genesis::plan("be my marketing intern, run weekly campaigns", "maya")?;
/// let inputs = argo_genesis::build_gate_eval_inputs(&plan)?;
/// assert_eq!(inputs.seeds.len(), 10);
/// assert!(!inputs.contract_text.is_empty());
/// # Ok(()) }
/// ```
pub fn build_gate_eval_inputs(plan: &GenesisPlan) -> Result<GateEvalInputs, crate::GenesisError> {
    let actions = plan
        .manifest
        .actions
        .as_ref()
        .map(eval_seed::suggested_actions_from_manifest)
        .unwrap_or_default();
    let seeds =
        eval_seed::generate(&plan.charter, &actions).map_err(|e| crate::GenesisError::Validation {
            stage: "hire_gate.eval_seed.generate",
            detail: e.to_string(),
        })?;
    let contract_text = behaviour_contract_text(plan);
    Ok(GateEvalInputs {
        seeds: seeds.iter().map(to_input).collect(),
        contract_text,
    })
}

pub(crate) fn to_input(seed: &EvalSeed) -> DeterministicEvalInput {
    DeterministicEvalInput {
        id: seed.id.clone(),
        task: seed.task.clone(),
        rubric_keywords: seed.rubric_keywords.clone(),
    }
}

/// The teammate's behaviour contract, lowercased: the rendered system
/// prompt (charter, role, safety rules) plus the action vocabulary and
/// every action description. This is what the deterministic eval subject
/// "answers" each seed with — an empty/thin teammate produces a contract
/// that lacks the role-derived rubric keywords and so fails the floor.
#[must_use]
pub fn behaviour_contract_text(plan: &GenesisPlan) -> String {
    let mut out = build_system_prompt(&plan.charter, plan.archetype, &plan.persona);
    out.push('\n');
    out.push_str(&plan.manifest.vertical.name);
    out.push('\n');
    out.push_str(&plan.manifest.vertical.description);
    if let Some(actions) = plan.manifest.actions.as_ref() {
        for (name, decl) in &actions.actions {
            out.push('\n');
            out.push_str(name);
            out.push(' ');
            out.push_str(&decl.description);
        }
    }
    out.to_lowercase()
}

#[cfg(test)]
#[path = "hire_gate_tests.rs"]
mod tests;
