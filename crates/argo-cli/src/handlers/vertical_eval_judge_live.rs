//! Live LLM judge wiring for scored eval (#701). Sibling of
//! `vertical_eval_score.rs` so that file stays under the
//! 500-LOC budget. The seam from #698 is wired here for
//! Anthropic; other provider families return a clear "not yet
//! supported" error rather than silently failing.
//!
//! ## Design
//!
//! Two layers — pure + transport:
//!
//! 1. The pure layer (in `argo_vertical::eval_score_judge`)
//!    builds the prompt, parses the response, and shapes
//!    `JudgeVerdict`. Unit-testable without any provider call.
//! 2. This file owns the transport layer: load the provider
//!    manifest, read the API key, build the right transport,
//!    dispatch the prompt. Tests inject a `JudgeBackend`
//!    (trait-based) so the dispatch path runs end-to-end with
//!    a mock LLM and no network access.
//!
//! ## Why a trait, not just a function
//!
//! The trait isolates tests from `argo_provider::Transport`.
//! Mocking the full `Transport` trait here would require
//! shipping a stub for every method (streaming, capability
//! report, etc.). The `JudgeBackend` trait is a one-method
//! abstraction over "ask a model to grade one fixture" — the
//! exact surface this slice needs.

use argo_provider::{
    AnthropicTransport, ProviderFamily, ProviderManifest, ProviderRequest, Sampling, Transport,
};
use argo_types::{
    ArgoError, ArgoResult, ContentBlock, Message, MessageId, MessageRole, SessionId,
};
use argo_vertical::{
    build_judge_prompt, parse_judge_response, FixtureScore, FixtureStatus, JudgeVerdict,
};
use async_trait::async_trait;
use chrono::Utc;
use std::path::PathBuf;
use std::sync::Arc;

/// One-method abstraction over "ask a model to grade one
/// fixture". Tests inject a stub; production wires
/// [`AnthropicJudgeBackend`].
#[async_trait]
pub(super) trait JudgeBackend: Send + Sync {
    /// Source label that ends up in `JudgeVerdict.source`. Live
    /// backends use `live:<provider>/<model>`; mock backends
    /// pick something traceable like `mock:foo`.
    fn source(&self) -> String;

    /// Send the prompt to the underlying model and return the
    /// raw response text. Errors propagate so the dispatcher
    /// can surface them as `ArgoError::Provider`.
    async fn grade(&self, prompt: &str) -> ArgoResult<String>;
}

/// Build a live Anthropic-backed judge from a manifest path
/// + the env-resolved API key. Fails loudly when the env var
/// isn't set — operators get an actionable message instead of
/// a silent fallback.
pub(super) fn build_anthropic_backend(
    provider_id: &str,
    model: &str,
) -> ArgoResult<AnthropicJudgeBackend> {
    let cwd = std::env::current_dir()
        .map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))?;
    let path: PathBuf = cwd
        .join("manifests")
        .join("providers")
        .join(format!("{provider_id}.toml"));
    let body = std::fs::read_to_string(&path).map_err(|e| {
        ArgoError::Provider(format!(
            "live judge: provider manifest `{}` unreadable: {e} \
             (expected at `{}`)",
            provider_id,
            path.display()
        ))
    })?;
    let manifest = ProviderManifest::from_toml_str(&body)?;
    if !matches!(manifest.provider.family, ProviderFamily::Anthropic) {
        return Err(ArgoError::Provider(format!(
            "live judge: provider family `{:?}` not yet wired; Anthropic-family is the only supported judge backend in #701. \
             Ship a follow-up to extend BedrockTransport / OpenAi* if you need them.",
            manifest.provider.family
        )));
    }
    let env_var = manifest.auth.api_key_env.clone();
    let api_key = std::env::var(&env_var).map_err(|_| {
        ArgoError::Provider(format!(
            "live judge: provider `{provider_id}` requires env `{env_var}` (resolved from \
             manifest `auth.api_key_env`). Set it and re-run, or use --judge heuristic for an \
             offline run."
        ))
    })?;
    // Detect Anthropic extended-thinking mode BEFORE the
    // manifest is consumed by the transport. Anthropic rejects
    // requests with `temperature != 1` when `thinking` is
    // enabled (HTTP 400: "temperature may only be set to 1
    // when thinking is enabled"). The shipped
    // `manifests/providers/anthropic.toml` carries
    // `[quirks].send-thinking-blocks = true`, so this MUST
    // honour the constraint or every live judge call 400s.
    let thinking_enabled = manifest_enables_thinking(&manifest);
    let transport = AnthropicTransport::from_manifest(manifest, &api_key)?;
    Ok(AnthropicJudgeBackend {
        transport: Arc::new(transport),
        model: model.to_string(),
        provider_id: provider_id.to_string(),
        thinking_enabled,
    })
}

/// Read the manifest's `[quirks].send-thinking-blocks` flag.
/// `true` means the transport will inject the `thinking` block
/// into every request — and Anthropic requires `temperature=1`
/// in that case. Pure inspector — no IO; lets the temperature-
/// selection helper stay testable.
pub(super) fn manifest_enables_thinking(manifest: &ProviderManifest) -> bool {
    matches!(
        manifest.quirks.get("send-thinking-blocks"),
        Some(toml::Value::Boolean(true))
    )
}

/// Pick the temperature for an Anthropic live judge request.
/// Pure: no IO, no provider state, just policy.
///
/// - `thinking_enabled = true` → `1.0` (Anthropic constraint;
///   any other value triggers HTTP 400 "temperature may only be
///   set to 1 when thinking is enabled").
/// - `thinking_enabled = false` → `0.0` (deterministic verdicts
///   for the same fixture across runs).
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
/// `argo_provider::anthropic::wire::apply_quirks`). We can't
/// override it from the manifest today; this constant mirrors
/// the transport's value so the live judge can pick a
/// `max_tokens` safely above it. If the transport ever exposes
/// a manifest field for the budget, replace this constant with
/// a getter that reads the parsed manifest.
pub(super) const ANTHROPIC_THINKING_BUDGET_TOKENS: u32 = 1024;

/// Pick `max_tokens` for an Anthropic live judge request.
/// Pure: no IO, no provider state.
///
/// Anthropic returns HTTP 400 when
/// `max_tokens <= thinking.budget_tokens`. The transport
/// hardcodes `budget_tokens = 1024` whenever thinking is
/// enabled, so we MUST request well above that to leave room
/// for the actual JSON verdict the judge emits.
///
/// - `thinking_enabled = true` → `2048` (1024 budget + 1024
///   headroom for the verdict JSON; the verdict is small but
///   the buffer matters when models occasionally pad
///   reasoning before the fence).
/// - `thinking_enabled = false` → `512` (compact JSON only;
///   no reasoning channel to fund).
///
/// Invariant: when `thinking_enabled`, the returned value is
/// strictly greater than [`ANTHROPIC_THINKING_BUDGET_TOKENS`].
/// The regression test asserts this so a future edit can't
/// silently re-introduce the HTTP 400.
#[must_use]
pub(super) fn judge_max_tokens_for(thinking_enabled: bool) -> u32 {
    if thinking_enabled {
        // Computed (not magic number): budget + at least 1024
        // headroom for the verdict JSON. Using the constant
        // means a future budget change in the transport
        // automatically widens our cap.
        ANTHROPIC_THINKING_BUDGET_TOKENS + 1024
    } else {
        512
    }
}

/// Production Anthropic backend. Wraps an `AnthropicTransport`
/// + the model id + the cached thinking-mode flag. `source()`
/// returns `live:<provider>/<model>` so the verdict's audit
/// trail names the exact model.
pub(super) struct AnthropicJudgeBackend {
    transport: Arc<AnthropicTransport>,
    model: String,
    provider_id: String,
    /// Cached at construction time from the manifest's
    /// `[quirks].send-thinking-blocks` flag (#701 hotfix).
    /// Drives the temperature pick — without honouring this,
    /// every request 400s when the manifest enables thinking.
    thinking_enabled: bool,
}

#[async_trait]
impl JudgeBackend for AnthropicJudgeBackend {
    fn source(&self) -> String {
        format!("live:{}/{}", self.provider_id, self.model)
    }

    async fn grade(&self, prompt: &str) -> ArgoResult<String> {
        let session_id = SessionId(uuid::Uuid::nil());
        let messages = vec![Message {
            id: MessageId(uuid::Uuid::now_v7()),
            session_id,
            role: MessageRole::User,
            content: vec![ContentBlock::Text {
                text: prompt.to_string(),
            }],
            tool_calls: Vec::new(),
            tool_call_id: None,
            reasoning: None,
            created_at: Utc::now(),
            provider_meta: serde_json::Value::Null,
        }];
        let req = ProviderRequest {
            model: self.model.clone(),
            messages,
            tools: Vec::new(),
            // Tight system prompt → judge verdicts are stable
            // for the same input. Temperature is policy-driven
            // (see `judge_temperature_for`): 0.0 when free to
            // choose; 1.0 when thinking-mode forces it.
            system: Some(
                "You are a strict, deterministic eval grader. Reply with one fenced ```json block matching the requested shape. No prose around the block.".into(),
            ),
            sampling: Sampling {
                temperature: Some(judge_temperature_for(self.thinking_enabled)),
                // #701c: max_tokens MUST exceed
                // thinking.budget_tokens or Anthropic returns
                // HTTP 400 ("max_tokens must be greater than
                // thinking.budget_tokens"). The helper picks
                // 2048 when thinking is on, 512 when off.
                max_tokens: Some(judge_max_tokens_for(self.thinking_enabled)),
                ..Sampling::default()
            },
        };
        let resp = self.transport.complete(&req).await?;
        Ok(resp.text.unwrap_or_default())
    }
}

/// Grade one fixture via the supplied backend. Builds the
/// prompt, dispatches, parses. Returns an `uncertain` verdict
/// (with score 0.5) when the LLM call itself errors so the
/// suite surfaces the failure without crashing the run.
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

/// Re-grade every Scored fixture in `score_fixtures` using
/// `backend`. Replaces the placeholder `not_wired` verdicts
/// from `eval_score_judge::judge_fixture` with live verdicts.
/// Skips no_actual / malformed fixtures (no point judging
/// rows the rule engine couldn't score).
///
/// The fixture row's `input` and `expected` blocks must be
/// supplied alongside the rule-engine outcome — the rule
/// engine collapses them, so this layer needs the suite body
/// re-parsed. Caller passes a `(input, expected, actual)`
/// triple per fixture, indexed by id.
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

#[cfg(test)]
#[path = "vertical_eval_judge_live_tests.rs"]
mod tests;
