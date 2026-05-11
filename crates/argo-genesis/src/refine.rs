//! Stream `claude-genesis-llm-refinement` — opt-in LLM-driven
//! refinement pass on top of the deterministic Genesis pipeline.
//!
//! The G-genesis PR body promised this seam:
//!
//! > The orchestrator's shape leaves room for an LLM refinement
//! > pass to slot in without reshaping.
//!
//! This module ships that pass. It runs strictly AFTER the
//! deterministic pipeline (so the operator gets a known-good
//! draft no matter what the LLM does) and re-runs the safety
//! judge on every action the model proposes (so a sensitive verb
//! the model marks autonomous-safe gets auto-fixed before the
//! manifest is written).
//!
//! ## Behaviour
//!
//! 1. Take a finalised `GenesisPlan` from
//!    [`crate::pipeline::plan`].
//! 2. Build a strict JSON-schema prompt asking the model to
//!    confirm/revise the archetype, suggest additional actions
//!    (up to [`RefineConfig::max_new_actions`]), and add 5–10
//!    lines of charter-specific context to the system prompt.
//! 3. Call Anthropic via [`AnthropicCaller`] (the trait below —
//!    the production impl is in [`crate::refine::http`]; tests
//!    inject a fake).
//! 4. Validate the response against [`RefinementResponse`]. On
//!    parse failure, log a `tracing::warn!` and return the input
//!    plan unchanged.
//! 5. Apply the validated changes:
//!    - `archetype_confirmed` is informational; we don't switch
//!      archetypes mid-pipeline.
//!    - `additional_actions` are merged into the plan's actions
//!      map; the safety judge re-runs (auto-fix included) on the
//!      merged set.
//!    - `system_prompt_addendum` is appended to the existing
//!      prompt (with a separator) — never overwrites.
//!
//! ## Configuration
//!
//! [`RefineConfig::from_env`] returns a config built from
//! `ANTHROPIC_API_KEY` (required), `SKYCOT_REFINE_MODEL`
//! (optional, defaults to `claude-sonnet-4-6`), and
//! `ARGO_GENESIS_REFINE_MAX_ACTIONS` (optional, defaults to
//! [`RefineConfig::DEFAULT_MAX_NEW_ACTIONS`]).

use crate::error::GenesisError;
use crate::pipeline::GenesisPlan;
use crate::safety_judge;
use argo_vertical::{ActionDeclaration, ActionRiskLevel, ActionsSection, ExecutorDeclaration};
use serde::{Deserialize, Serialize};

/// Optional refinement config. Built once at cli startup; passed
/// to [`refine_plan`] when `--refine` is on.
#[derive(Debug, Clone)]
pub struct RefineConfig {
    /// Anthropic API key. Required when `--refine` is passed.
    pub api_key: String,
    /// Model id to use for the refinement call. Defaults to
    /// `claude-sonnet-4-6` per the brief's note that Sonnet is
    /// the right tier for structured output.
    pub model: String,
    /// API base URL. Tests override; production defaults to
    /// `https://api.anthropic.com`.
    pub base_url: String,
    /// Soft cap on how many additional actions the refinement
    /// pass will keep. The model is asked for up to this many in
    /// the prompt, and the parser silently truncates anything
    /// over the cap (a debug-level log records the truncation).
    /// Default is [`Self::DEFAULT_MAX_NEW_ACTIONS`]; overridable
    /// via `ARGO_GENESIS_REFINE_MAX_ACTIONS`.
    pub max_new_actions: usize,
}

impl RefineConfig {
    /// Default model id for the refinement pass.
    pub const DEFAULT_MODEL: &'static str = "claude-sonnet-4-6";

    /// Default base URL — Anthropic's public endpoint.
    pub const DEFAULT_BASE_URL: &'static str = "https://api.anthropic.com";

    /// Default soft cap on refined `additional_actions`.
    /// Generous enough for charters that genuinely need 4-6 new
    /// actions; conservative enough to bound runaway LLM output.
    pub const DEFAULT_MAX_NEW_ACTIONS: usize = 8;

    /// Build from env. Returns
    /// [`GenesisError::Validation`] when `ANTHROPIC_API_KEY` is
    /// unset — the cli surfaces this at startup so the operator
    /// sees the gap before the deterministic pipeline runs.
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
        let max_new_actions = max_new_actions_from_env();
        Ok(Self {
            api_key,
            model,
            base_url,
            max_new_actions,
        })
    }
}

/// Read the soft cap from the env, defaulting to
/// [`RefineConfig::DEFAULT_MAX_NEW_ACTIONS`] when unset or
/// unparseable. Used both by [`RefineConfig::from_env`] and by
/// [`refine_plan`] (which doesn't take a `RefineConfig` directly
/// — the cli builds the config and constructs the caller, but
/// `refine_plan`'s public seam is `(plan, caller)` and we
/// preserve that signature here).
fn max_new_actions_from_env() -> usize {
    std::env::var("ARGO_GENESIS_REFINE_MAX_ACTIONS")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .filter(|n| *n > 0)
        .unwrap_or(RefineConfig::DEFAULT_MAX_NEW_ACTIONS)
}

/// One additional action the model proposes. The number kept
/// is bounded by [`RefineConfig::max_new_actions`] (default
/// [`RefineConfig::DEFAULT_MAX_NEW_ACTIONS`]); anything over the
/// cap is silently truncated by [`refine_plan`]. Each kept
/// action is auto-fix + safety-judge re-run before merging into
/// the plan.
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
/// Anything that doesn't deserialise cleanly into this shape is
/// rejected; the pipeline falls back to the deterministic
/// output.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RefinementResponse {
    /// Whether the model agrees with the deterministic
    /// archetype assignment. Informational — we don't switch
    /// archetypes mid-pipeline.
    pub archetype_confirmed: bool,
    /// Optional model-picked archetype name (informational; if
    /// it disagrees, we still trust the deterministic
    /// classifier).
    #[serde(default)]
    pub archetype_suggested: Option<String>,
    /// Additional actions the model thinks the seed vocab
    /// missed. The number kept is bounded by
    /// [`RefineConfig::max_new_actions`]; [`refine_plan`]
    /// truncates any excess before merging.
    #[serde(default)]
    pub additional_actions: Vec<ProposedAction>,
    /// 5-10 lines of charter-specific context appended to the
    /// existing system prompt. Empty string = no addendum.
    #[serde(default)]
    pub system_prompt_addendum: String,
}

impl RefinementResponse {
    /// Sanity bounds on the addendum: non-binding length-wise
    /// but must not be abusive (e.g. 100KB). We cap at 4096
    /// chars. The action-count cap is handled by
    /// [`refine_plan`] via silent truncation, not by this
    /// validator — over-cap counts are not parse errors.
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

/// Silently truncate `additional_actions` down to `max`. A
/// debug-level log records the cap firing so operators who care
/// can tell the difference between "model under-shot" and "cap
/// limited the output".
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
/// production impl is in [`http`] below; tests inject a fake
/// (synchronous, deterministic).
#[async_trait::async_trait]
pub trait AnthropicCaller: Send + Sync {
    /// Make one call. Returns the model's raw JSON response. The
    /// caller validates against [`RefinementResponse`].
    async fn call(&self, prompt: String) -> Result<String, GenesisError>;
}

/// Refine a deterministic plan via the LLM. Errors during the
/// LLM call or validation are logged + swallowed — the input
/// plan is returned unchanged in that case so a transient
/// network failure doesn't fail the whole `argo genesis create`
/// invocation.
pub async fn refine_plan(
    plan: GenesisPlan,
    caller: &dyn AnthropicCaller,
) -> Result<GenesisPlan, GenesisError> {
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
            return Ok(plan);
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
                return Ok(plan);
            }
            apply_refinement(plan, resp)
        }
        Err(parse_err) => {
            let preview: String = raw.chars().take(200).collect();
            tracing::warn!(
                target: "argo_genesis::refine",
                error = %parse_err,
                preview = %preview,
                "refinement response did not match schema; falling back",
            );
            Ok(plan)
        }
    }
}

/// Apply the validated refinement. Merges new actions into the
/// plan's manifest, runs the safety judge auto-fix + final
/// check, and appends the addendum to the system prompt.
pub fn apply_refinement(
    mut plan: GenesisPlan,
    response: RefinementResponse,
) -> Result<GenesisPlan, GenesisError> {
    // Merge additional actions into the manifest.
    if let Some(actions) = plan.manifest.actions.as_mut() {
        merge_proposed_actions(actions, response.additional_actions);
        // Re-run the safety judge with auto-fix on the merged
        // set; sensitive verbs the model marked autonomous-safe
        // get fixed before the judge fails.
        safety_judge::auto_fix(actions);
        safety_judge::judge(actions)?;
    }
    // Append the addendum to the system prompt placeholder.
    // Genesis's pipeline writes the prompt to `prompts/system.md`
    // separately; here we stash the addendum on the plan's
    // manifest description so the writer picks it up. Future
    // work: thread an explicit addendum field through the plan.
    if !response.system_prompt_addendum.is_empty() {
        plan.manifest.vertical.description = format!(
            "{}\n\n## Refined addendum\n{}",
            plan.manifest.vertical.description, response.system_prompt_addendum
        );
    }
    Ok(plan)
}

fn merge_proposed_actions(actions: &mut ActionsSection, proposed: Vec<ProposedAction>) {
    for p in proposed {
        actions
            .actions
            .entry(p.action_type.clone())
            .or_insert_with(|| ActionDeclaration {
                risk: p.risk,
                requires_approval: p.requires_approval,
                description: p.description,
                executor: ExecutorDeclaration::default(),
                ..ActionDeclaration::default()
            });
    }
}

fn build_prompt(plan: &GenesisPlan, max_new_actions: usize) -> String {
    let archetype_label = plan.archetype.label();
    let action_names: Vec<&str> = plan
        .manifest
        .actions
        .as_ref()
        .map(|a| a.actions.keys().map(String::as_str).collect())
        .unwrap_or_default();
    format!(
        "You're refining a draft Skycot vertical generated from a charter.\n\n\
         Charter: {charter}\n\n\
         Deterministic archetype: {archetype_label}\n\
         Deterministic actions: [{actions}]\n\n\
         Respond with a JSON object exactly matching this schema:\n\
         {{\n\
           \"archetype_confirmed\": <bool>,\n\
           \"archetype_suggested\": <string or null>,\n\
           \"additional_actions\": [\n\
             {{ \"action_type\": <snake_case>, \"risk\": \"low\"|\"medium\"|\"high\",\n\
                \"requires_approval\": <bool>, \"description\": <string> }}\n\
           ] (propose up to {max_new_actions} new actions),\n\
           \"system_prompt_addendum\": <string up to 4096 chars>\n\
         }}\n\n\
         Do not propose actions that already exist. Do not propose actions \
         that touch external systems autonomously — sensitive verbs (send, \
         post, pay, publish, delete, share, message) MUST be \
         requires_approval=true.",
        charter = plan.charter,
        archetype_label = archetype_label,
        actions = action_names.join(", "),
        max_new_actions = max_new_actions,
    )
}

/// Simple no-op [`AnthropicCaller`] for tests that don't need
/// to exercise the response path.
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

/// HTTP-backed [`AnthropicCaller`] used in production. Lives in
/// a sub-module so the test path can mock it without pulling in
/// reqwest's tls stack.
pub mod http {
    use super::*;
    use reqwest::Client;

    /// Production caller. Uses the existing reqwest workspace
    /// dep — no new HTTP client crate.
    pub struct ReqwestCaller {
        client: Client,
        config: RefineConfig,
    }

    impl ReqwestCaller {
        /// Build with a client carrying a 30s per-request
        /// timeout (matches the cu executor's convention).
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
            // The Anthropic response shape: { "content": [{ "type": "text", "text": "..." }] }
            // Extract the first text block — the cli passes that to RefinementResponse::deserialize.
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
