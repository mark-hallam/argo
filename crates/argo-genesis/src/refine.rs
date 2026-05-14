//! Stream `claude-genesis-llm-refinement` — opt-in LLM-driven
//! refinement pass on top of the deterministic Genesis pipeline.
//!
//! Runs strictly AFTER the deterministic pipeline (so the
//! operator gets a known-good draft no matter what the LLM
//! does) and re-runs the safety judge on every action the model
//! proposes.
//!
//! ## Behaviour
//!
//! 1. Take a finalised `GenesisPlan` from `crate::pipeline::plan`.
//! 2. Build a strict JSON-schema prompt asking the model to
//!    confirm/revise the archetype, suggest additional actions
//!    (up to [`RefineConfig::max_new_actions`]), and add a
//!    charter-specific addendum to the system prompt.
//! 3. Call Anthropic via [`AnthropicCaller`]; tests inject a fake.
//! 4. Validate the response against [`RefinementResponse`]. On
//!    failure, log + return the input plan unchanged.
//! 5. Truncate over-cap responses, merge new actions (capped at
//!    [`RefineConfig::max_vocab_size`]), force
//!    `requires_approval = true` on Medium/High-risk newly-added
//!    actions, then run safety-judge auto-fix + final check.
//!
//! ## Configuration
//!
//! [`RefineConfig::from_env`] reads `ANTHROPIC_API_KEY`
//! (required), `SKYCOT_REFINE_MODEL`, `SKYCOT_REFINE_BASE_URL`,
//! `ARGO_GENESIS_REFINE_MAX_ACTIONS`, and
//! `ARGO_GENESIS_REFINE_MAX_VOCAB`.

use crate::error::GenesisError;
use crate::pipeline::GenesisPlan;
use argo_vertical::ActionRiskLevel;
use serde::{Deserialize, Serialize};

#[path = "refine_apply.rs"]
mod apply;
pub use apply::{apply_refinement, apply_refinement_with_cap};
use apply::build_prompt;

/// Optional refinement config. Built once at cli startup; passed
/// to [`refine_plan`] when `--refine` is on.
#[derive(Debug, Clone)]
pub struct RefineConfig {
    /// Anthropic API key. Required when `--refine` is passed.
    pub api_key: String,
    /// Model id to use for the refinement call.
    pub model: String,
    /// API base URL. Tests override; production defaults to
    /// [`Self::DEFAULT_BASE_URL`].
    pub base_url: String,
    /// Soft cap on how many additional actions the refinement
    /// pass will keep. The model is asked for up to this many in
    /// the prompt, and the parser silently truncates anything
    /// over the cap. Env override `ARGO_GENESIS_REFINE_MAX_ACTIONS`.
    pub max_new_actions: usize,
    /// Cap on the total number of action_types a refinement pass
    /// may grow the manifest's vocabulary to. Proposed entries
    /// whose action_type would push the vocabulary past this cap
    /// are silently dropped with a debug log. Env override
    /// `ARGO_GENESIS_REFINE_MAX_VOCAB`.
    pub max_vocab_size: usize,
}

impl RefineConfig {
    /// Default model id for the refinement pass.
    pub const DEFAULT_MODEL: &'static str = "claude-sonnet-4-6";

    /// Default base URL — Anthropic's public endpoint.
    pub const DEFAULT_BASE_URL: &'static str = "https://api.anthropic.com";

    /// Default soft cap on refined `additional_actions`.
    pub const DEFAULT_MAX_NEW_ACTIONS: usize = 8;

    /// Default vocabulary growth cap. Generous against real
    /// charters (8-20 actions); cap prevents runaway LLM
    /// hallucination growing the manifest to 200+ entries in a
    /// single pass.
    pub const DEFAULT_MAX_VOCAB_SIZE: usize = 64;

    /// Build from env. Returns [`GenesisError::Validation`] when
    /// `ANTHROPIC_API_KEY` is unset.
    pub fn from_env() -> Result<Self, GenesisError> {
        let api_key =
            argo_config::cred::get("ANTHROPIC_API_KEY").map_err(|_| GenesisError::Validation {
                stage: "refine.config",
                detail: "ANTHROPIC_API_KEY must be set when --refine is passed".into(),
            })?;
        let model = std::env::var("SKYCOT_REFINE_MODEL")
            .unwrap_or_else(|_| Self::DEFAULT_MODEL.to_string());
        let base_url = std::env::var("SKYCOT_REFINE_BASE_URL")
            .unwrap_or_else(|_| Self::DEFAULT_BASE_URL.to_string());
        Ok(Self {
            api_key,
            model,
            base_url,
            max_new_actions: max_new_actions_from_env(),
            max_vocab_size: max_vocab_size_from_env(),
        })
    }
}

fn max_new_actions_from_env() -> usize {
    std::env::var("ARGO_GENESIS_REFINE_MAX_ACTIONS")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .filter(|n| *n > 0)
        .unwrap_or(RefineConfig::DEFAULT_MAX_NEW_ACTIONS)
}

fn max_vocab_size_from_env() -> usize {
    std::env::var("ARGO_GENESIS_REFINE_MAX_VOCAB")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .filter(|n| *n > 0)
        .unwrap_or(RefineConfig::DEFAULT_MAX_VOCAB_SIZE)
}

/// One additional action the model proposes. The number kept is
/// bounded by [`RefineConfig::max_new_actions`]; anything over
/// the cap is silently truncated. Each kept action is auto-fix +
/// safety-judge re-run before merging into the plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProposedAction {
    /// `action_type` in canonical snake_case.
    pub action_type: String,
    /// `low | medium | high`.
    pub risk: ActionRiskLevel,
    /// Whether the action requires approval. The safety judge
    /// auto-fixes this to `true` for sensitive verbs regardless
    /// of what the model says, so a wrong value here is harmless.
    pub requires_approval: bool,
    /// One-line description.
    pub description: String,
}

/// Strict JSON-schema response from the refinement LLM call.
/// Anything that doesn't deserialise cleanly is rejected; the
/// pipeline falls back to the deterministic output.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RefinementResponse {
    /// Whether the model agrees with the deterministic
    /// archetype assignment. Informational only.
    pub archetype_confirmed: bool,
    /// Optional model-picked archetype name (informational).
    #[serde(default)]
    pub archetype_suggested: Option<String>,
    /// Additional actions the model thinks the seed vocab missed.
    #[serde(default)]
    pub additional_actions: Vec<ProposedAction>,
    /// 5-10 lines of charter-specific context appended to the
    /// existing system prompt.
    #[serde(default)]
    pub system_prompt_addendum: String,
    /// Action types newly added to the manifest's vocabulary in
    /// this refinement pass. Populated server-side by
    /// [`apply_refinement_with_cap`] — the LLM does not supply
    /// this. Empty when no new vocab grew. Exposed for operator
    /// visibility via `argo genesis refine --show-vocab-evolution`.
    #[serde(default)]
    pub evolved_vocab: Vec<String>,
}

impl RefinementResponse {
    fn validate(&self) -> Result<(), GenesisError> {
        if self.system_prompt_addendum.len() > 4096 {
            return Err(GenesisError::Validation {
                stage: "refine.parse",
                detail: format!(
                    "system_prompt_addendum is {} bytes; max 4096",
                    self.system_prompt_addendum.len()
                ),
            });
        }
        Ok(())
    }
}

fn truncate_additional_actions(resp: &mut RefinementResponse, max: usize) {
    let proposed = resp.additional_actions.len();
    if proposed > max {
        let dropped = proposed - max;
        resp.additional_actions.truncate(max);
        tracing::debug!(
            target: "argo_genesis::refine",
            max_new_actions = max,
            proposed,
            dropped,
            "refinement response over max_new_actions cap; truncated",
        );
    }
}

/// Trait the refinement pass calls to talk to Anthropic. The
/// production impl is in [`http`]; tests inject a fake.
#[async_trait::async_trait]
pub trait AnthropicCaller: Send + Sync {
    /// Make one call. Returns the model's raw JSON response.
    async fn call(&self, prompt: String) -> Result<String, GenesisError>;
}

/// Refine a deterministic plan via the LLM. Errors during the
/// LLM call or validation are logged + swallowed — the input
/// plan is returned unchanged so a transient failure doesn't
/// fail the whole `argo genesis create`. Uses the env-driven
/// vocabulary cap; callers that need to surface vocabulary
/// evolution should use [`refine_plan_with_cap`].
pub async fn refine_plan(
    plan: GenesisPlan,
    caller: &dyn AnthropicCaller,
) -> Result<GenesisPlan, GenesisError> {
    refine_plan_with_cap(plan, caller, max_vocab_size_from_env())
        .await
        .map(|(plan, _)| plan)
}

/// Same as [`refine_plan`] but caps how many action_types the
/// refinement may add to the manifest's vocabulary, and returns
/// the names of action_types newly added. The list mirrors
/// `RefinementResponse.evolved_vocab` and is what the
/// `argo genesis refine --show-vocab-evolution` flag prints.
pub async fn refine_plan_with_cap(
    plan: GenesisPlan,
    caller: &dyn AnthropicCaller,
    max_vocab_size: usize,
) -> Result<(GenesisPlan, Vec<String>), GenesisError> {
    let max_new_actions = max_new_actions_from_env();
    let prompt = build_prompt(&plan, max_new_actions);
    let raw = match caller.call(prompt).await {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!(
                target: "argo_genesis::refine",
                error = %e,
                "refinement call failed; falling back to deterministic plan",
            );
            return Ok((plan, Vec::new()));
        }
    };
    match serde_json::from_str::<RefinementResponse>(&raw) {
        Ok(mut resp) => {
            truncate_additional_actions(&mut resp, max_new_actions);
            if let Err(validate_err) = resp.validate() {
                tracing::warn!(
                    target: "argo_genesis::refine",
                    error = %validate_err,
                    "refinement response failed validation; falling back",
                );
                return Ok((plan, Vec::new()));
            }
            apply_refinement_with_cap(plan, resp, max_vocab_size)
        }
        Err(parse_err) => {
            let preview: String = raw.chars().take(200).collect();
            tracing::warn!(
                target: "argo_genesis::refine",
                error = %parse_err,
                preview = %preview,
                "refinement response did not match schema; falling back",
            );
            Ok((plan, Vec::new()))
        }
    }
}

/// Simple no-op [`AnthropicCaller`] for tests that don't need to
/// exercise the response path.
#[doc(hidden)]
pub struct NoopCaller {
    pub response: String,
}

#[async_trait::async_trait]
impl AnthropicCaller for NoopCaller {
    async fn call(&self, _prompt: String) -> Result<String, GenesisError> {
        Ok(self.response.clone())
    }
}

/// HTTP-backed [`AnthropicCaller`] used in production.
pub mod http {
    use super::*;
    use reqwest::Client;

    /// Production caller. Uses the existing reqwest workspace dep.
    pub struct ReqwestCaller {
        client: Client,
        config: RefineConfig,
    }

    impl ReqwestCaller {
        /// Build with a client carrying a 30s per-request timeout.
        pub fn new(config: RefineConfig) -> Result<Self, GenesisError> {
            let client = Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .map_err(|e| GenesisError::Validation {
                    stage: "refine.client.build",
                    detail: format!("reqwest client build: {e}"),
                })?;
            Ok(Self { client, config })
        }
    }

    #[async_trait::async_trait]
    impl AnthropicCaller for ReqwestCaller {
        async fn call(&self, prompt: String) -> Result<String, GenesisError> {
            let url = format!("{}/v1/messages", self.config.base_url);
            let body = serde_json::json!({
                "model": &self.config.model,
                "max_tokens": 1024,
                "messages": [
                    { "role": "user", "content": prompt }
                ],
            });
            let resp = self
                .client
                .post(&url)
                .header("x-api-key", &self.config.api_key)
                .header("anthropic-version", "2023-06-01")
                .json(&body)
                .send()
                .await
                .map_err(|e| GenesisError::Validation {
                    stage: "refine.http.send",
                    detail: format!("anthropic call: {e}"),
                })?;
            if !resp.status().is_success() {
                return Err(GenesisError::Validation {
                    stage: "refine.http.status",
                    detail: format!("anthropic returned HTTP {}", resp.status()),
                });
            }
            let raw: serde_json::Value =
                resp.json().await.map_err(|e| GenesisError::Validation {
                    stage: "refine.http.parse",
                    detail: format!("anthropic body: {e}"),
                })?;
            let text = raw
                .get("content")
                .and_then(|c| c.as_array())
                .and_then(|arr| {
                    arr.iter()
                        .find(|b| b.get("type").and_then(|t| t.as_str()) == Some("text"))
                })
                .and_then(|b| b.get("text"))
                .and_then(|t| t.as_str())
                .ok_or_else(|| GenesisError::Validation {
                    stage: "refine.http.shape",
                    detail: "anthropic response missing text block".into(),
                })?
                .to_string();
            Ok(text)
        }
    }
}

#[cfg(test)]
#[path = "refine_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "refine_evolve_vocab_tests.rs"]
mod evolve_vocab_tests;
