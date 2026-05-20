//! System-prompt composition extracted from [`crate::turn`] so the
//! parent file stays under the 500-LOC ceiling (HARD RULE 6).
//!
//! Resolution order (see [`build_system_prompt`]):
//! 1. If `session_skill` rows exist for `session_id`, use *only*
//!    those skills (per-session curation, #279).
//! 2. Otherwise, when `vertical_id` is bound (#679 — Rule 6),
//!    fall back to skills scoped to that vertical
//!    (`SkillStore::list_skills_for_vertical`). Cross-vertical
//!    skill bleed-through is blocked at this layer.
//! 3. Otherwise (unbound legacy session), fall back to every
//!    active row in the workspace.

use argo_types::{ArgoResult, SessionId, TenantId};
use argo_store::SessionStore;
use std::sync::Arc;

use super::turn_canary;
use super::turn_helpers;
use super::turn_memory;
use super::turn_skills;
use super::turn_skills_migration;
use super::turn_skills_runtime;

/// Concatenate skill bodies into a single system-prompt string
/// that's passed on every round of the turn. Returns `None` when
/// nothing is in scope (the provider transport then sends no
/// `system` block at all).
#[cfg(test)]
pub(crate) async fn build_system_prompt(
    store: &Arc<SessionStore>,
    session_id: SessionId,
    vertical_id: Option<&str>,
    tenant_id: TenantId,
    worker_id: Option<argo_store::WorkerId>,
) -> ArgoResult<Option<String>> {
    let (prompt, _pending) =
        build_system_prompt_with_ab(store, session_id, vertical_id, tenant_id, worker_id, "")
            .await?;
    Ok(prompt)
}

/// Compose the system prompt AND resolve A/B variants in one pass.
///
/// Behaves like [`build_system_prompt`] but additionally returns the
/// pending A/B outcomes that the caller must record after the turn
/// settles. `invocation_id` is the SHA-256 seed for
/// `argo_skill::select_variant`; pass an empty string when the
/// caller does NOT care about A/B selection (the variant choice
/// falls back to "keep all rows" because no run can match).
pub(crate) async fn build_system_prompt_with_ab(
    store: &Arc<SessionStore>,
    session_id: SessionId,
    vertical_id: Option<&str>,
    tenant_id: TenantId,
    worker_id: Option<argo_store::WorkerId>,
    invocation_id: &str,
) -> ArgoResult<(Option<String>, Vec<turn_skills_runtime::PendingAbOutcome>)> {
    let skills = argo_skill::SkillStore::new(Arc::clone(store));
    let mut active =
        turn_helpers::load_prompt_skills(&skills, session_id, vertical_id, worker_id).await?;
    turn_canary::apply_canary_routing(store, &skills, session_id, tenant_id, &mut active).await?;
    let (active, pending) =
        turn_skills_runtime::resolve_variants_for_turn(store, invocation_id, active).await;
    turn_skills_migration::emit_pending_migration_hints(store, &active).await;
    let mut out = turn_helpers::render_skill_prompt(active);
    // mission-skill-session-boot-wire: render the trust-gated
    // "Available skills" block (name + version + description only,
    // no body) once per session boot, between the active workspace
    // skills and the memory composition. Empty list / lookup error
    // → omit; session boot never fails.
    if let Some(block) =
        turn_skills::compose_available_skills_block(store, tenant_id, worker_id).await
    {
        turn_skills::inject_skills_block(&mut out, &block);
    }
    let memory = turn_memory::compose_prompt_memory(store, tenant_id, worker_id).await;
    if !memory.is_empty() {
        if !out.is_empty() {
            out.push('\n');
        }
        out.push_str(&memory);
    }
    let prompt = if out.is_empty() { None } else { Some(out) };
    Ok((prompt, pending))
}
