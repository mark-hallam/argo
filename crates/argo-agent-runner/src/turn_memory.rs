//! mission-wire-memory-compose-context (Phase B): hydrate the
//! workspace-shared memory namespace into the agent system prompt.
//!
//! Loaded via `#[path = "turn_memory.rs"] mod turn_memory;` from
//! `turn.rs` — kept in a sibling file because `turn.rs` is near
//! the 500-LOC ceiling. Soft-fails on store error: a
//! `tracing::warn!` is emitted and an empty block returned so
//! memory never blocks conversation flow. Per-worker scope arrives
//! in the follow-up brief once `session.worker_id` lands.

use argo_store::SessionStore;
use argo_types::TenantId;
use std::sync::Arc;

/// Fetch shared-team memory rows for `tenant_id` and render them
/// via `MemoryNs::compose_context`. Returns the markdown block,
/// or empty string when there are no rows or the store is
/// unreachable.
pub(crate) async fn compose_shared_team_memory(
    store: &Arc<SessionStore>,
    tenant_id: TenantId,
) -> String {
    let memory = argo_memory::MemoryStore::new(Arc::clone(store));
    match memory
        .read_shared_team(&argo_memory::shared_team(tenant_id))
        .await
    {
        Ok(entries) => {
            let rows: Vec<_> = entries
                .into_iter()
                .map(|e| (e.written_at, e.content))
                .collect();
            argo_identity::MemoryNs::new(
                tenant_id,
                argo_identity::WorkerId::nil(),
                argo_identity::MemoryScope::SharedTeam,
            )
            .compose_context(&rows)
        }
        Err(e) => {
            tracing::warn!(
                error = %e,
                tenant = %tenant_id.0,
                "memory read_shared_team failed; skipping memory block"
            );
            String::new()
        }
    }
}
