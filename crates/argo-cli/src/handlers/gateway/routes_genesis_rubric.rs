//! Provider-touching half of the LLM-graded capability rubric (Genesis P7,
//! DEC-025 #7).
//!
//! The pure half ([`argo_genesis::rubric`]) builds the prompt, parses the
//! verdict, and composes the [`argo_genesis::RubricOutcome`]. THIS half dials
//! the model over the SAME shared provider transport the archetype classifier +
//! the safety judge already use (`complete_once` → `argo_provider::Transport`),
//! flag-gated by [`GENESIS_LLM_RUBRIC_ENV`] (mirroring the classifier's
//! `ARGO_GENESIS_LLM_CLASSIFIER` opt-in discipline), and degrades fail-safe.
//!
//! ## Degrade fail-safe — never a silent pass (the load-bearing invariant)
//!
//! - **Flag off** (the test/default posture) → [`argo_genesis::RubricOutcome::not_applicable`]:
//!   the rubric is a vacuous pass; the deterministic eval floor + the other gate
//!   axes still gate. NO provider dial-out, so test runs stay deterministic and
//!   byte-identical exactly like the classifier path.
//! - **Flag on, provider/transport failed** → [`argo_genesis::RubricOutcome::unavailable`]
//!   (degraded → the composer holds the teammate in `needs_review`, audited,
//!   held in dry-run). This is the same degraded posture the safety judge takes
//!   (`GateJudgeClient`): a model that *can't run* must neither brick hiring nor
//!   wave a teammate through.
//! - **Flag on, provider returned a usable grade** → the real per-dimension
//!   verdict (a thin/incoherent teammate is hard-blocked).
//!
//! Because the gateway already runs the safety judge over `complete_once`, and
//! the rubric reuses the SAME transport + the SAME [`resolve_provider_id`]
//! resolution, P7 adds NO new provider edge — it is one more narrow client over
//! the existing dispatcher.

use super::routes_genesis_stream_llm::{complete_once, resolve_provider_id};
use argo_genesis::{grade_rubric, JudgeInput, RubricClient, RubricOutcome, RubricVerdict};
use argo_types::ArgoResult;
use async_trait::async_trait;
use std::sync::atomic::{AtomicBool, Ordering};

/// Explicit opt-in for the LLM-graded capability rubric (Genesis P7). Defaults
/// **off** so test runs stay on the deterministic floor only (no subprocess
/// dial-out, no flakiness) — mirroring the classifier's
/// `ARGO_GENESIS_LLM_CLASSIFIER` discipline. The live gateway turns it on by
/// exporting `ARGO_GENESIS_LLM_RUBRIC=1` in `~/.argo/env.local`.
pub(super) const GENESIS_LLM_RUBRIC_ENV: &str = "ARGO_GENESIS_LLM_RUBRIC";

/// `true` when [`GENESIS_LLM_RUBRIC_ENV`] is set to a truthy value (`1` /
/// `true` / `on` / `yes`, case-insensitive). Absent / empty / any other value
/// reads as off — the default-off posture that keeps the rubric a vacuous pass
/// (the deterministic floor stays the gate) until the operator opts in.
pub(super) fn rubric_enabled() -> bool {
    matches!(
        std::env::var(GENESIS_LLM_RUBRIC_ENV)
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase()
            .as_str(),
        "1" | "true" | "on" | "yes"
    )
}

/// Run the LLM rubric axis for a teammate, returning the composed
/// [`RubricOutcome`] for [`argo_genesis::GateVerdict::with_rubric`].
///
/// - Flag off → [`RubricOutcome::not_applicable`] (vacuous pass; the
///   deterministic floor + other axes gate). No provider call.
/// - Flag on, grader transport failed → [`RubricOutcome::unavailable`]
///   (degraded → `needs_review`, NEVER a silent pass).
/// - Flag on, grader produced a grade → the real per-dimension outcome.
pub(super) async fn run_rubric_axis(
    input: JudgeInput<'_>,
    behaviour_contract: &str,
) -> RubricOutcome {
    if !rubric_enabled() {
        return RubricOutcome::not_applicable();
    }
    let client = GateRubricClient::new(resolve_provider_id());
    let verdict = grade_rubric(&input, behaviour_contract, &client).await;
    // `grade_rubric` already maps a transport error to `Unavailable`, but we
    // double-check the client's own transport-failure flag so a model that
    // returned an unparseable response on a WORKING transport stays
    // `Unavailable` (degraded) too — both are "could not produce a usable
    // grade", and neither is ever a silent pass. The verdict the pure half
    // produced is authoritative; the flag only documents the failure mode.
    if client.transport_failed() {
        if let RubricVerdict::Graded { .. } = verdict {
            // Defensive: a graded verdict on a failed transport is impossible
            // (a failed call returns Err → Unavailable), but if it ever
            // happened, degrade rather than trust a grade from a broken dial.
            return RubricOutcome::unavailable("rubric grader transport failed mid-grade");
        }
    }
    RubricOutcome::from_verdict(verdict)
}

/// [`RubricClient`] over the shared provider transport — the SAME machinery as
/// the archetype classifier + the safety judge's `GateJudgeClient`. Records
/// whether the underlying provider round-trip errored so the caller can keep a
/// transport failure firmly in the degraded (`needs_review`) lane.
struct GateRubricClient {
    provider_id: String,
    transport_failed: AtomicBool,
}

impl GateRubricClient {
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
impl RubricClient for GateRubricClient {
    async fn grade(&self, prompt: &str) -> ArgoResult<String> {
        // Hire-gate rubric grader: operator-authed + bounded (one call per hire,
        // default-OFF flag), NOT a pre-auth stranger surface — gated by
        // construction but uncapped (`None`). The genesis GENERATION path
        // carries the real anonymous cap.
        // 1024 max_tokens (not the classifier's 64): claude-sonnet-4-6 emits prose
        // before the JSON rubric verdict and the HTTP transport respects the cap, so
        // 64 truncated it. 1024 = ample for prose + JSON, at/under the thinking budget
        // so the deterministic (temperature 0) grader keeps thinking OFF.
        match complete_once(&self.provider_id, prompt, None, 1024).await {
            Ok(text) => Ok(text),
            Err(e) => {
                self.transport_failed.store(true, Ordering::Relaxed);
                Err(e)
            }
        }
    }
}

#[cfg(test)]
#[path = "routes_genesis_rubric_tests.rs"]
mod tests;
