//! Real model-backed delegation executor for `POST
//! /api/v1/orchestration/team-runs` (DEC-023 P3).
//!
//! The Supervisor engine ([`argo_orchestrator::Supervisor`]) makes the
//! team-run *topology* real — bounded-concurrency fan-out, `depends_on`
//! dependency layers, failure-drops-dependents, tenant-scoped
//! orchestration-run rows. But [`argo_orchestrator::PersistingExecutor`]
//! returns a deterministic acknowledgement string per step (the honest
//! seam): the rows are real, the *work* is a placeholder.
//!
//! [`RuntimeStepExecutor`] closes that gap. For each delegated step it
//! calls a **real model** through the SAME provider abstraction Genesis
//! and the Pitch Agent use
//! (`argo_agent_runner::{build_transport, load_manifest}` →
//! `argo_provider::Transport::complete`), producing genuine output text
//! for the delegated `task`. It **composes** [`PersistingExecutor`]
//! rather than re-implementing it: the child run row + the lead's
//! synthesis row are persisted by the exact same writes
//! ([`PersistingExecutor::record_step_run`] /
//! [`PersistingExecutor::record_synthesis_run`]), so topology + the
//! audit-coherent `[team:<id>]` row tree are byte-for-byte unchanged —
//! only the per-step `detail` (and the lead-synthesis headline) become
//! real model output.
//!
//! ## Flag-gated (mirrors Genesis's `ARGO_GENESIS_LLM_CLASSIFIER`)
//!
//! Default **OFF**. The team-runs route uses [`PersistingExecutor`]
//! unless [`ORCH_LLM_EXECUTOR_ENV`] (`ARGO_ORCH_LLM_EXECUTOR`) is set to
//! a truthy value, keeping the test suite deterministic (no subprocess
//! dial-out). Per DEC-012 the dev/demo provider is `claude-code-cli`
//! (billed against the operator's Claude Code subscription, not the
//! metered Anthropic API); the provider id is read from
//! [`ORCH_LLM_PROVIDER_ENV`], falling back to the pitch/genesis path's
//! `ARGO_PITCH_LLM_PROVIDER`, then `claude-code-cli`. No HTTP API is
//! hardcoded — the manifest's `transport_kind` decides the driver.
//!
//! ## No theater
//!
//! Any provider error on a step falls back to
//! [`PersistingExecutor::deterministic_detail`] for that step — so a
//! transient model hiccup never hard-breaks a run, and a fallback reads
//! identically to a deterministic run rather than masquerading as real
//! output. Output is truncated to [`MAX_STEP_OUTPUT_CHARS`] so one
//! verbose step cannot bloat the row tree.

use std::sync::Arc;

use argo_agent_runner::{build_transport, load_manifest};
use argo_orchestrator::{PersistingExecutor, StepContext, StepExecutor, StepId, TeamPlan};
use argo_provider::{ProviderRequest, Sampling};
use argo_types::{
    ArgoError, ArgoResult, ContentBlock, Message, MessageId, MessageRole, SessionId, TenantId,
};
use async_trait::async_trait;
use chrono::Utc;
use tokio::sync::Mutex;
use uuid::Uuid;

/// Explicit opt-in for the real model-backed delegation executor.
/// Defaults **off** so test runs stay on the deterministic
/// [`PersistingExecutor`] path — byte-identical rows, no subprocess
/// dial-out, no flakiness. The live gateway turns it on by exporting
/// `ARGO_ORCH_LLM_EXECUTOR=1` in `~/.argo/env.local`. Mirrors the
/// Genesis path's `ARGO_GENESIS_LLM_CLASSIFIER` opt-in discipline.
pub(super) const ORCH_LLM_EXECUTOR_ENV: &str = "ARGO_ORCH_LLM_EXECUTOR";

/// Env var naming the provider manifest used for delegated-step
/// generation. Defaults are resolved in [`resolve_provider_id`].
pub(super) const ORCH_LLM_PROVIDER_ENV: &str = "ARGO_ORCH_LLM_PROVIDER";

/// Default delegation model. The `claude-code-cli` subprocess driver
/// ignores this (the CLI auto-routes), but the HTTP families need a
/// concrete id, so it is pinned here rather than `"latest"`.
const ORCH_MODEL: &str = "claude-sonnet-4-6";

/// Per-step generation budget. Delegated steps are short pieces of
/// work, not essays; this caps cost + latency while leaving room for a
/// substantive answer.
const ORCH_MAX_TOKENS: u32 = 700;

/// Hard ceiling on the stored per-step `detail` length. A verbose model
/// answer is truncated (with an ellipsis marker) so a single step
/// cannot bloat the orchestration-run row tree.
const MAX_STEP_OUTPUT_CHARS: usize = 4_000;

/// Ceiling on the lead-synthesis headline appended to the synthesis run
/// row's title. Kept short so the `/orchestration` lead row reads as a
/// one-line summary, never a wall of text.
const MAX_SYNTHESIS_CHARS: usize = 600;

/// `true` when [`ORCH_LLM_EXECUTOR_ENV`] is set to a truthy value
/// (`1` / `true` / `on` / `yes`, case-insensitive). Absent / empty /
/// any other value reads as off.
pub(super) fn llm_executor_enabled() -> bool {
    matches!(
        std::env::var(ORCH_LLM_EXECUTOR_ENV)
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase()
            .as_str(),
        "1" | "true" | "on" | "yes"
    )
}

/// Resolve the provider id for the orchestration LLM executor.
///
/// Order: `ARGO_ORCH_LLM_EXECUTOR` provider override
/// (`ARGO_ORCH_LLM_PROVIDER`) → `ARGO_PITCH_LLM_PROVIDER` (so one
/// dev/demo export drives every LLM surface) → `claude-code-cli` (the
/// DEC-012 dev/demo default).
pub(super) fn resolve_provider_id() -> String {
    for key in [ORCH_LLM_PROVIDER_ENV, "ARGO_PITCH_LLM_PROVIDER"] {
        if let Ok(v) = std::env::var(key) {
            let trimmed = v.trim();
            if !trimmed.is_empty() {
                return trimmed.to_string();
            }
        }
    }
    "claude-code-cli".to_string()
}

/// One real per-step output, buffered so [`RuntimeStepExecutor::synthesize`]
/// can build the lead summary from the steps' *actual* outputs (the
/// trait's `synthesize` is not handed the outcomes).
#[derive(Clone)]
struct StepOutput {
    step_id: StepId,
    text: String,
    /// `true` when this output came from a real provider round-trip;
    /// `false` when the step fell back to the deterministic detail.
    real: bool,
}

/// Model-backed [`StepExecutor`]. Composes a [`PersistingExecutor`] for
/// the row writes (topology + audit chain unchanged) and dials a real
/// model for each step's output via the shared transport abstraction.
pub(super) struct RuntimeStepExecutor {
    persist: Arc<PersistingExecutor>,
    provider_id: String,
    /// Per-step real outputs, captured in execution order for the lead
    /// synthesis. Behind a mutex because the engine fans steps out
    /// concurrently within a layer.
    outputs: Mutex<Vec<StepOutput>>,
}

impl RuntimeStepExecutor {
    /// Build over a [`PersistingExecutor`] (reused for persistence) and
    /// the resolved provider id.
    pub(super) fn new(persist: Arc<PersistingExecutor>, provider_id: String) -> Self {
        Self {
            persist,
            provider_id,
            outputs: Mutex::new(Vec::new()),
        }
    }

    /// Generate the delegated worker's real output for one step, or fall
    /// back to the deterministic detail on any provider error. Returns
    /// the (possibly truncated) text plus whether it is genuine model
    /// output.
    async fn step_output(&self, ctx: &StepContext) -> (String, bool) {
        match complete_once(&self.provider_id, &delegation_prompt(ctx)).await {
            Ok(text) if !text.trim().is_empty() => {
                (truncate(text.trim(), MAX_STEP_OUTPUT_CHARS), true)
            }
            // Empty answer or any provider error → honest deterministic
            // fallback (never a hard break, never theater).
            Ok(_) => (PersistingExecutor::deterministic_detail(ctx), false),
            Err(e) => {
                tracing::warn!(
                    event = "orch.llm_executor.fallback",
                    step = %ctx.step_id,
                    worker = %ctx.to,
                    error = %e,
                    "delegated-step model call failed; using deterministic detail",
                );
                (PersistingExecutor::deterministic_detail(ctx), false)
            }
        }
    }
}

#[async_trait]
impl StepExecutor for RuntimeStepExecutor {
    async fn execute(&self, ctx: &StepContext) -> ArgoResult<(String, String)> {
        // Persist the SAME tenant-scoped, `[team:<id>]`-titled child row
        // the deterministic executor writes (topology + audit unchanged).
        let run_id = self.persist.record_step_run(ctx).await?;
        let (detail, real) = self.step_output(ctx).await;
        self.outputs.lock().await.push(StepOutput {
            step_id: ctx.step_id.clone(),
            text: detail.clone(),
            real,
        });
        Ok((run_id, detail))
    }

    async fn synthesize(
        &self,
        plan: &TeamPlan,
        tenant_id: TenantId,
        team_run_id: &str,
    ) -> ArgoResult<String> {
        // Build the real lead synthesis from the steps' ACTUAL outputs
        // BEFORE writing the row, so the headline lands through the same
        // single tenant-scoped insert (no title UPDATE, no migration).
        // Best-effort + no theater: only synthesize when at least one
        // step produced genuine model output; a provider error here just
        // omits the headline (canonical row still lands) — the run never
        // fails on synthesis.
        let outputs = self.outputs.lock().await.clone();
        let summary = self.lead_summary(plan, &outputs).await;
        self.persist
            .record_synthesis_run_with_summary(plan, tenant_id, team_run_id, summary.as_deref())
            .await
    }
}

impl RuntimeStepExecutor {
    /// Generate the lead-synthesis headline from the steps' real
    /// outputs, or `None` when no step produced genuine output / the
    /// provider errors (the canonical synthesis row then carries no
    /// headline — honest, never theater).
    async fn lead_summary(&self, plan: &TeamPlan, outputs: &[StepOutput]) -> Option<String> {
        if !outputs.iter().any(|o| o.real) {
            return None;
        }
        match complete_once(&self.provider_id, &synthesis_prompt(plan, outputs)).await {
            Ok(text) if !text.trim().is_empty() => {
                Some(truncate(text.trim(), MAX_SYNTHESIS_CHARS))
            }
            Ok(_) => None,
            Err(e) => {
                tracing::warn!(
                    event = "orch.llm_executor.synthesis_fallback",
                    lead = %plan.lead,
                    error = %e,
                    "lead-synthesis model call failed; row persisted without headline",
                );
                None
            }
        }
    }
}

/// Prompt for one delegated step: frame the model as the delegated
/// worker doing the `task`, and ask for the work product itself (not a
/// meta description). Kept provider-agnostic + persona-light because the
/// engine seam only carries the worker id, not a persona record.
fn delegation_prompt(ctx: &StepContext) -> String {
    format!(
        "You are an AI teammate (worker id {worker}) on a team led by another worker. \
You have been delegated the following task as part of a larger team effort. \
Do the work and return the work product itself — concise, concrete, and directly \
usable by the lead. Do not restate the instructions or describe what you would do; \
produce the actual result.\n\nTask: {task}",
        worker = ctx.to,
        task = ctx.task,
    )
}

/// Prompt for the lead synthesis: hand the model every completed step's
/// real output and ask for a short integrated summary the lead would
/// report up. Only the real outputs are worth synthesizing.
fn synthesis_prompt(plan: &TeamPlan, outputs: &[StepOutput]) -> String {
    let mut body = String::new();
    for o in outputs.iter().filter(|o| o.real) {
        body.push_str(&format!("- Step `{}`: {}\n", o.step_id, o.text));
    }
    format!(
        "You are the lead worker coordinating the team effort titled \"{title}\". \
Your teammates returned the results below. Write a short (2-4 sentence) synthesis \
integrating them into one coherent answer for the operator. Do not list the steps \
back; synthesize.\n\n{body}",
        title = plan.title,
        body = body,
    )
}

/// Truncate `s` to at most `max` chars, appending a short marker when it
/// was cut so a reader knows output was elided. Char-boundary safe.
fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let kept: String = s.chars().take(max.saturating_sub(1)).collect();
    format!("{kept}…")
}

/// One-shot completion against the named provider manifest. Mirrors the
/// pitch/genesis paths exactly: load the manifest from
/// `<cwd>/manifests/providers/<id>.toml` (subprocess vs HTTP decided by
/// the manifest), then `complete` a single user turn at temperature 0.
async fn complete_once(provider_id: &str, prompt: &str) -> ArgoResult<String> {
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Io(format!("current_dir: {e}")))?;
    let manifest = load_manifest(&cwd, provider_id).map_err(|e| {
        ArgoError::Provider(format!(
            "orch llm: load provider `{provider_id}` manifest: {e}"
        ))
    })?;
    let transport = build_transport(manifest).await?;
    let req = ProviderRequest {
        model: ORCH_MODEL.to_string(),
        messages: vec![user_message(prompt)],
        tools: Vec::new(),
        system: None,
        sampling: Sampling {
            temperature: Some(0.0),
            max_tokens: Some(ORCH_MAX_TOKENS),
            ..Sampling::default()
        },
    };
    let resp = transport.complete(&req).await?;
    Ok(resp.text.unwrap_or_default())
}

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
#[path = "routes_orchestration_team_llm_tests.rs"]
mod tests;
