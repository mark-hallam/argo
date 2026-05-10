//! Anthropic-style [`JudgeBackend`] implementations.
//!
//! Sibling to `vertical_eval_judge_backends` (the OpenAI-family file)
//! so each stays under the 500-LOC budget. Houses the two families
//! that honour the `[quirks].send-thinking-blocks` manifest flag:
//!
//! - **Anthropic** — Messages API. When the manifest enables thinking,
//!   the request MUST send `temperature = 1.0` AND
//!   `max_tokens > thinking.budget_tokens` (1024). See
//!   `vertical_eval_judge_live::judge_temperature_for` and
//!   `judge_max_tokens_for`.
//! - **Bedrock** — Converse API for Claude-on-Bedrock, which inherits
//!   the same Anthropic thinking constraint when the manifest's
//!   underlying model is a Claude.
//!
//! The [`super::vertical_eval_judge_live::build_judge_backend`]
//! dispatcher picks the right backend based on the manifest's
//! `[provider].family`.

use super::vertical_eval_judge_backends::{
    load_manifest, one_user_message, read_api_key, require_family, JUDGE_SYSTEM,
};
use super::vertical_eval_judge_live::{
    judge_max_tokens_for, judge_temperature_for, manifest_enables_thinking, JudgeBackend,
};
use argo_provider::{
    AnthropicTransport, BedrockTransport, ProviderFamily, ProviderRequest, Sampling, Transport,
};
use argo_types::ArgoResult;
use async_trait::async_trait;
use std::sync::Arc;

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

    #[tokio::test]
    async fn build_bedrock_backend_errors_clearly_when_manifest_missing() {
        let err = build_bedrock_backend(
            "definitely-not-a-real-provider-id-67890",
            "anthropic.claude-3-haiku",
        )
        .await
        .unwrap_err();
        let msg = format!("{err}");
        assert!(
            msg.contains("definitely-not-a-real-provider-id-67890"),
            "{msg}"
        );
        assert!(msg.contains("manifests/providers"), "{msg}");
    }
}
