//! Supporting plumbing extracted from
//! [`super::turn_helpers_run::build_turn_dispatch_ctx`] to keep that
//! module under the 500-LOC file budget. Two self-contained helpers:
//! the per-`(tenant, session)` [`argo_tool::ProcessRegistry`] cache and
//! the inbound-handoff resolver. Pure relocation — no behavior change.
//!
//! Every symbol is `pub(super)` and only consumed by
//! `turn_helpers_run`.

use argo_orchestrator::{EscalationStep, HandoffId, HandoffStatus};
use argo_types::{ArgoResult, SessionId, TenantId};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

type ProcessRegistryKey = (TenantId, SessionId);
type ProcessRegistryCache = HashMap<ProcessRegistryKey, Arc<argo_tool::ProcessRegistry>>;

fn process_registry_cache() -> &'static Mutex<ProcessRegistryCache> {
    static CACHE: OnceLock<Mutex<ProcessRegistryCache>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Return the shared [`argo_tool::ProcessRegistry`] for a
/// `(tenant_id, session_id)` pair, creating it on first use. Caching
/// keeps a session's background processes addressable across rounds
/// without leaking them between tenants or sessions.
pub(super) fn process_registry_for(
    tenant_id: TenantId,
    session_id: SessionId,
) -> Arc<argo_tool::ProcessRegistry> {
    let mut cache = process_registry_cache()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    cache
        .entry((tenant_id, session_id))
        .or_insert_with(|| Arc::new(argo_tool::ProcessRegistry::new()))
        .clone()
}

/// Find the active inbound handoff (if any) assigned to `worker_id`
/// for this session, scanning most-recent-first. Returns `None` when
/// the session carries no worker or has no pending/acknowledged
/// handoff targeting it.
pub(super) async fn current_inbound_handoff_id(
    orchestrator: &argo_orchestrator::OrchestratorStore,
    tenant_id: TenantId,
    session_id: &argo_orchestrator::SessionId,
    worker_id: Option<argo_store::WorkerId>,
) -> ArgoResult<Option<HandoffId>> {
    let Some(worker_id) = worker_id else {
        return Ok(None);
    };
    let handoffs = orchestrator
        .list_handoffs_for_session(tenant_id, session_id)
        .await?;
    Ok(handoffs.into_iter().rev().find_map(|handoff| {
        let active = matches!(
            handoff.status,
            HandoffStatus::Pending | HandoffStatus::Acknowledged
        );
        let assigned_to_worker =
            matches!(&handoff.to, EscalationStep::Worker(id) if id == &worker_id);
        (active && assigned_to_worker).then_some(handoff.id)
    }))
}
