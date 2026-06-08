//! Hire → scheduler duty bridge (UF Phase-B · B1).
//!
//! ## Why this exists
//! The hire path persists a worker's cadence into the identity layer's
//! `duty` rows (via `IdentityStore::create_worker_from_draft`, which
//! writes the `worker`/`worker_persona` role JSON and the legacy `duty`
//! table). The autonomous **scheduler**, however, reads a *different*
//! table — `role_duty` — through [`argo_orchestrator::DutyStore`]
//! (`crates/argo-orchestrator/src/duty_scheduler_store.rs`). With no
//! bridge, a freshly-hired teammate's schedule is invisible to the
//! scheduler. This module closes that gap: after the worker commits, we
//! mirror its enabled duties into `role_duty` using the scheduler's own
//! writer ([`DutyStore::insert`]) — never a parallel writer, per the
//! brief's grep-before-write note.
//!
//! ## Default-OFF posture (DEC-036 / DEC-040)
//! Writing a `role_duty` row is **safe plumbing only**. It makes the
//! cadence *listable* by the scheduler; it does not arm autonomous
//! execution. Whether a listed duty actually fires is gated downstream
//! by B2's `ARGO_DUTY_AUTOFIRE` fire-gate, which stays default-closed.
//! This bridge therefore lands dormant: it never enables anything.
//!
//! ## tenant_id representation (explicit reconciliation)
//! The legacy `duty` table stores `tenant_id` as **TEXT**
//! (`store_sql.rs`), whereas `role_duty` stores it as a **BLOB** (the
//! raw 16-byte UUID). We do not hand-convert: by writing through
//! [`DutyStore::insert`] we inherit exactly the BLOB encoding the
//! scheduler reads back (`DutyStore::list_active` /
//! `list_for_tenant` bind `tenant.0.as_bytes()`). The same `TenantId`
//! value the hire resolved flows straight through, so the write and the
//! scheduler's read agree on representation by construction — no silent
//! TEXT-vs-BLOB mismatch.
//!
//! ## Tenant isolation
//! Each `DutyRecord` carries the caller's resolved `tenant_id`; the
//! scheduler's reads are tenant-scoped, so tenant A's hire produces
//! rows only A can list. Pinned by the bridge tests.

use argo_identity::{HireDraft, WorkerId};
use argo_orchestrator::{DutyRecord, DutyStore};
use argo_store::SessionStore;
use argo_types::TenantId;
use std::sync::Arc;

/// Mirror a freshly-hired worker's **enabled** duties into the
/// scheduler's `role_duty` table.
///
/// Only duties with `default_enabled = true` are bridged — the exact
/// set `create_worker_from_draft` persists into the worker's role (see
/// `store_hire.rs`: the role's `duties` are filtered on
/// `default_enabled`). Keeping the same filter means the scheduler's
/// view of the worker matches the worker's own role: no phantom
/// disabled duties, no missing enabled ones.
///
/// Returns the number of `role_duty` rows written. A per-row insert
/// failure is **non-fatal** (the worker is already committed; failing
/// the HTTP response here would be misleading) — it is logged as a
/// warning, mirroring how the hire path treats audit-write failures.
pub(super) async fn bridge_hire_duties(
    store: &Arc<SessionStore>,
    tenant_id: TenantId,
    worker_id: WorkerId,
    draft: &HireDraft,
) -> usize {
    let duty_store = DutyStore::new(Arc::clone(store));
    let mut written = 0;
    for duty in draft.duties.iter().filter(|d| d.default_enabled) {
        // Normalize the cron through the SAME canonical converter the HTTP
        // duties writer uses (5-field wire → 6-field stored). Writing a raw
        // 5-field cron verbatim (the prior behaviour) makes the scheduler's
        // 6-field parser reject it, and `should_fire`'s `Err` then aborts the
        // WHOLE `duty_scheduler` tick (multi-tenant blast radius). Skip+warn a
        // bad cron rather than commit a poison row.
        let stored_cron =
            match super::routes_duties::normalize_stored_cron(&duty.cron) {
                Ok(c) => c,
                Err(e) => {
                    tracing::warn!(
                        target: "gateway::workers_hire",
                        error = %e,
                        worker_id = %worker_id,
                        cron = %duty.cron,
                        entry_action = %duty.entry_action,
                        "hire→scheduler duty skipped: invalid cron (would poison scheduler tick)"
                    );
                    continue;
                }
            };
        let record = DutyRecord::new(
            tenant_id,
            worker_id,
            stored_cron,
            duty.entry_action.clone(),
            duty.description.clone(),
        );
        match duty_store.insert(&record).await {
            Ok(()) => written += 1,
            Err(e) => {
                tracing::warn!(
                    target: "gateway::workers_hire",
                    error = %e,
                    worker_id = %worker_id,
                    cron = %duty.cron,
                    entry_action = %duty.entry_action,
                    "hire→scheduler role_duty bridge insert failed"
                );
            }
        }
    }
    written
}

#[cfg(test)]
#[path = "routes_workers_hire_duty_bridge_tests.rs"]
mod tests;
