//! OpenAI-family [`JudgeBackend`] implementations and shared helpers.
//!
//! Sibling to `vertical_eval_judge_live` so that file stays under the
//! 500-LOC budget. Each backend wraps an [`argo_provider`] transport and
//! implements the one-method `JudgeBackend` trait. The
//! [`super::vertical_eval_judge_live::build_judge_backend`] dispatcher
//! picks the right one based on the manifest's `[provider].family`.
//!
//! Anthropic-family backends (Anthropic, Bedrock) live in the sibling
//! `vertical_eval_judge_backends_anthropic` module — both share a
//! thinking-mode quirk that makes them a cohesive unit. The two builders
//! are re-exported from this module so the dispatcher's import surface
//! stays a single path.
//!
//! ## Family quirks the live judge has to honour
//!
//! - **OpenAI Responses** — the o-series reasoning models (o1 / o3 /
//!   o4) reject `temperature != 1.0`. We detect via
//!   [`is_openai_o_series_model`] and omit the temperature override
//!   entirely (provider default is 1.0).
//! - **OpenAI Compatible** — plain Chat Completions; deterministic
//!   `temperature = 0.0` is fine.
//! - **Anthropic / Bedrock** — see
//!   [`super::vertical_eval_judge_backends_anthropic`].

use super::vertical_eval_judge_live::JudgeBackend;
use argo_provider::{
    OpenAiCompatTransport, OpenAiResponsesTransport, ProviderFamily, ProviderManifest,
    ProviderRequest, Sampling, Transport,
};
use argo_types::{ArgoError, ArgoResult, ContentBlock, Message, MessageId, MessageRole, SessionId};
use async_trait::async_trait;
use chrono::Utc;
use std::path::PathBuf;
use std::sync::Arc;

pub(super) use super::vertical_eval_judge_backends_anthropic::{
    build_anthropic_backend, build_bedrock_backend,
};

/// System prompt shared by every family backend. Tight wording → judge
/// verdicts are stable for the same input across families.
pub(super) const JUDGE_SYSTEM: &str = "You are a strict, deterministic eval grader. Reply with one fenced ```json block matching the requested shape. No prose around the block.";

/// Compact deterministic cap for non-thinking judge calls.
const JUDGE_MAX_TOKENS_DEFAULT: u32 = 512;

/// Heuristic match for OpenAI o-series reasoning models (o1 / o3 / o4),
/// which reject `temperature != 1.0`. Conservative: matches model
/// names starting with `o1`, `o3`, or `o4`. The `gpt-4o` /
/// `gpt-4o-mini` family is NOT in this set — those accept arbitrary
/// temperature. If OpenAI ships another constrained family, extend
/// the match here. Coverage in the unit-test block below.
#[must_use]
pub(super) fn is_openai_o_series_model(model: &str) -> bool {
    let m = model.trim();
    m.starts_with("o1") || m.starts_with("o3") || m.starts_with("o4")
}

/// Load `manifests/providers/<provider_id>.toml` relative to cwd.
/// Shared by every family builder so error messages stay consistent.
pub(super) fn load_manifest(provider_id: &str) -> ArgoResult<ProviderManifest> {
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Io(format!("current_dir: {e}")))?;
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
    ProviderManifest::from_toml_str(&body)
}

/// Read the API key from the manifest's `auth.api_key_env`. Used by
/// every family except Bedrock, which gets credentials from the AWS
/// SDK's default chain.
pub(super) fn read_api_key(manifest: &ProviderManifest, provider_id: &str) -> ArgoResult<String> {
    let env_var = manifest.auth.api_key_env.clone();
    argo_config::cred::get(&env_var).map_err(|_| {
        ArgoError::Provider(format!(
            "live judge: provider `{provider_id}` requires env `{env_var}` (resolved from \
             manifest `auth.api_key_env`). Set it and re-run, or use --judge heuristic for an \
             offline run."
        ))
    })
}

pub(super) fn one_user_message(prompt: &str) -> Vec<Message> {
    vec![Message {
        id: MessageId(uuid::Uuid::now_v7()),
        session_id: SessionId(uuid::Uuid::nil()),
        role: MessageRole::User,
        content: vec![ContentBlock::Text {
            text: prompt.to_string(),
        }],
        tool_calls: Vec::new(),
        tool_call_id: None,
        reasoning: None,
        created_at: Utc::now(),
        provider_meta: serde_json::Value::Null,
    }]
}

pub(super) fn require_family(
    manifest: &ProviderManifest,
    expected: ProviderFamily,
    builder: &str,
) -> ArgoResult<()> {
    if manifest.provider.family != expected {
        return Err(ArgoError::Provider(format!(
            "live judge: {builder} requires family={expected:?}; got {:?}",
            manifest.provider.family
        )));
    }
    Ok(())
}

// ===================================================================
// OpenAI Chat Completions (`openai-compatible`)
// ===================================================================

/// OpenAI Chat Completions backend (used for OpenAI proper, OpenRouter,
/// Together, vLLM, and any provider whose manifest declares
/// `family = "openai-compatible"`).
pub(super) struct OpenAiCompatJudgeBackend {
    transport: Arc<OpenAiCompatTransport>,
    model: String,
    provider_id: String,
}

impl std::fmt::Debug for OpenAiCompatJudgeBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OpenAiCompatJudgeBackend")
            .field("provider_id", &self.provider_id)
            .field("model", &self.model)
            .finish()
    }
}

#[async_trait]
impl JudgeBackend for OpenAiCompatJudgeBackend {
    fn source(&self) -> String {
        format!("live:{}/{}", self.provider_id, self.model)
    }

    async fn grade(&self, prompt: &str) -> ArgoResult<String> {
        let req = ProviderRequest {
            model: self.model.clone(),
            messages: one_user_message(prompt),
            tools: Vec::new(),
            system: Some(JUDGE_SYSTEM.into()),
            sampling: Sampling {
                temperature: Some(0.0),
                max_tokens: Some(JUDGE_MAX_TOKENS_DEFAULT),
                ..Sampling::default()
            },
        };
        let resp = self.transport.complete(&req).await?;
        Ok(resp.text.unwrap_or_default())
    }
}

/// Build an OpenAI-Compatible judge backend.
pub(super) fn build_openai_compat_backend(
    provider_id: &str,
    model: &str,
) -> ArgoResult<OpenAiCompatJudgeBackend> {
    let manifest = load_manifest(provider_id)?;
    require_family(
        &manifest,
        ProviderFamily::OpenaiCompatible,
        "build_openai_compat_backend",
    )?;
    let api_key = read_api_key(&manifest, provider_id)?;
    let transport = OpenAiCompatTransport::from_manifest(manifest, &api_key)?;
    Ok(OpenAiCompatJudgeBackend {
        transport: Arc::new(transport),
        model: model.to_string(),
        provider_id: provider_id.to_string(),
    })
}

// ===================================================================
// OpenAI Responses (`openai-responses`)
// ===================================================================

/// OpenAI Responses-API backend. Detects o-series reasoning models at
/// construction time and omits `temperature` from the request body
/// when the model rejects anything other than 1.0.
pub(super) struct OpenAiResponsesJudgeBackend {
    transport: Arc<OpenAiResponsesTransport>,
    model: String,
    provider_id: String,
    /// `true` when the model is an o-series reasoner (o1/o3/o4) and
    /// rejects custom temperatures. Cached at construction so the
    /// hot grade path stays branchless.
    is_o_series: bool,
}

impl std::fmt::Debug for OpenAiResponsesJudgeBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OpenAiResponsesJudgeBackend")
            .field("provider_id", &self.provider_id)
            .field("model", &self.model)
            .field("is_o_series", &self.is_o_series)
            .finish()
    }
}

#[async_trait]
impl JudgeBackend for OpenAiResponsesJudgeBackend {
    fn source(&self) -> String {
        format!("live:{}/{}", self.provider_id, self.model)
    }

    async fn grade(&self, prompt: &str) -> ArgoResult<String> {
        // o-series rejects temperature override entirely; sending
        // anything other than the provider default (1.0) returns
        // HTTP 400. We therefore omit it (None) for o-series and
        // pin to 0.0 deterministic for everything else.
        let temperature = if self.is_o_series { None } else { Some(0.0) };
        let req = ProviderRequest {
            model: self.model.clone(),
            messages: one_user_message(prompt),
            tools: Vec::new(),
            system: Some(JUDGE_SYSTEM.into()),
            sampling: Sampling {
                temperature,
                max_tokens: Some(JUDGE_MAX_TOKENS_DEFAULT),
                ..Sampling::default()
            },
        };
        let resp = self.transport.complete(&req).await?;
        Ok(resp.text.unwrap_or_default())
    }
}

/// Build an OpenAI Responses judge backend.
pub(super) fn build_openai_responses_backend(
    provider_id: &str,
    model: &str,
) -> ArgoResult<OpenAiResponsesJudgeBackend> {
    let manifest = load_manifest(provider_id)?;
    require_family(
        &manifest,
        ProviderFamily::OpenaiResponses,
        "build_openai_responses_backend",
    )?;
    let api_key = read_api_key(&manifest, provider_id)?;
    let is_o_series = is_openai_o_series_model(model);
    let transport = OpenAiResponsesTransport::from_manifest(manifest, &api_key)?;
    Ok(OpenAiResponsesJudgeBackend {
        transport: Arc::new(transport),
        model: model.to_string(),
        provider_id: provider_id.to_string(),
        is_o_series,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn o_series_detection_matches_constrained_models() {
        assert!(is_openai_o_series_model("o1"));
        assert!(is_openai_o_series_model("o1-mini"));
        assert!(is_openai_o_series_model("o1-preview"));
        assert!(is_openai_o_series_model("o3"));
        assert!(is_openai_o_series_model("o3-mini"));
        assert!(is_openai_o_series_model("o4-mini"));
        // Whitespace tolerance — accept lazy operator input.
        assert!(is_openai_o_series_model("  o1-mini  "));
    }

    #[test]
    fn o_series_detection_rejects_non_constrained_models() {
        // gpt-4o is NOT an o-series — it accepts arbitrary temperature.
        assert!(!is_openai_o_series_model("gpt-4o"));
        assert!(!is_openai_o_series_model("gpt-4o-mini"));
        // Plain GPT models — definitely not.
        assert!(!is_openai_o_series_model("gpt-4"));
        assert!(!is_openai_o_series_model("gpt-3.5-turbo"));
        // Empty / whitespace.
        assert!(!is_openai_o_series_model(""));
        assert!(!is_openai_o_series_model("   "));
        // Anthropic / Bedrock model ids — must not match.
        assert!(!is_openai_o_series_model("claude-3-5-sonnet"));
        assert!(!is_openai_o_series_model("anthropic.claude-3-haiku"));
    }

    #[test]
    fn build_openai_compat_backend_errors_clearly_when_manifest_missing() {
        let err = build_openai_compat_backend("definitely-not-a-real-provider-id-67890", "gpt-4o")
            .unwrap_err();
        let msg = format!("{err}");
        assert!(
            msg.contains("definitely-not-a-real-provider-id-67890"),
            "{msg}"
        );
        assert!(msg.contains("manifests/providers"), "{msg}");
    }

    #[test]
    fn build_openai_responses_backend_errors_clearly_when_manifest_missing() {
        let err = build_openai_responses_backend("definitely-not-a-real-provider-id-67890", "o1")
            .unwrap_err();
        let msg = format!("{err}");
        assert!(
            msg.contains("definitely-not-a-real-provider-id-67890"),
            "{msg}"
        );
        assert!(msg.contains("manifests/providers"), "{msg}");
    }
}
