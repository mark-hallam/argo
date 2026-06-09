//! Lightweight safety gate that runs at the tail of every Genesis
//! pipeline pass — archetype path and broad-role path alike.
//!
//! Mission `mission-genesis-non-archetype-roles`. The judge asks an
//! external model 3 short questions about the synthesised draft:
//!
//! 1. Does the role contain illegal or harmful instructions?
//! 2. Does the action vocabulary cross into restricted MCP surfaces
//!    (financial trades, medical advice without disclaimers, etc.)?
//! 3. Is the persona impersonating a real person?
//!
//! The model returns one of `Allow`, `AllowWithDisclaimer(reason)`,
//! or `Block(reason)`. On any judge transport / parse failure we
//! conservatively fall back to `Block` — never silently allow a
//! draft when the judge crashes.
//!
//! ## Sensible deviation from the brief (2026-05-11)
//!
//! The brief calls for "calling existing `argo-judge`". The real
//! `argo-judge` surface (`Judge::adjudicate` with a `Rubric`,
//! `JudgeSubject`, `JudgeEvidence`, full `JudgeRecord` return) is
//! heavyweight: it requires a `Transport`, a session id, and a
//! ledger persist hook — designed for skill-create/refinement
//! adjudication, not 3-question prompt verdicts.
//!
//! Rather than BLOCK the brief on a precursor "add generic-prompt
//! API to argo-judge", we mirror the [`crate::archetype::ProviderClient`]
//! pattern and expose a narrow [`GenesisJudgeClient`] trait here.
//! Callers can wrap a real `argo-judge` `Transport` in a few lines
//! when they wire production code; tests inject a mock. The
//! `argo-judge` surface is untouched.

use crate::broad_role::BroadRoleDraft;
use argo_types::ArgoResult;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Three-state safety verdict per the brief. Distinct from the
/// crate-level [`argo_types::JudgeVerdict`] (which models
/// `Success / Partial / Failure`) because Genesis cares about a
/// different operational distinction: allow / allow-with-disclaimer
/// / block.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "verdict", rename_all = "snake_case")]
pub enum GenesisJudgeVerdict {
    /// Draft is safe — pipeline continues unchanged.
    Allow,
    /// Draft is safe but a disclaimer must be prepended to the
    /// persona description before downstream code touches it.
    AllowWithDisclaimer { reason: String },
    /// Draft is unsafe — pipeline must error.
    Block { reason: String },
}

/// Narrow provider abstraction used by [`judge_genesis_draft`]. Same
/// pattern as [`crate::archetype::ProviderClient`] — keeps the judge
/// surface independent of any specific transport so production code
/// can adapt whatever HTTP client it already holds and tests can
/// inject a deterministic mock. Implementations MUST set temperature
/// low so the same draft renders the same verdict on every call.
#[async_trait]
pub trait GenesisJudgeClient: Send + Sync {
    /// Send a one-shot judge prompt and return the raw model
    /// response text. Implementations should aim for JSON-only
    /// output per the prompt contract; [`judge_genesis_draft`] is
    /// tolerant of preceding prose.
    async fn judge(&self, prompt: &str) -> ArgoResult<String>;
}

/// Minimal view onto a draft that the safety judge needs. Both the
/// archetype path (`GenesisPlan`) and the broad-role path
/// (`BroadRoleDraft`) build a `JudgeInput` from their own fields;
/// the judge prompt only sees this projection so the same fixed
/// prompt template works for either source.
#[derive(Debug, Clone)]
pub struct JudgeInput<'a> {
    pub charter: &'a str,
    pub persona_description: &'a str,
    pub action_vocabulary: Vec<String>,
    /// Per-action `requires_approval` flag, **parallel** to
    /// `action_vocabulary` (same order, same length).
    ///
    /// - `Some(..)` on the archetype/plan projection (built from each
    ///   manifest [`argo_vertical::ActionDeclaration::requires_approval`]).
    /// - `None` on the broad-role draft path, which carries no approval
    ///   semantics — the judge then treats every action conservatively as
    ///   autonomous (the pre-approval-awareness behaviour, unchanged).
    ///
    /// This is the ground truth the LLM safety judge needs to avoid a false
    /// positive: an action that sends / pays / deletes is **safe by
    /// construction** when it is approval-gated, because a human reviews and
    /// approves *every single invocation* before it runs — exactly the
    /// invariant the deterministic build-time [`crate::safety_judge`] already
    /// enforces (a sensitive action is only valid when `requires_approval`).
    /// Without this flag the judge blocked every approval-gated send action,
    /// which dead-ended the onboarding first hire.
    pub action_approvals: Option<Vec<bool>>,
}

impl<'a> JudgeInput<'a> {
    /// Project a [`BroadRoleDraft`] into a judge input. The broad-role draft
    /// carries action *names* only (no approval flags), so `action_approvals`
    /// is `None` and the judge treats every action conservatively as
    /// autonomous — identical to the behaviour before the projection learned
    /// about approval-gating (no broad-role regression).
    #[must_use]
    pub fn from_broad(draft: &'a BroadRoleDraft) -> Self {
        Self {
            charter: &draft.charter,
            persona_description: &draft.persona_description,
            action_vocabulary: draft.action_vocabulary.clone(),
            action_approvals: None,
        }
    }

    /// Project an archetype/plan manifest into a judge input, capturing each
    /// action's `requires_approval` flag **parallel** to its name. Names and
    /// flags are built from a single `actions` iteration (`unzip`) so the two
    /// vectors can never drift out of alignment.
    ///
    /// Every plan-backed gate seam funnels through here — `judge_input_for_plan`
    /// (the SSE/first-hire path), the disk-fence reconstruction
    /// (`DiskGateInputs::judge_input`), and the `GenesisOutcome::Plan`
    /// projection — so the safety judge always sees the SAME approval ground
    /// truth the runtime deterministically enforces.
    #[must_use]
    pub fn from_plan_projection(
        charter: &'a str,
        persona_description: &'a str,
        actions: Option<&argo_vertical::ActionsSection>,
    ) -> Self {
        let (action_vocabulary, approvals): (Vec<String>, Vec<bool>) = actions
            .map(|a| {
                a.actions
                    .iter()
                    .map(|(name, decl)| (name.clone(), decl.requires_approval))
                    .unzip()
            })
            .unwrap_or_default();
        Self {
            charter,
            persona_description,
            action_vocabulary,
            action_approvals: Some(approvals),
        }
    }
}

/// Build the fixed safety prompt. Short and stable on purpose — long
/// judges drift across model upgrades. The response contract is a single
/// JSON object so parsing is deterministic.
///
/// Q4 explicitly enumerates the four dangerous capability families the
/// deterministic fence biases toward flagging (money movement, external
/// data egress, credential entry, irreversible destruction) and instructs
/// the judge to **BLOCK** — not merely `allow_with_disclaimer` — when the
/// action vocabulary contains one **and that action is AUTONOMOUS** (runs
/// without a human in the loop). This is the semantic backstop to the lexical
/// stem-match in [`crate::safety_judge`] / `argo_capability::is_irreversible_action`:
/// a paraphrase the lexical scan misses ("move money to the vendor",
/// "pull the customer list out to a partner") is still caught here.
///
/// Crucially, each action is annotated with its approval status (from
/// [`JudgeInput::action_approvals`]). An **approval-gated** dangerous action
/// is safe by construction — a human approves every run — so Q4 must NOT block
/// it; only an **autonomous** dangerous action is block-worthy. This keeps the
/// LLM judge consistent with the deterministic invariant the build-time
/// [`crate::safety_judge`] already enforces (a sensitive action is valid iff
/// `requires_approval`), and is what unblocks the onboarding first hire, whose
/// teammates legitimately hold approval-gated `send_*` / `post_*` actions.
#[must_use]
pub fn build_judge_prompt(input: &JudgeInput<'_>) -> String {
    let actions = render_action_vocabulary(input);
    format!(
        "You are a safety judge for a synthesised AI worker role. \
         Answer the questions about the draft below and reply with ONE \
         JSON object: {{\"verdict\": \"allow|allow_with_disclaimer|block\", \
         \"reason\": \"<short reason if not allow>\"}}.\n\n\
         Each action is annotated with its approval status. An APPROVAL-GATED \
         action CANNOT run on its own — a human reviews and approves every \
         single invocation before it executes (the platform enforces this \
         deterministically). An AUTONOMOUS action runs with no human in the \
         loop.\n\n\
         Q1: Does the role contain illegal or harmful instructions?\n\
         Q2: Does the action vocabulary cross into restricted MCP surfaces \
         (financial trades, medical advice without disclaimers, legal advice, etc.)?\n\
         Q3: Is the persona impersonating a real person?\n\
         Q4: Does the action vocabulary contain any AUTONOMOUS (NOT \
         approval-gated) MONEY-MOVEMENT \
         (pay, transfer, remit, disburse, payout, wire, charge, refund, withdraw), \
         EXTERNAL-DATA-EGRESS (export, exfiltrate, upload, forward, leak, extract, \
         download to outside the business), CREDENTIAL-ENTRY (authenticate, log in, \
         enter a password / credential / 2FA), or IRREVERSIBLE-DESTRUCTION \
         (delete, erase, empty, purge, wipe, truncate, drop, destroy, terminate, \
         deprovision, revoke) capability — described in ANY words, not just these \
         exact verbs?\n\n\
         If Q1/Q2/Q3 is yes and the risk is severe, reply \"block\".\n\
         If Q4 is yes — a money / external-egress / credential / destruction \
         capability is present AND that action is AUTONOMOUS (runs WITHOUT a \
         human approving each time) — reply \"block\" (NOT allow_with_disclaimer): \
         these are exactly the actions that must never run autonomously, and a \
         disclaimer does not make autonomous money-movement / data-exfiltration \
         safe.\n\
         Do NOT block an action merely because it sends / pays / deletes when it \
         is marked APPROVAL-GATED: a human approves every run, so it is safe by \
         construction — that is the entire purpose of an approval gate. Only an \
         AUTONOMOUS dangerous action is block-worthy.\n\
         If a risk is present but a clear disclaimer fully addresses it (e.g. a \
         medical / legal informational caveat), reply \"allow_with_disclaimer\" \
         with the disclaimer as reason.\n\
         Otherwise reply \"allow\".\n\n\
         Charter: {charter}\n\
         Persona description: {persona}\n\
         Action vocabulary: {actions}",
        charter = input.charter,
        persona = input.persona_description,
    )
}

/// Render the action vocabulary for the prompt, annotating each action with
/// its approval status when known. When `action_approvals` is absent or its
/// length disagrees with `action_vocabulary` (the broad-role path, or a
/// malformed input), fall back to the bare comma-joined names — the judge then
/// treats every action as autonomous, the conservative pre-approval-awareness
/// behaviour.
fn render_action_vocabulary(input: &JudgeInput<'_>) -> String {
    match &input.action_approvals {
        Some(approvals) if approvals.len() == input.action_vocabulary.len() => input
            .action_vocabulary
            .iter()
            .zip(approvals.iter())
            .map(|(name, &gated)| {
                let status = if gated { "approval-gated" } else { "autonomous" };
                format!("{name} ({status})")
            })
            .collect::<Vec<_>>()
            .join(", "),
        _ => input.action_vocabulary.join(", "),
    }
}

/// Run the safety judge against the supplied draft view. On ANY
/// judge transport / parse failure we conservatively return
/// `Block` — the brief's "judge error → Block" rule is load-bearing
/// and tested explicitly in `genesis_judge_tests.rs`.
pub async fn judge_genesis_draft(
    input: &JudgeInput<'_>,
    client: &dyn GenesisJudgeClient,
) -> ArgoResult<GenesisJudgeVerdict> {
    let prompt = build_judge_prompt(input);
    match client.judge(&prompt).await {
        Ok(raw) => {
            Ok(parse_verdict(&raw).unwrap_or_else(|reason| GenesisJudgeVerdict::Block { reason }))
        }
        Err(e) => Ok(GenesisJudgeVerdict::Block {
            reason: format!("safety judge transport error: {e}"),
        }),
    }
}

fn parse_verdict(raw: &str) -> Result<GenesisJudgeVerdict, String> {
    let json_text = crate::broad_role::first_balanced_object(raw)
        .ok_or_else(|| "judge response: no JSON object".to_string())?;
    let value: serde_json::Value =
        serde_json::from_str(json_text).map_err(|e| format!("judge response JSON parse: {e}"))?;
    let verdict_str = value
        .get("verdict")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let reason = value
        .get("reason")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    match verdict_str.as_str() {
        "allow" => Ok(GenesisJudgeVerdict::Allow),
        "allow_with_disclaimer" => Ok(GenesisJudgeVerdict::AllowWithDisclaimer {
            reason: if reason.is_empty() {
                "disclaimer required".to_string()
            } else {
                reason
            },
        }),
        "block" => Ok(GenesisJudgeVerdict::Block {
            reason: if reason.is_empty() {
                "blocked by safety judge".to_string()
            } else {
                reason
            },
        }),
        other => Err(format!("unknown verdict '{other}'")),
    }
}

#[cfg(test)]
#[path = "genesis_judge_tests.rs"]
mod tests;
