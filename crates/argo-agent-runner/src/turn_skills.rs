//! mission-skill-session-boot-wire — render the trust-gated
//! "Available skills" block for the session's worker and inject it
//! into the system prompt at boot time.
//!
//! Loaded via `#[path = "turn_skills.rs"] mod turn_skills;` from
//! `turn.rs` (mirrors the `turn_memory.rs` / `turn_skills_runtime.rs`
//! sibling pattern). Keeps the lookup + render out of `turn.rs` so
//! that file stays under the 500-LOC HARD RULE 6 ceiling.
//!
//! The block lands AFTER the persona / vertical-prompt and the
//! workspace-skill bodies but BEFORE the memory composition appended
//! by [`crate::turn_memory::compose_prompt_memory`]. Empty list ⇒
//! the entire block is omitted (don't pollute the prompt with
//! negative context). Lookup error ⇒ `tracing::warn!` + omit; session
//! boot never fails because the skill store is unreachable or a
//! worker row is missing.
//!
//! Trait wiring: `argo-skill` defines [`WorkerTrustReader`] and does
//! NOT depend on `argo-identity` (the dep arrow is
//! `argo-identity → argo-store`; inverting it would create a cycle).
//! A sibling adapter [`IdentityWorkerTrustReader`] already exists in
//! `argo-cli/src/handlers/worker_trust_reader.rs` but `argo-cli` is
//! downstream of `argo-agent-runner`, so this module repeats the
//! newtype rather than importing it.
use argo_identity::IdentityStore;
use argo_skill::{Skill, SkillStore, WorkerTrustReader};
use argo_store::SessionStore;
use argo_types::{ArgoResult, TenantId};
use async_trait::async_trait;
use std::sync::Arc;
use uuid::Uuid;

/// Render the trust-gated skill list as the markdown "Available
/// skills" block injected at session boot. Returns `None` when the
/// list is empty so the caller can omit the block entirely instead
/// of emitting "No skills available." (negative context pollutes the
/// prompt).
///
/// Format is load-bearing: one of the brief's six unit tests pins
/// exact bytes. Tweak only with a documented prompt-engineering
/// reason.
pub(crate) fn render_available_skills_block(skills: &[Skill]) -> Option<String> {
    if skills.is_empty() {
        return None;
    }
    let mut out = String::from("## Available skills\n");
    for s in skills {
        let desc = s.frontmatter.description.trim();
        if desc.is_empty() {
            out.push_str(&format!(
                "- **{}** (v{})\n",
                s.frontmatter.name, s.frontmatter.version
            ));
        } else {
            out.push_str(&format!(
                "- **{}** (v{}): {}\n",
                s.frontmatter.name, s.frontmatter.version, desc
            ));
        }
    }
    Some(out)
}

/// Append the rendered "Available skills" block onto the prompt
/// buffer with a clean blank-line separator from whatever preceded
/// it (workspace-skill bodies, or the vertical persona if no skills
/// were loaded). Callers append the memory block AFTER this call, so
/// the final ordering is `persona → workspace-skill bodies →
/// available skills → memory`.
pub(crate) fn inject_skills_block(buf: &mut String, block: &str) {
    if buf.is_empty() {
        buf.push_str(block);
        return;
    }
    if !buf.ends_with("\n\n") {
        if buf.ends_with('\n') {
            buf.push('\n');
        } else {
            buf.push_str("\n\n");
        }
    }
    buf.push_str(block);
}

/// Resolve the worker's trust grade, call
/// [`SkillStore::available_for_worker`], and render the block. Used
/// by `build_system_prompt_with_ab` once per turn (so once per
/// session boot — there is no separate boot vs. per-turn prompt
/// assembly in this runtime). Errors degrade to `None` with a
/// `tracing::warn!` so a missing worker row or unreachable skill
/// store never crashes session boot.
pub(crate) async fn compose_available_skills_block(
    store: &Arc<SessionStore>,
    tenant_id: TenantId,
    worker_id: Option<argo_store::WorkerId>,
) -> Option<String> {
    let worker_id = worker_id?.0;
    let identity = IdentityStore::new(store);
    let reader = IdentityTrustReader { identity: &identity };
    let skills_store = SkillStore::new(Arc::clone(store));
    match skills_store
        .available_for_worker(&reader, tenant_id, worker_id)
        .await
    {
        Ok(skills) => render_available_skills_block(&skills),
        Err(e) => {
            tracing::warn!(
                error = %e,
                tenant_id = %tenant_id.0,
                worker_id = %worker_id,
                "available_for_worker lookup failed; omitting Available skills block",
            );
            None
        }
    }
}

/// Newtype adapter mirroring `argo-cli`'s
/// `IdentityWorkerTrustReader` — re-defined here because `argo-cli`
/// is downstream of `argo-agent-runner`. Translates the trait's bare
/// `Uuid` back into the [`WorkerId`] newtype the store API takes.
struct IdentityTrustReader<'a, 's> {
    identity: &'a IdentityStore<'s>,
}

#[async_trait]
impl WorkerTrustReader for IdentityTrustReader<'_, '_> {
    async fn trust_grade(
        &self,
        tenant_id: TenantId,
        worker_id: Uuid,
    ) -> ArgoResult<Option<u8>> {
        let worker = self
            .identity
            .get_worker(tenant_id, argo_identity::WorkerId(worker_id))
            .await?;
        Ok(worker.map(|w| w.trust_grade))
    }
}
