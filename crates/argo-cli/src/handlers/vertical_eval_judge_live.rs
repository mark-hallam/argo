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
    let transport = AnthropicTransport::from_manifest(manifest, &api_key)?;
    Ok(AnthropicJudgeBackend {
        transport: Arc::new(transport),
        model: model.to_string(),
        provider_id: provider_id.to_string(),
    })
}

/// Production Anthropic backend. Wraps an `AnthropicTransport`
/// and a model id. `source()` returns `live:<provider>/<model>`
/// so the verdict's audit trail names the exact model.
pub(super) struct AnthropicJudgeBackend {
    transport: Arc<AnthropicTransport>,
    model: String,
    provider_id: String,
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
            // Tight system prompt + low temperature → judge
            // verdicts are stable run-to-run for the same input.
            system: Some(
                "You are a strict, deterministic eval grader. Reply with one fenced ```json block matching the requested shape. No prose around the block.".into(),
            ),
            sampling: Sampling {
                temperature: Some(0.0),
                max_tokens: Some(512),
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
