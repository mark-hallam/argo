//! Real provider-backed [`SkillDistillationCaller`] — distils a cohort
//! of successful trajectories into a reusable SKILL.md body via the
//! SAME shared LLM transport the pitch / genesis / ghostwriter routes
//! use (`argo_agent_runner::{build_transport, load_manifest}` →
//! `argo_provider::Transport`).
//!
//! This is the live counterpart to
//! [`argo_skill::DeterministicDistillationCaller`] (which ships a
//! mechanical reformat of the anchor trajectory — no model, no network).
//! It is **opt-in**: the self-improve tick selects it only when the live
//! flag is ON (see [`crate::handlers::skill_self_improve`]); the default
//! stays deterministic so CI and the demo are reproducible.
//!
//! ## Eval boundary (load-bearing — DEC-042 / NO-THEATER)
//! Making distillation real does NOT change the promotion gate. A
//! distilled skill is proposed **draft-only** and graduates ONLY when
//! the existing measured A/B-eval (`run_ab_eval` → `decide_promotion`)
//! clears the [`argo_skill::PromotionPolicy`] floor. The verifier is a
//! measured pass-rate, never the model's self-report.
//!
//! Provider selection mirrors the pitch route's
//! `ARGO_PITCH_LLM_PROVIDER`: [`ENV_DISTILL_PROVIDER`] resolves the
//! manifest id (default `anthropic`; set `claude-code-cli` to route
//! through the subprocess driver / Claude Code subscription).

use argo_agent_runner::{build_transport, load_manifest};
use argo_provider::{ProviderRequest, Sampling, Transport};
use argo_skill::dreams_distillation::{Cohort, DistillationResult, SkillDistillationCaller};
use argo_types::{
    ArgoError, ArgoResult, ContentBlock, Message, MessageId, MessageRole, SessionId,
};
use async_trait::async_trait;
use chrono::Utc;
use std::sync::Arc;
use uuid::Uuid;

/// Env var that names the provider manifest used for distillation.
/// Mirrors `ARGO_PITCH_LLM_PROVIDER`.
pub const ENV_DISTILL_PROVIDER: &str = "ARGO_SKILL_DISTILL_PROVIDER";

/// Env var that turns the live caller ON in the self-improve tick.
/// Default OFF → the deterministic caller runs (reproducible CI/demo).
pub const ENV_DISTILL_LIVE: &str = "ARGO_SKILL_DISTILL_LIVE";

/// Default provider manifest id.
pub const DEFAULT_DISTILL_PROVIDER: &str = "anthropic";

/// Model id requested for distillation. Pinned here (like the pitch
/// route's `PITCH_MODEL`) so a demo run doesn't roll forward mid-call.
const DISTILL_MODEL: &str = "claude-sonnet-4-6";

/// Output cap — a SKILL.md body is a page or two of Markdown.
const DISTILL_MAX_TOKENS: u32 = 1_500;

/// Cost-estimate rates in USD micros per token for `DISTILL_MODEL`
/// (Sonnet-class: ~$3 / 1M input, ~$15 / 1M output). Used only to feed
/// the caller-site daily-cap accounting — an estimate, not a bill.
const COST_MICROS_PER_INPUT_TOKEN: i64 = 3;
const COST_MICROS_PER_OUTPUT_TOKEN: i64 = 15;

/// Live, provider-backed skill-distillation caller.
///
/// Construct with [`ProviderDistillationCaller::from_env`] in production
/// (resolves the manifest per call). A test-only `with_transport`
/// constructor injects a stub transport so the live `distill` path can be
/// exercised without a network.
pub struct ProviderDistillationCaller {
    provider_id: String,
    /// Injected transport. When `None`, [`Self::distill`] builds one from
    /// `provider_id` via the shared `build_transport`/`load_manifest`
    /// machinery on each call.
    transport: Option<Arc<dyn Transport>>,
}

impl ProviderDistillationCaller {
    /// Build a caller for an explicit provider manifest id. The transport
    /// is resolved lazily on each [`Self::distill`] call.
    #[must_use]
    pub fn new(provider_id: impl Into<String>) -> Self {
        Self {
            provider_id: provider_id.into(),
            transport: None,
        }
    }

    /// Build a caller from the environment. Reads [`ENV_DISTILL_PROVIDER`]
    /// (default [`DEFAULT_DISTILL_PROVIDER`]).
    #[must_use]
    pub fn from_env() -> Self {
        let provider_id =
            std::env::var(ENV_DISTILL_PROVIDER).unwrap_or_else(|_| DEFAULT_DISTILL_PROVIDER.into());
        Self::new(provider_id)
    }

    /// Build a caller with a pre-supplied transport. The `provider_id` is
    /// retained for diagnostics only; the supplied transport is used
    /// verbatim (no manifest load). Test-only today — exercises the live
    /// `distill` path against a stub transport without a network; promote
    /// to a public constructor when a production caller holds a shared
    /// transport to inject.
    #[cfg(test)]
    pub fn with_transport(provider_id: impl Into<String>, transport: Arc<dyn Transport>) -> Self {
        Self {
            provider_id: provider_id.into(),
            transport: Some(transport),
        }
    }

    /// Resolve the transport: the injected one if present, else build it
    /// from the manifest via the SHARED `build_transport`/`load_manifest`
    /// dispatcher (the same path `build_pitch_transport` takes).
    async fn transport(&self) -> ArgoResult<Arc<dyn Transport>> {
        if let Some(t) = &self.transport {
            return Ok(Arc::clone(t));
        }
        let cwd = std::env::current_dir().map_err(|e| ArgoError::Io(format!("current_dir: {e}")))?;
        let manifest = load_manifest(&cwd, &self.provider_id).map_err(|e| {
            ArgoError::Provider(format!(
                "skill distill: load provider `{}` manifest: {e}",
                self.provider_id
            ))
        })?;
        let boxed: Box<dyn Transport> = build_transport(manifest).await?;
        Ok(Arc::from(boxed))
    }
}

#[async_trait]
impl SkillDistillationCaller for ProviderDistillationCaller {
    async fn distill(&self, cohort: &Cohort) -> ArgoResult<DistillationResult> {
        let req = ProviderRequest {
            model: DISTILL_MODEL.to_string(),
            messages: vec![user_message(&render_synthesis_prompt(cohort))],
            tools: Vec::new(),
            system: Some(SYSTEM_PROMPT.to_string()),
            sampling: Sampling {
                temperature: Some(0.2),
                max_tokens: Some(DISTILL_MAX_TOKENS),
                ..Sampling::default()
            },
        };
        let transport = self.transport().await?;
        let resp = transport.complete(&req).await?;
        let body = resp
            .text
            .filter(|t| !t.trim().is_empty())
            .ok_or_else(|| ArgoError::Provider("skill distill: empty model response".into()))?;
        // Echo the model the transport actually reported when present, so
        // the audit/provenance row is truthful; fall back to the request
        // model id otherwise. Either way it is NOT "deterministic".
        let model = resp
            .provider_meta
            .get("model")
            .and_then(|v| v.as_str())
            .map_or_else(|| DISTILL_MODEL.to_string(), str::to_string);
        let cost_micros = estimate_cost_micros(resp.tokens_in, resp.tokens_out);
        Ok(DistillationResult {
            body,
            model,
            cost_micros,
        })
    }
}

/// System prompt — instructs the model to write a reusable SKILL.md body
/// that generalises the cohort's successful runs (not a transcript).
const SYSTEM_PROMPT: &str = "You are a skill-distillation engine for an AI-teammate platform. \
You are given a cohort of successful task runs (their step sequences, final outputs, and \
success metrics). Write a single reusable SKILL.md body in Markdown that GENERALISES the \
pattern these runs share, so a teammate can reproduce the success on a new instance of the \
same task. Output ONLY the Markdown body: a one-line title heading, a short purpose line, \
and a numbered '## Steps' procedure. Do not restate any single transcript verbatim, do not \
invent steps the runs did not perform, and do not include preamble or commentary.";

/// Render the synthesis prompt from the cohort. Each member contributes
/// its trajectory steps, final captured text, and success metric so the
/// model can find the shared pattern.
fn render_synthesis_prompt(cohort: &Cohort) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "Distil a reusable skill from {n} successful `{action}` runs in the `{vertical}` vertical.\n\n",
        n = cohort.members.len(),
        action = cohort.action_type,
        vertical = cohort.vertical_id,
    ));
    for (i, m) in cohort.members.iter().enumerate() {
        out.push_str(&format!(
            "## Run {idx} (success {metric:.2})\n",
            idx = i + 1,
            metric = m.success_metric,
        ));
        out.push_str("Steps:\n");
        for step in &m.trajectory {
            out.push_str(&format!("- {step}\n"));
        }
        if let Some(final_text) = &m.final_text {
            if !final_text.is_empty() {
                out.push_str(&format!("Final output: {final_text}\n"));
            }
        }
        out.push('\n');
    }
    out.push_str(
        "Now write the generalised SKILL.md body (title heading, purpose line, numbered Steps).\n",
    );
    out
}

/// Estimate the USD-micros cost of one distillation call from the
/// provider-reported token counts. Missing counts contribute 0.
fn estimate_cost_micros(tokens_in: Option<u32>, tokens_out: Option<u32>) -> i64 {
    let cin = i64::from(tokens_in.unwrap_or(0)) * COST_MICROS_PER_INPUT_TOKEN;
    let cout = i64::from(tokens_out.unwrap_or(0)) * COST_MICROS_PER_OUTPUT_TOKEN;
    cin + cout
}

/// One user message carrying the synthesis prompt. Mirrors the pitch
/// route's `user_message` shaping (nil session id — this is a one-shot
/// completion, not a persisted conversation).
fn user_message(prompt: &str) -> Message {
    Message {
        id: MessageId(Uuid::now_v7()),
        session_id: SessionId(Uuid::nil()),
        role: MessageRole::User,
        content: vec![ContentBlock::Text {
            text: prompt.to_string(),
        }],
        tool_calls: Vec::new(),
        tool_call_id: None,
        reasoning: None,
        created_at: Utc::now(),
        provider_meta: serde_json::Value::Null,
    }
}

#[cfg(test)]
#[path = "provider_distillation_caller_tests.rs"]
mod tests;
