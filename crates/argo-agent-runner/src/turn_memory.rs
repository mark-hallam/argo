//! mission-wire-memory-compose-context (Phase B): hydrate the
//! workspace-shared and worker memory namespaces into the agent
//! system prompt.
//!
//! Loaded via `#[path = "turn_memory.rs"] mod turn_memory;` from
//! `turn.rs` — kept in a sibling file because `turn.rs` is near
//! the 500-LOC ceiling. Soft-fails on store error: a
//! `tracing::warn!` is emitted and an empty block returned so
//! memory never blocks conversation flow.

use argo_store::SessionStore;
use argo_types::TenantId;
use std::sync::Arc;

const MEMORY_LAYERS_ENV: &str = "ARGO_MEMORY_LAYERS";
const SYSTEM_PROMPT_ACTION: &str = "system_prompt";

#[derive(Clone, Copy)]
pub(crate) enum PromptMemoryMode {
    Auto,
    Shared,
    Worker,
    Both,
}

impl PromptMemoryMode {
    fn from_env() -> Self {
        match std::env::var(MEMORY_LAYERS_ENV)
            .unwrap_or_default()
            .to_ascii_lowercase()
            .as_str()
        {
            "shared" => Self::Shared,
            "worker" => Self::Worker,
            "both" => Self::Both,
            _ => Self::Auto,
        }
    }
}

pub(crate) async fn compose_prompt_memory(
    store: &Arc<SessionStore>,
    tenant_id: TenantId,
    worker_id: Option<argo_store::WorkerId>,
) -> String {
    compose_prompt_memory_for_mode(store, tenant_id, worker_id, PromptMemoryMode::from_env()).await
}

pub(crate) async fn compose_prompt_memory_for_mode(
    store: &Arc<SessionStore>,
    tenant_id: TenantId,
    worker_id: Option<argo_store::WorkerId>,
    mode: PromptMemoryMode,
) -> String {
    match mode {
        PromptMemoryMode::Shared => compose_shared_team_memory(store, tenant_id).await,
        PromptMemoryMode::Worker => compose_worker_memory(store, tenant_id, worker_id).await,
        PromptMemoryMode::Both => {
            let worker = compose_worker_memory(store, tenant_id, worker_id).await;
            let shared = compose_shared_team_memory(store, tenant_id).await;
            join_memory_blocks(worker, shared)
        }
        PromptMemoryMode::Auto => {
            let worker = compose_worker_memory(store, tenant_id, worker_id).await;
            if worker.is_empty() {
                compose_shared_team_memory(store, tenant_id).await
            } else {
                worker
            }
        }
    }
}

async fn compose_worker_memory(
    store: &Arc<SessionStore>,
    tenant_id: TenantId,
    worker_id: Option<argo_store::WorkerId>,
) -> String {
    let Some(worker_id) = worker_id else {
        return String::new();
    };
    let memory = argo_memory::MemoryStore::new(Arc::clone(store));
    let ns_worker = argo_identity::WorkerId(worker_id.0);
    let ns = argo_identity::MemoryNs::new(
        tenant_id,
        ns_worker,
        argo_identity::MemoryScope::WorkerSelf,
    );
    match memory.compose_context(&ns, SYSTEM_PROMPT_ACTION).await {
        Ok(pack) => render_worker_memory(&pack.text),
        Err(e) => {
            tracing::warn!(
                error = %e,
                tenant = %tenant_id.0,
                worker = %worker_id.0,
                "memory compose_context failed; skipping worker memory block"
            );
            String::new()
        }
    }
}

/// Fetch shared-team memory rows for `tenant_id` and render them
/// via `MemoryNs::compose_context`. Returns the markdown block,
/// or empty string when there are no rows or the store is
/// unreachable.
async fn compose_shared_team_memory(
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

fn render_worker_memory(text: &str) -> String {
    let text = text.trim();
    if text.is_empty() {
        String::new()
    } else {
        format!("## Memory (worker-scoped)\n{text}\n")
    }
}

fn join_memory_blocks(first: String, second: String) -> String {
    match (first.is_empty(), second.is_empty()) {
        (true, true) => String::new(),
        (false, true) => first,
        (true, false) => second,
        (false, false) => format!("{}\n{}", first.trim_end(), second),
    }
}
