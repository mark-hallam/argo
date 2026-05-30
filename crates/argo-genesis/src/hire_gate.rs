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
//! teammate's rendered behaviour contract contains the seed's rubric
//! keywords (which are derived from the role + its actions). A real,
//! richly-specified teammate's contract names its actions and domain, so
//! the keywords appear; an empty stub's contract does not. That is exactly
//! the "gameable by keyword-parroting, but a true floor against empty
//! workers" property the design doc calls for.

use crate::eval_seed::{self, EvalSeed};
use crate::genesis_judge::JudgeInput;
use crate::pipeline::GenesisPlan;
use crate::prompt_writer::build_system_prompt;
use std::path::Path;

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
    /// The semantic safety-judge verdict token (`allow` /
    /// `allow_with_disclaimer` / `block` / `unavailable`).
    pub safety_verdict: String,
    /// Human-readable reasons for a non-pass (empty on pass). One per line
    /// is the gateway's join convention.
    pub reasons: Vec<String>,
}

impl GateVerdict {
    /// Compose the final verdict from the three independent signals: the
    /// deterministic eval floor, the role-contract comprehension gate, and
    /// the semantic safety judge.
    ///
    /// Precedence (fail-closed): a hard `Block` from any check that *ran*
    /// (eval floor below the bar, a comprehension governance miss, or a
    /// safety-judge `Block`) → [`GateState::Blocked`]; else a
    /// degraded/unavailable safety signal (a check that could not run) →
    /// [`GateState::NeedsReview`]; else [`GateState::Pass`]. Comprehension is
    /// an all-must-pass invariant gate, so any miss is a hard block — the
    /// teammate's own contract failed to demonstrate it understands a scope
    /// or governance boundary.
    #[must_use]
    pub fn compose(
        eval: EvalFloorOutcome,
        comprehension: crate::comprehension::ComprehensionOutcome,
        safety: SafetySignal,
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
            safety_verdict,
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

fn to_input(seed: &EvalSeed) -> DeterministicEvalInput {
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

/// Project a [`GenesisPlan`] onto the safety judge's [`JudgeInput`]. Mirrors
/// the projection `plan_or_broad` already uses so both paths feed the judge
/// an identical view of the archetype plan.
#[must_use]
pub fn judge_input_for_plan(plan: &GenesisPlan) -> JudgeInput<'_> {
    JudgeInput {
        charter: &plan.charter,
        persona_description: &plan.persona,
        action_vocabulary: plan
            .manifest
            .actions
            .as_ref()
            .map(|a| a.actions.keys().cloned().collect())
            .unwrap_or_default(),
    }
}

/// Owned hire-gate inputs reconstructed from a written vertical directory.
///
/// The flagship SSE path builds gate inputs straight off the in-memory
/// [`GenesisPlan`]; the hire-commit fence runs LATER, against a teammate
/// whose vertical is already on disk and whose original [`GenesisPlan`] is
/// gone. This reconstructs the SAME deterministic inputs + safety-judge
/// projection from the on-disk `eval-seed.toml`, `prompts/system.md`, and
/// `vertical.toml` so the hire fence re-runs the identical gate (fail-closed
/// parity — no per-surface divergence).
#[derive(Debug, Clone)]
pub struct DiskGateInputs {
    /// The deterministic seed tasks parsed from `eval-seed.toml`.
    pub seeds: Vec<DeterministicEvalInput>,
    /// The behaviour contract (lowercased) the deterministic subject answers
    /// with — the on-disk system prompt plus the action vocabulary.
    pub contract_text: String,
    /// Charter text for the safety-judge projection (operator's words).
    pub charter: String,
    /// Persona handle for the safety-judge projection.
    pub persona: String,
    /// Action vocabulary for the safety-judge projection.
    pub action_vocabulary: Vec<String>,
}

impl DiskGateInputs {
    /// Borrowed safety-judge projection — same shape both gate paths feed
    /// the judge.
    #[must_use]
    pub fn judge_input(&self) -> JudgeInput<'_> {
        JudgeInput {
            charter: &self.charter,
            persona_description: &self.persona,
            action_vocabulary: self.action_vocabulary.clone(),
        }
    }
}

#[derive(serde::Deserialize)]
struct SeedDoc {
    #[serde(default)]
    seed: Vec<EvalSeed>,
}

/// Reconstruct hire-gate inputs from a written Genesis vertical directory.
///
/// Reads `eval-seed.toml` (the 10 starter seeds + rubric keywords),
/// `prompts/system.md` (the behaviour contract), and `vertical.toml` (name,
/// description, persona, action vocabulary). Fails closed: a missing /
/// unparseable `eval-seed.toml` yields an empty seed set, which the eval
/// floor treats as a `Blocked` verdict.
///
/// # Errors
///
/// Returns [`crate::GenesisError`] only when `vertical.toml` is missing or
/// unparseable — a teammate whose manifest cannot be read must not be
/// activated.
pub fn gate_inputs_from_dir(vertical_dir: &Path) -> Result<DiskGateInputs, crate::GenesisError> {
    let manifest_body = std::fs::read_to_string(vertical_dir.join("vertical.toml"))
        .map_err(|e| crate::GenesisError::Io {
            what: "hire_gate.manifest.read",
            source: e,
        })?;
    let manifest: argo_vertical::VerticalManifest =
        toml::from_str(&manifest_body).map_err(|e| crate::GenesisError::Validation {
            stage: "hire_gate.manifest.parse",
            detail: e.to_string(),
        })?;

    let seeds = parse_seed_file(vertical_dir);
    let contract_text = disk_contract_text(vertical_dir, &manifest);
    let action_vocabulary = manifest
        .actions
        .as_ref()
        .map(|a| a.actions.keys().cloned().collect())
        .unwrap_or_default();

    Ok(DiskGateInputs {
        seeds,
        contract_text,
        // The verbatim charter isn't persisted as its own field; the system
        // prompt embeds it under "## Charter (operator's words)". Use the
        // vertical description as the charter projection for the judge — the
        // judge also sees the full action vocabulary, which is what the
        // restricted-surface question keys off.
        charter: manifest.vertical.description.clone(),
        // The persona handle isn't a standalone manifest field (role.persona
        // is a doc PATH); the human-readable vertical name is the closest
        // persona projection for the judge's impersonation question.
        persona: manifest.vertical.name.clone(),
        action_vocabulary,
    })
}

fn parse_seed_file(vertical_dir: &Path) -> Vec<DeterministicEvalInput> {
    let Ok(body) = std::fs::read_to_string(vertical_dir.join("eval-seed.toml")) else {
        return Vec::new();
    };
    let Ok(doc) = toml::from_str::<SeedDoc>(&body) else {
        return Vec::new();
    };
    doc.seed.iter().map(to_input).collect()
}

fn disk_contract_text(vertical_dir: &Path, manifest: &argo_vertical::VerticalManifest) -> String {
    let mut out = std::fs::read_to_string(vertical_dir.join("prompts").join("system.md"))
        .unwrap_or_default();
    out.push('\n');
    out.push_str(&manifest.vertical.name);
    out.push('\n');
    out.push_str(&manifest.vertical.description);
    if let Some(actions) = manifest.actions.as_ref() {
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
