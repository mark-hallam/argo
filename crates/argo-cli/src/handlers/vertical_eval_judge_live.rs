//! Live LLM judge wiring for scored eval (#701, multi-provider follow-up).
//!
//! Sibling of `vertical_eval_score.rs` (so that file stays under the
//! 500-LOC budget) and parent to `vertical_eval_judge_backends.rs`
//! (which holds the four family-specific [`JudgeBackend`] impls so
//! THIS file stays under the budget).
//!
//! ## Design
//!
//! Three layers — pure / dispatcher / transport:
//!
//! 1. The pure layer (in `argo_vertical::eval_score_judge`) builds the
//!    prompt, parses the response, and shapes `JudgeVerdict`.
//!    Unit-testable without any provider call.
//! 2. The dispatcher layer (this file) owns:
//!     - the [`JudgeBackend`] trait — a one-method abstraction over
//!       "ask a model to grade one fixture",
//!     - the [`build_judge_backend`] cross-family dispatcher, which
//!       picks the right backend based on the manifest's
//!       `[provider].family`,
//!     - the policy helpers ([`judge_temperature_for`],
//!       [`judge_max_tokens_for`], [`manifest_enables_thinking`])
//!       shared by the family backends.
//! 3. The transport layer (in `vertical_eval_judge_backends`) holds
//!    one `JudgeBackend` impl per family + their builders.
//!
//! ## Why a trait, not just a function
//!
//! The trait isolates tests from `argo_provider::Transport`. Mocking
//! the full `Transport` trait here would require shipping a stub for
//! every method (streaming, capability report, etc.). The
//! `JudgeBackend` trait is a one-method abstraction over "ask a model
//! to grade one fixture" — the exact surface this slice needs.

use argo_provider::{ProviderFamily, ProviderManifest};
use argo_types::ArgoResult;
use argo_vertical::{
    build_judge_prompt, parse_judge_response, FixtureScore, FixtureStatus, JudgeVerdict,
};
use async_trait::async_trait;

// Re-exports so existing call sites + tests see the family backends
// at this module's path. `build_anthropic_backend` is the legacy
// constructor used by `vertical_eval_score::live_judge_for_suite`
// before #701b's multi-provider dispatch landed; new code should
// prefer `build_judge_backend`.
pub(super) use super::vertical_eval_judge_backends::{
    build_anthropic_backend, build_bedrock_backend, build_openai_compat_backend,
    build_openai_responses_backend,
};

/// One-method abstraction over "ask a model to grade one fixture".
/// Tests inject a stub; production wires one of the four family
/// backends in `vertical_eval_judge_backends` via
/// [`build_judge_backend`].
#[async_trait]
pub(super) trait JudgeBackend: Send + Sync {
    /// Source label that ends up in `JudgeVerdict.source`. Live
    /// backends use `live:<provider>/<model>`; mock backends pick
    /// something traceable like `mock:foo`.
    fn source(&self) -> String;

    /// Send the prompt to the underlying model and return the raw
    /// response text. Errors propagate so the dispatcher can surface
    /// them as `ArgoError::Provider`.
    async fn grade(&self, prompt: &str) -> ArgoResult<String>;
}

/// Cross-family dispatcher. Loads the manifest, peeks at
/// `[provider].family`, and builds the matching backend. Async
/// because the Bedrock backend needs the AWS SDK's default
/// credential chain (`aws_config::load_defaults`).
///
/// New call sites should prefer this over the per-family builders;
/// the per-family functions remain pub(super) for tests + the
/// existing Anthropic-only call site in `vertical_eval_score`.
pub(super) async fn build_judge_backend(
    provider_id: &str,
    model: &str,
) -> ArgoResult<Box<dyn JudgeBackend>> {
    // Peek at the family before picking a builder. The family
    // builders re-load the manifest themselves so each one stays
    // a clean pub(super) function — this peek is cheap (one
    // file read + TOML parse) and keeps the dispatcher free of
    // construction details.
    let manifest = super::vertical_eval_judge_backends::load_manifest(provider_id)?;
    match manifest.provider.family {
        ProviderFamily::Anthropic => Ok(Box::new(build_anthropic_backend(provider_id, model)?)),
        ProviderFamily::OpenaiCompatible => Ok(Box::new(build_openai_compat_backend(
            provider_id,
            model,
        )?)),
        ProviderFamily::OpenaiResponses => Ok(Box::new(build_openai_responses_backend(
            provider_id,
            model,
        )?)),
        ProviderFamily::Bedrock => {
            Ok(Box::new(build_bedrock_backend(provider_id, model).await?))
        }
    }
}

/// Read the manifest's `[quirks].send-thinking-blocks` flag. `true`
/// means the transport will inject the `thinking` block into every
/// request — and Anthropic requires `temperature=1` in that case.
/// Pure inspector — no IO; lets the temperature-selection helper
/// stay testable.
pub(super) fn manifest_enables_thinking(manifest: &ProviderManifest) -> bool {
    matches!(
        manifest.quirks.get("send-thinking-blocks"),
        Some(toml::Value::Boolean(true))
    )
}

/// Pick the temperature for an Anthropic / Bedrock-Claude live judge
/// request. Pure: no IO, no provider state, just policy.
///
/// - `thinking_enabled = true` → `1.0` (Anthropic constraint; any
///   other value triggers HTTP 400 "temperature may only be set to 1
///   when thinking is enabled").
/// - `thinking_enabled = false` → `0.0` (deterministic verdicts for
///   the same fixture across runs).
#[must_use]
pub(super) fn judge_temperature_for(thinking_enabled: bool) -> f32 {
    if thinking_enabled {
        1.0
    } else {
        0.0
    }
}

/// Hardcoded extended-thinking budget the Anthropic transport
/// injects when `[quirks].send-thinking-blocks = true` (see
/// `argo_provider::anthropic::wire::apply_quirks`). We can't override
/// it from the manifest today; this constant mirrors the transport's
/// value so the live judge can pick a `max_tokens` safely above it.
/// If the transport ever exposes a manifest field for the budget,
/// replace this constant with a getter that reads the parsed
/// manifest.
pub(super) const ANTHROPIC_THINKING_BUDGET_TOKENS: u32 = 1024;

/// Pick `max_tokens` for an Anthropic / Bedrock-Claude live judge
/// request. Pure: no IO, no provider state.
///
/// Anthropic returns HTTP 400 when
/// `max_tokens <= thinking.budget_tokens`. The transport hardcodes
/// `budget_tokens = 1024` whenever thinking is enabled, so we MUST
/// request well above that to leave room for the actual JSON
/// verdict the judge emits.
///
/// - `thinking_enabled = true` → `2048` (1024 budget + 1024 headroom
///   for the verdict JSON; the verdict is small but the buffer
///   matters when models occasionally pad reasoning before the
///   fence).
/// - `thinking_enabled = false` → `512` (compact JSON only; no
///   reasoning channel to fund).
///
/// Invariant: when `thinking_enabled`, the returned value is
/// strictly greater than [`ANTHROPIC_THINKING_BUDGET_TOKENS`]. The
/// regression test asserts this so a future edit can't silently
/// re-introduce the HTTP 400.
#[must_use]
pub(super) fn judge_max_tokens_for(thinking_enabled: bool) -> u32 {
    if thinking_enabled {
        // Computed (not magic number): budget + at least 1024
        // headroom for the verdict JSON. Using the constant means
        // a future budget change in the transport automatically
        // widens our cap.
        ANTHROPIC_THINKING_BUDGET_TOKENS + 1024
    } else {
        512
    }
}

/// Grade one fixture via the supplied backend. Builds the prompt,
/// dispatches, parses. Returns an `uncertain` verdict (with score
/// 0.5) when the LLM call itself errors so the suite surfaces the
/// failure without crashing the run.
pub(super) async fn grade_fixture<B: JudgeBackend + ?Sized>(
    backend: &B,
    fixture_input: &str,
    expected: &serde_json::Value,
    actual: &str,
) -> JudgeVerdict {
    let prompt = build_judge_prompt(fixture_input, expected, actual);
    let source = backend.source();
    match backend.grade(&prompt).await {
        Ok(text) => parse_judge_response(&text, &source),
        Err(e) => JudgeVerdict {
            passed: false,
            reason: format!("uncertain: live judge call failed: {e}"),
            source,
            score: Some(0.5),
            failed_criteria: vec!["judge_call_failed".into()],
        },
    }
}

/// Re-grade every Scored fixture in `score_fixtures` using `backend`.
/// Replaces the placeholder `not_wired` verdicts from
/// `eval_score_judge::judge_fixture` with live verdicts. Skips
/// no_actual / malformed fixtures (no point judging rows the rule
/// engine couldn't score).
///
/// The fixture row's `input` and `expected` blocks must be supplied
/// alongside the rule-engine outcome — the rule engine collapses
/// them, so this layer needs the suite body re-parsed. Caller passes
/// a `(input, expected, actual)` triple per fixture, indexed by id.
pub(super) async fn judge_all_fixtures<B: JudgeBackend + ?Sized>(
    backend: &B,
    score_fixtures: &[FixtureScore],
    fixture_inputs: &[(String, String, serde_json::Value, String)], // (id, input, expected, actual)
) -> Vec<Option<JudgeVerdict>> {
    let mut out: Vec<Option<JudgeVerdict>> = Vec::with_capacity(score_fixtures.len());
    for fx in score_fixtures {
        if !matches!(fx.status, FixtureStatus::Scored) {
            out.push(None);
            continue;
        }
        let row = fixture_inputs.iter().find(|(id, _, _, _)| *id == fx.id);
        match row {
            Some((_, input, expected, actual)) => {
                let v = grade_fixture(backend, input, expected, actual).await;
                out.push(Some(v));
            }
            None => out.push(None),
        }
    }
    out
}

// Tests live in a sibling file to keep this one tight. They cover
// the trait + dispatcher + policy helpers via a `MockBackend`
// implementing `JudgeBackend`, so no API key is required.
#[cfg(test)]
#[path = "vertical_eval_judge_live_tests.rs"]
mod tests;
