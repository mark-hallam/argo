//! Inner agent-loop helpers extracted from `turn.rs` to keep that
//! file under the 500-LOC ceiling (HARD RULE 6).
//!
//! Loaded via `#[path = "turn_helpers.rs"] mod turn_helpers;` from
//! `turn.rs`, mirroring the `turn_memory.rs` sibling pattern. The
//! tool-dispatch loop driven by [`run_turn_with_tools`] lives here;
//! the public entry points (`run_turn` / `run_turn_with_events`)
//! and the system-prompt composer remain in `turn.rs`. Context
//! compression / truncation helpers — invoked once per round before
//! the provider call — live in the `turn_helpers_context` sibling.
//!
//! No public API changes — `run_turn_with_tools` is `pub(super)`
//! and only `turn::run_turn_with_events` calls it.
use super::turn_helpers_context::prepare_context_messages;
use super::turn_helpers_overlay::{append_worker_overlays_as_workspace, merge_worker_overlays};
use super::turn_helpers_run::{
    build_turn_dispatch_ctx, dispatch_one_tool_call, emit_halt, finalize_turn, run_one_round,
};
use super::turn_skills_runtime::PendingAbOutcome;
use super::TurnResult;
use crate::helpers::{message_from_row, RunEventSink, SpendCtx};
use crate::turn_vertical::SessionGrantState;
use argo_audit::AuditSink;
use argo_provider::{ProviderRequest, Sampling, Transport};
use argo_skill::{
    LayeredLookup, Skill, SkillRelevanceConfig, SkillRelevanceSelector, SkillSelectionContext,
    SkillStore,
};
use argo_store::{SessionStore, WorkerId};
use argo_types::{ArgoError, ArgoResult, SessionId};
use std::path::PathBuf;
use std::sync::Arc;

pub(super) async fn load_prompt_skills(
    workspace: &SkillStore,
    session_id: SessionId,
    vertical_id: Option<&str>,
    worker_id: Option<WorkerId>,
) -> ArgoResult<Vec<Skill>> {
    let library = open_skill_library().await?;
    load_prompt_skills_with_library(
        workspace,
        session_id,
        vertical_id,
        library.as_ref(),
        worker_id,
    )
    .await
}

pub(super) async fn load_prompt_skills_with_library(
    workspace: &SkillStore,
    session_id: SessionId,
    vertical_id: Option<&str>,
    library: Option<&SkillStore>,
    worker_id: Option<WorkerId>,
) -> ArgoResult<Vec<Skill>> {
    let attached = workspace.skills_for_session(session_id).await?;
    if !attached.is_empty() {
        return Ok(attached);
    }

    let Some(library) = library else {
        return match vertical_id {
            Some(vid) => workspace.list_skills_for_vertical(vid).await,
            None => merge_worker_overlays(workspace, workspace.list_active().await?, worker_id)
                .await,
        };
    };

    let lookup = LayeredLookup::with_library(workspace, library);
    let mut rows = lookup.list_for_action("low").await?;
    append_worker_overlays_as_workspace(workspace, &mut rows, worker_id).await?;
    let selector = SkillRelevanceSelector::new(load_skill_relevance_config()?);
    let vertical_tag;
    let task_tags: &[&str] = match vertical_id {
        Some(vid) => {
            vertical_tag = [vid];
            &vertical_tag
        }
        None => &[],
    };
    Ok(selector.select(
        rows,
        SkillSelectionContext {
            action_type: None,
            vertical_id,
            task_tags,
        },
    ))
}

pub(super) fn render_skill_prompt(active: Vec<Skill>) -> String {
    let mut out = String::new();
    if active.is_empty() {
        return out;
    }
    out.push_str(
        "You have access to the following workspace skills. \
         Apply them as guidance for tool selection, response style, \
         and domain knowledge.\n\n",
    );
    for s in active {
        out.push_str(&format!(
            "## {} (v{})\n",
            s.frontmatter.name, s.frontmatter.version
        ));
        if !s.frontmatter.description.is_empty() {
            out.push_str(&s.frontmatter.description);
            out.push('\n');
        }
        out.push('\n');
        out.push_str(s.body.trim_end());
        out.push_str("\n\n---\n\n");
    }
    out
}

async fn open_skill_library() -> ArgoResult<Option<SkillStore>> {
    let Some(path) = std::env::var_os("ARGO_SKILL_LIBRARY_DB") else {
        return Ok(None);
    };
    let store = SessionStore::open(PathBuf::from(path)).await?;
    store.migrate().await?;
    Ok(Some(SkillStore::new(Arc::new(store))))
}

fn load_skill_relevance_config() -> ArgoResult<SkillRelevanceConfig> {
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Io(format!("cwd: {e}")))?;
    for path in [cwd.join(".skycot/argo.toml"), cwd.join("argo.toml")] {
        if !path.exists() {
            continue;
        }
        let cfg = argo_config::ArgoConfig::load(&path)?
            .skill_library
            .relevance;
        return Ok(SkillRelevanceConfig::from_operator_config(
            cfg.max_marketplace_skills,
            &cfg.min_trust_grade,
        ));
    }
    Ok(SkillRelevanceConfig::default())
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn run_turn_with_tools(
    store: &Arc<SessionStore>,
    transport: &dyn Transport,
    cwd: &std::path::Path,
    session_id: SessionId,
    model: &str,
    audit: &AuditSink,
    spend: &SpendCtx<'_>,
    max_rounds: usize,
    stream: bool,
    events: Option<&RunEventSink>,
    vertical_prompt: Option<&str>,
    vertical_scope: Option<&str>,
    vertical_deny: &std::collections::HashSet<String>,
    vertical_approval: &std::collections::HashSet<String>,
    session_grants: SessionGrantState,
    worker_id: Option<argo_store::WorkerId>,
    invocation_id: &str,
    pending_ab_outcomes: &mut Vec<PendingAbOutcome>,
) -> ArgoResult<TurnResult> {
    let mut session_grants = session_grants;
    let ctx = build_turn_dispatch_ctx(
        store,
        cwd,
        session_id,
        model,
        spend,
        vertical_prompt,
        vertical_scope,
        vertical_deny,
        worker_id,
        invocation_id,
        pending_ab_outcomes,
    )
    .await?;
    let mut last_outcome = None;
    for round in 0..max_rounds {
        let req = build_round_request(store, &ctx, audit, session_id, model).await?;
        let outcome = run_one_round(
            store, transport, session_id, audit, spend, events, req, stream, round,
        )
        .await?;
        if outcome.tool_calls.is_empty() {
            return Ok(finalize_turn(outcome));
        }
        for call in &outcome.tool_calls {
            dispatch_one_tool_call(
                &ctx,
                store,
                cwd,
                session_id,
                audit,
                spend,
                events,
                vertical_deny,
                vertical_approval,
                &mut session_grants,
                call,
            )
            .await?;
        }
        last_outcome = Some(outcome);
    }
    // #724 — when we hit `max_rounds` the for-loop ends naturally;
    // we DON'T return Err. Every dispatched tool just got its
    // `tool_result` appended, so the caller can issue another
    // `run_turn` to continue. The `stopped_on_tool_budget` flag and
    // the `Halt` audit hint let the outer worker step loop convert
    // this into a clean step boundary rather than treating it as a
    // transport error.
    emit_halt(session_id, audit, events, max_rounds).await?;
    Ok(budget_exhausted_result(last_outcome))
}

/// Assemble the [`ProviderRequest`] for one round — fetches the
/// session history, applies context compression, then snapshots the
/// tools / system prompt from `ctx`.
async fn build_round_request(
    store: &Arc<SessionStore>,
    ctx: &super::turn_helpers_run::TurnDispatchCtx,
    audit: &AuditSink,
    session_id: SessionId,
    model: &str,
) -> ArgoResult<ProviderRequest> {
    let rows = store.list_messages(session_id).await?;
    let messages = rows
        .iter()
        .map(|r| message_from_row(r, session_id))
        .collect::<ArgoResult<Vec<_>>>()?;
    let messages = prepare_context_messages(messages, &ctx.context_policy, audit, session_id).await?;
    Ok(ProviderRequest {
        model: model.to_string(),
        messages,
        tools: ctx.tools.clone(),
        system: ctx.system_prompt.clone(),
        sampling: Sampling::default(),
    })
}

/// Build the budget-exhausted [`TurnResult`] from the most recent
/// completed round's outcome. Falls back to a zero-token "nothing
/// produced" result if `max_rounds == 0` (no rounds ran).
fn budget_exhausted_result(last: Option<super::turn_helpers_run::RoundOutcome>) -> TurnResult {
    match last {
        Some(o) => TurnResult {
            assistant_id: o.assistant_id,
            text: o.text,
            tokens_in: o.tokens_in,
            tokens_out: o.tokens_out,
            stopped_on_tool_budget: true,
        },
        None => TurnResult {
            assistant_id: uuid::Uuid::nil(),
            text: String::new(),
            tokens_in: None,
            tokens_out: None,
            stopped_on_tool_budget: true,
        },
    }
}
