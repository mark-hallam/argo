//! Per-worker memory seeders — real memory facts + open conversation
//! threads for the canonical v1 demo hero workers (Frontier-B depth,
//! worker-memory surfaces).
//!
//! Sibling to `seed_worker_overlays.rs` / `seed_retrain_records`, same
//! idempotent pattern. Both entry points are called from
//! `crate::seed::seed_enrichment_surfaces` after the worker rows exist.
//!
//! **NO THEATER — real data on the real read path.**
//!
//! * Facts are written via the EXACT primitive the live gateway uses —
//!   [`argo_memory::MemoryStore::append_in_namespace`] into the worker's
//!   own `memory_namespace` (`MemoryFile::Memory`, scope
//!   `MemoryScope::WorkerSelf`) — the same call
//!   `routes_worker_memory::fact_create_response` makes. The
//!   `GET /api/v1/workers/<id>/memory` read (`fetch_active_facts`) keys on
//!   `(tenant_id, worker_id, memory_scope=worker_self, file=memory)`, which
//!   is precisely what this writes, so the seeded facts surface on the live
//!   Memory tab. No parallel surface.
//!
//! * Conversation threads are written via the first-class
//!   [`argo_store::conversation_thread`] store (migration v87) the gateway
//!   read consumes (`fetch_active_conversations`). Tenant + worker-scoped.
//!
//! Facts + threads are persona-tied and echo seeded events (e.g. Mariah's
//! Northwind NET-30 term, Priya's Diego-Alvarez Thursday interview), so a
//! reviewer who drills in sees a coherent story, not lorem ipsum.

use std::sync::Arc;

use argo_identity::{MemoryNs, MemoryScope};
use argo_memory::{MemoryFile, MemoryStore};
use argo_store::conversation_thread::{count_for_worker, insert, ConversationThreadInsert};
use argo_store::SessionStore;
use argo_types::{TenantId, WorkspaceId};
use chrono::Utc;
use uuid::Uuid;

use crate::personas::DEMO_SEED_PERSONAS;
use crate::SeedResult;

/// Sentinel `root_path` for the demo-internal workspace row that
/// `memory_entry.workspace_id` (an FK to `workspace(id)`) anchors to. The
/// `workspace_id` column is NOT read back by the Memory-tab query — that
/// keys on `(tenant_id, worker_id, scope, file)` — so a stable internal
/// workspace satisfies the FK without affecting what the operator sees.
/// `ensure_workspace` is idempotent by `root_path`, so re-seeds reuse it.
const SEED_MEMORY_WORKSPACE_ROOT: &str = "argo-demo-seed::worker-memory";

/// Persona-tied facts, keyed by display name. Each fact is a real,
/// plausible thing that worker would have learned — echoing the seeded
/// approvals / duties so the Memory tab tells one coherent story.
fn facts_for(persona_name: &str) -> &'static [&'static str] {
    match persona_name {
        "Mariah" => &[
            "Northwind pays NET-30, not NET-15 — bill them on the longer term.",
            "Listing photos for the CCM helmet line convert best shot on a white background.",
            "Repost stale listings on Tuesday mornings — that is when buyer traffic peaks.",
        ],
        "Priya" => &[
            "Diego Alvarez prefers Thursday interviews — book his slots Thursday where possible.",
            "Always confirm work-authorisation status before forwarding a candidate to the panel.",
            "The hiring manager for the platform role wants a 45-minute screen, not 30.",
        ],
        "Peta" => &[
            "The Q2 launch milestone is the hard deadline — everything else flexes around it.",
            "Devon is out Fridays; route Friday blockers to the on-call IC instead.",
        ],
        "Jamal" => &[
            "Refunds over $500 always need the account owner CC'd before they go out.",
            "Tier-1 may not promise SLA credits — escalate those to a human.",
        ],
        "Nori" => &[
            "Product announcements ship Tuesday 09:00 PST — never Friday afternoon.",
            "The brand voice is warm and plain — no exclamation marks in customer comms.",
        ],
        "Portia" => &[
            "Only quote metrics from the approved investor fact-sheet — never improvise a number.",
            "If asked something outside the approved facts, say so and offer to follow up.",
        ],
        "Grant" => &[
            "Enterprise signups from a .edu or .gov domain route to the named-account team.",
            "Never send more than one outreach follow-up without a reply.",
        ],
        "Marcus" => &[
            "Every reply is draft-only — Mark approves before anything posts.",
            "Lead with the idea, not the product — the moat is the thinking.",
        ],
        _ => &[],
    }
}

/// One seeded conversation thread for a persona. `(counterparty, topic,
/// minutes_ago)`. Empty for personas with no natural open thread (they
/// honestly show "No open conversations right now.").
fn threads_for(persona_name: &str) -> &'static [(&'static str, &'static str, i64)] {
    match persona_name {
        "Mariah" => &[
            ("Northwind Trading", "Re-price the CCM helmet lot for the NET-30 renewal", 24),
            ("buyer: m_alvarez", "Counter-offer on listing #3120 — holding at $215", 140),
        ],
        "Priya" => &[
            ("Diego Alvarez", "Interview scheduling — Thursday 10:30 AM NZST", 18),
            ("hiring manager", "Platform-role screen length — confirming 45 min", 320),
        ],
        "Jamal" => &[("customer #18293", "Refund request — over $500, pending owner CC", 52)],
        "Nori" => &[("comms review", "Q2 audit-log-export announcement — final copy", 95)],
        _ => &[],
    }
}

/// Seed both worker-memory surfaces — real facts + open conversation
/// threads — onto `report`, keeping `seed.rs` to a single call. Each piece
/// is independently idempotent. See [`seed_worker_memory_facts`] +
/// [`seed_worker_conversation_threads`].
pub(crate) async fn seed_worker_memory(
    store: &Arc<SessionStore>,
    tenant_id: TenantId,
    worker_ids: &[argo_identity::WorkerId],
    report: &mut crate::SeedReport,
) -> SeedResult<()> {
    report.worker_memory_facts_seeded =
        seed_worker_memory_facts(store, tenant_id, worker_ids).await?;
    report.worker_conversation_threads_seeded =
        seed_worker_conversation_threads(store, tenant_id, worker_ids).await?;
    Ok(())
}

/// Seed 2-4 real memory facts per hero worker via the live memory write
/// path. Idempotent: if the worker already has any active fact under its
/// `WorkerSelf` namespace, the worker is skipped (re-seeding to add a
/// teammate must not duplicate the panel). Returns the number of facts
/// inserted (`0` on a full re-seed).
///
/// `worker_ids` is positional-aligned with [`DEMO_SEED_PERSONAS`] — the
/// same spine `seed_demo_workers` walked.
pub(crate) async fn seed_worker_memory_facts(
    store: &Arc<SessionStore>,
    tenant_id: TenantId,
    worker_ids: &[argo_identity::WorkerId],
) -> SeedResult<usize> {
    if worker_ids.is_empty() {
        return Ok(0);
    }
    let workspace = ensure_seed_workspace(store).await?;
    let memory = MemoryStore::new(Arc::clone(store));
    let mut inserted = 0usize;
    for (persona, &worker_id) in DEMO_SEED_PERSONAS.iter().zip(worker_ids.iter()) {
        let facts = facts_for(persona.name);
        if facts.is_empty() {
            continue;
        }
        // Build the worker's own namespace exactly as the live worker row
        // carries it: tenant + worker id, scope WorkerSelf.
        let ns = MemoryNs::new(tenant_id, worker_id, MemoryScope::WorkerSelf);
        if fact_count(store, &ns).await? > 0 {
            continue; // already seeded for this worker
        }
        for fact in facts {
            memory
                .append_in_namespace(&ns, workspace, MemoryFile::Memory, (*fact).to_string(), None)
                .await?;
            inserted += 1;
        }
    }
    Ok(inserted)
}

/// Seed persona-tied open conversation threads for the hero workers via
/// the first-class `worker_conversation_thread` store (migration v87).
/// Idempotent per worker (skip if any thread exists). Returns the number
/// of threads inserted.
pub(crate) async fn seed_worker_conversation_threads(
    store: &Arc<SessionStore>,
    tenant_id: TenantId,
    worker_ids: &[argo_identity::WorkerId],
) -> SeedResult<usize> {
    if worker_ids.is_empty() {
        return Ok(0);
    }
    let pool = store.pool();
    let tenant = tenant_id.to_string();
    let now = Utc::now();
    let mut inserted = 0usize;
    for (persona, &worker_id) in DEMO_SEED_PERSONAS.iter().zip(worker_ids.iter()) {
        let threads = threads_for(persona.name);
        if threads.is_empty() {
            continue;
        }
        let worker_blob = worker_id.0.as_bytes().to_vec();
        if count_for_worker(pool, &tenant, &worker_blob).await? > 0 {
            continue; // already seeded for this worker
        }
        for (counterparty, topic, minutes_ago) in threads {
            let id = format!("ct_{}", Uuid::now_v7());
            let last_message_at =
                (now - chrono::Duration::minutes(*minutes_ago)).timestamp_micros();
            insert(
                pool,
                &ConversationThreadInsert {
                    id: &id,
                    tenant_id: &tenant,
                    worker_id: &worker_blob,
                    counterparty_handle: counterparty,
                    topic,
                    last_message_at,
                    // The thread links to the worker's audit feed; the
                    // per-action drill-in is resolved by the SPA when a
                    // matching action exists, else the feed root.
                    audit_href: Some("/audit"),
                },
            )
            .await?;
            inserted += 1;
        }
    }
    Ok(inserted)
}

/// Active-fact count under a worker's `WorkerSelf` namespace — the same
/// predicate `fetch_active_facts` reads (minus the tombstone filter, which
/// the seed never writes). Used for the per-worker idempotency guard.
async fn fact_count(store: &Arc<SessionStore>, ns: &MemoryNs) -> SeedResult<i64> {
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM memory_entry \
         WHERE tenant_id = ? AND worker_id = ? AND memory_scope = 'worker_self' \
           AND file = ? AND superseded_by IS NULL",
    )
    .bind(ns.tenant_id.0.as_bytes().as_slice())
    .bind(ns.worker_id.0.as_bytes().as_slice())
    .bind(MemoryFile::Memory.filename())
    .fetch_one(store.pool())
    .await
    .map_err(|e| argo_types::ArgoError::Provider(format!("memory fact-count probe: {e}")))?;
    Ok(count)
}

/// Ensure the demo-internal workspace row that `memory_entry.workspace_id`
/// (an FK) anchors to exists, returning its id. Idempotent by `root_path`.
async fn ensure_seed_workspace(store: &Arc<SessionStore>) -> SeedResult<WorkspaceId> {
    let (_tenant, workspace) = store
        .ensure_workspace("argo-demo-seed worker-memory", SEED_MEMORY_WORKSPACE_ROOT)
        .await?;
    Ok(workspace)
}

#[cfg(test)]
#[path = "seed_worker_memory_tests.rs"]
mod tests;
