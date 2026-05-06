//! Family-specific [`JudgeBackend`] implementations.
//!
//! Sibling to `vertical_eval_judge_live` so that file stays under the
//! 500-LOC budget. Each backend wraps an [`argo_provider`] transport and
//! implements the one-method `JudgeBackend` trait. The
//! [`super::vertical_eval_judge_live::build_judge_backend`] dispatcher
//! picks the right one based on the manifest's `[provider].family`.
//!
//! ## Family quirks the live judge has to honour
//!
//! - **Anthropic** — extended-thinking constraint: when the manifest
//!   sets `[quirks].send-thinking-blocks = true`, the request MUST
//!   send `temperature = 1.0` AND `max_tokens > thinking.budget_tokens`
//!   (1024). See `vertical_eval_judge_live::judge_temperature_for` and
//!   `judge_max_tokens_for`.
//! - **Bedrock** — Claude-on-Bedrock honours the same Anthropic
//!   thinking constraint when the manifest's underlying model is a
//!   Claude. We detect via the manifest's `[quirks]` flag, mirroring
//!   the Anthropic family path.
//! - **OpenAI Responses** — the o-series reasoning models (o1 / o3 /
//!   o4) reject `temperature != 1.0`. We detect via
//!   [`is_openai_o_series_model`] and omit the temperature override
//!   entirely (provider default is 1.0).
//! - **OpenAI Compatible** — plain Chat Completions; deterministic
//!   `temperature = 0.0` is fine.

use super::vertical_eval_judge_live::{
    judge_max_tokens_for, judge_temperature_for, manifest_enables_thinking, JudgeBackend,
};
use argo_provider::{
    AnthropicTransport, BedrockTransport, OpenAiCompatTransport, OpenAiResponsesTransport,
    ProviderFamily, ProviderManifest, ProviderRequest, Sampling, Transport,
};
use argo_types::{ArgoError, ArgoResult, ContentBlock, Message, MessageId, MessageRole, SessionId};
use async_trait::async_trait;
use chrono::Utc;
use std::path::PathBuf;
use std::sync::Arc;

/// System prompt shared by every family backend. Tight wording → judge
/// verdicts are stable for the same input across families.
const JUDGE_SYSTEM: &str = "You are a strict, deterministic eval grader. Reply with one fenced ```json block matching the requested shape. No prose around the block.";

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
        std::env::current_dir().map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))?;
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
fn read_api_key(manifest: &ProviderManifest, provider_id: &str) -> ArgoResult<String> {
    let env_var = manifest.auth.api_key_env.clone();
    std::env::var(&env_var).map_err(|_| {
        ArgoError::Provider(format!(
            "live judge: provider `{provider_id}` requires env `{env_var}` (resolved from \
             manifest `auth.api_key_env`). Set it and re-run, or use --judge heuristic for an \
             offline run."
        ))
    })
}

fn one_user_message(prompt: &str) -> Vec<Message> {
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

fn require_family(
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

// ===================================================================
// Anthropic
// ===================================================================

/// Anthropic Messages backend. Honours the `[quirks].send-thinking-blocks`
/// flag — when enabled, sends `temperature = 1.0` and a `max_tokens`
/// strictly greater than `thinking.budget_tokens` (1024).
pub(super) struct AnthropicJudgeBackend {
    transport: Arc<AnthropicTransport>,
    model: String,
    provider_id: String,
    thinking_enabled: bool,
}

impl std::fmt::Debug for AnthropicJudgeBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AnthropicJudgeBackend")
            .field("provider_id", &self.provider_id)
            .field("model", &self.model)
            .field("thinking_enabled", &self.thinking_enabled)
            .finish()
    }
}

#[async_trait]
impl JudgeBackend for AnthropicJudgeBackend {
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
                temperature: Some(judge_temperature_for(self.thinking_enabled)),
                max_tokens: Some(judge_max_tokens_for(self.thinking_enabled)),
                ..Sampling::default()
            },
        };
        let resp = self.transport.complete(&req).await?;
        Ok(resp.text.unwrap_or_default())
    }
}

/// Build an Anthropic judge backend.
pub(super) fn build_anthropic_backend(
    provider_id: &str,
    model: &str,
) -> ArgoResult<AnthropicJudgeBackend> {
    let manifest = load_manifest(provider_id)?;
    require_family(
        &manifest,
        ProviderFamily::Anthropic,
        "build_anthropic_backend",
    )?;
    let api_key = read_api_key(&manifest, provider_id)?;
    let thinking_enabled = manifest_enables_thinking(&manifest);
    let transport = AnthropicTransport::from_manifest(manifest, &api_key)?;
    Ok(AnthropicJudgeBackend {
        transport: Arc::new(transport),
        model: model.to_string(),
        provider_id: provider_id.to_string(),
        thinking_enabled,
    })
}

// ===================================================================
// Bedrock
// ===================================================================

/// AWS Bedrock Converse backend. Honours the same Anthropic
/// thinking-mode constraint when the manifest enables it (Claude
/// models on Bedrock follow the same temperature/max-tokens rules
/// as direct Anthropic).
pub(super) struct BedrockJudgeBackend {
    transport: Arc<BedrockTransport>,
    model: String,
    provider_id: String,
    thinking_enabled: bool,
}

impl std::fmt::Debug for BedrockJudgeBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BedrockJudgeBackend")
            .field("provider_id", &self.provider_id)
            .field("model", &self.model)
            .field("thinking_enabled", &self.thinking_enabled)
            .finish()
    }
}

#[async_trait]
impl JudgeBackend for BedrockJudgeBackend {
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
                temperature: Some(judge_temperature_for(self.thinking_enabled)),
                max_tokens: Some(judge_max_tokens_for(self.thinking_enabled)),
                ..Sampling::default()
            },
        };
        let resp = self.transport.complete(&req).await?;
        Ok(resp.text.unwrap_or_default())
    }
}

/// Build a Bedrock judge backend. Async because the AWS SDK's default
/// credential chain (`aws_config::load_defaults`) is async.
pub(super) async fn build_bedrock_backend(
    provider_id: &str,
    model: &str,
) -> ArgoResult<BedrockJudgeBackend> {
    let manifest = load_manifest(provider_id)?;
    require_family(&manifest, ProviderFamily::Bedrock, "build_bedrock_backend")?;
    let thinking_enabled = manifest_enables_thinking(&manifest);
    let aws_config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
    let transport = BedrockTransport::from_manifest(manifest, &aws_config)?;
    Ok(BedrockJudgeBackend {
        transport: Arc::new(transport),
        model: model.to_string(),
        provider_id: provider_id.to_string(),
        thinking_enabled,
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
        let err = build_openai_compat_backend(
            "definitely-not-a-real-provider-id-67890",
            "gpt-4o",
        )
        .unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("definitely-not-a-real-provider-id-67890"), "{msg}");
        assert!(msg.contains("manifests/providers"), "{msg}");
    }

    #[test]
    fn build_openai_responses_backend_errors_clearly_when_manifest_missing() {
        let err = build_openai_responses_backend(
            "definitely-not-a-real-provider-id-67890",
            "o1",
        )
        .unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("definitely-not-a-real-provider-id-67890"), "{msg}");
        assert!(msg.contains("manifests/providers"), "{msg}");
    }

    #[tokio::test]
    async fn build_bedrock_backend_errors_clearly_when_manifest_missing() {
        let err = build_bedrock_backend(
            "definitely-not-a-real-provider-id-67890",
            "anthropic.claude-3-haiku",
        )
        .await
        .unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("definitely-not-a-real-provider-id-67890"), "{msg}");
        assert!(msg.contains("manifests/providers"), "{msg}");
    }
}
