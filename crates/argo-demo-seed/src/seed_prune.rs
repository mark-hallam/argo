//! Record-readiness prune for the canonical v1 demo tenant.
//!
//! The canonical seed is UPSERT — re-running it (and interactive demo
//! sessions / verify-test runs) leaves test artifacts behind: stray
//! Genesis-hired workers (e.g. "Maya"), workers from earlier roster
//! iterations, and the like. This module prunes the demo tenant down to
//! EXACTLY the canonical roster so a freshly-seeded tenant is camera-clean.
//!
//! **CRITICAL SAFETY — demo-tenant-only, canonical-protected, additive:**
//!
//! * **Called from one place only.** [`prune_demo_tenant_to_canonical`] is
//!   invoked solely from [`crate::seed::run_canonical_v1_seed`] (the demo
//!   path). The general [`crate::seed::seed_workers`] path and every
//!   production code path never call it.
//! * **Scoped to the passed tenant.** Every read + write is keyed by the
//!   `tenant_id` argument; the prune can never touch another tenant's rows.
//! * **Canonical rows are never deleted.** A worker is pruned ONLY when its
//!   `persona.name` is **not** in `canonical_names`. The canonical eight
//!   are matched by name and always survive.
//! * **Fail-closed on an empty canonical set.** If `canonical_names` is
//!   empty the prune refuses to run (returns `0`) — a belt-and-braces guard
//!   so a future miswire can never turn this into "delete everything".
//! * **Reversible, not destructive.** Pruning uses
//!   [`argo_store::worker_archive::archive_worker`], which snapshots the
//!   full worker into `worker_archive` and removes the live `worker` /
//!   `persona` / `role` / `duty` rows in one transaction. Audit + handoff
//!   lineage is deliberately left intact. Nothing is hard-deleted here.

use std::collections::HashSet;
use std::sync::Arc;

use argo_identity::IdentityStore;
use argo_store::{worker_archive, SessionStore};
use argo_types::TenantId;

use crate::SeedResult;

/// Archive every live worker in `tenant_id` whose `persona.name` is not in
/// `canonical_names`, leaving the tenant with exactly the canonical roster
/// live. Returns the number of workers pruned (archived). Idempotent: a
/// second run finds no strays and returns `0`.
///
/// See the module docs for the full safety contract (demo-tenant-only,
/// canonical-protected, fail-closed on an empty set, reversible).
pub(crate) async fn prune_demo_tenant_to_canonical(
    store: &Arc<SessionStore>,
    tenant_id: TenantId,
    canonical_names: &[&str],
) -> SeedResult<usize> {
    // Fail-closed: never prune against an empty canonical set (would
    // otherwise archive the entire tenant).
    if canonical_names.is_empty() {
        return Ok(0);
    }
    let keep: HashSet<&str> = canonical_names.iter().copied().collect();

    let identities = IdentityStore::new(store);
    // Live (non-archived) workers only — already-archived strays are gone
    // from the live set and need no action.
    let live = identities.list_workers(tenant_id, false).await?;

    let pool = store.pool();
    let tenant_str = tenant_id.to_string();
    let mut pruned = 0usize;
    for worker in &live {
        if keep.contains(worker.persona.name.as_str()) {
            continue; // canonical — never touched
        }
        // Snapshot the full worker so the prune is reversible (rehydrate).
        let frozen = serde_json::to_string(worker)
            .map_err(|e| argo_types::ArgoError::Schema(format!("prune freeze worker: {e}")))?;
        worker_archive::archive_worker(
            pool,
            &worker.id.to_string(),
            &tenant_str,
            Some("argo-demo-seed"),
            Some("canonical-prune: non-roster worker removed for record-readiness"),
            &frozen,
        )
        .await?;
        pruned += 1;
    }
    Ok(pruned)
}

#[cfg(test)]
#[path = "seed_prune_tests.rs"]
mod tests;
