//! Per-worker trust-grade seeder for the canonical v1 demo.
//!
//! The worker-detail Trust tab (`routes_worker_trust_grade.rs` →
//! `GET /api/v1/workers/<id>/trust-grade`) reads `worker.trust_grade`.
//! A freshly-`draft`ed worker has `trust_grade = 0`
//! (`LearnedUnreviewed`), so every seeded teammate's Trust tab rendered
//! the same red "Learned, unreviewed" floor — cosmetically wrong for a
//! roster of established teammates the demo presents as a working
//! workforce.
//!
//! This module sets a realistic, **honest** non-zero grade per persona,
//! consistent with each one's seniority. No teammate is inflated to
//! `Builtin` (4, reserved for first-party skills); the most-established
//! sit at `PublishedVerified` (3, green), most at `Tenant` (2), and the
//! newest (draft-only Marcus) at `PublishedCommunity` (1). The grade is
//! a real column the routing engine's candidate ranker also reads, so
//! this is genuine data, not decoration.
//!
//! The numeric ladder mirrors `routes_worker_trust_grade::grade_for_u8`:
//!   0 → Learned, unreviewed · 1 → Published community ·
//!   2 → Tenant · 3 → Published verified · 4+ → Builtin
//!
//! Written via the canonical `IdentityStore::set_worker_trust_grade`
//! (tenant + worker scoped UPDATE), the same path the operator-promote
//! endpoint uses. Idempotent by construction: an UPDATE to the same
//! value is a no-op, so re-seeding never drifts a grade.

use argo_identity::IdentityStore;
use argo_store::SessionStore;
use argo_types::TenantId;
use std::sync::Arc;

use crate::personas::DemoPersona;
use crate::SeedResult;

/// Honest trust grade for a persona by name, keyed off the same spine
/// `seed_demo_workers` walked so a roster change can't silently leave a
/// worker at the unreviewed floor. Grades reflect seniority, not
/// flattery:
///   - Mariah (headline established lister) + Portia (founder-caliber
///     pitch agent) → 3 (Published verified)
///   - Peta / Priya / Jamal / Nori / Grant (established line workers)
///     → 2 (Tenant)
///   - Marcus (newest, draft-only marketing teammate) → 1 (Published
///     community)
///
/// A persona not listed gets `None` → left at the `draft` default
/// rather than guessed.
fn grade_for_persona(name: &str) -> Option<u8> {
    match name {
        "Mariah" | "Portia" => Some(3),
        "Peta" | "Priya" | "Jamal" | "Nori" | "Grant" => Some(2),
        "Marcus" => Some(1),
        _ => None,
    }
}

/// Set an honest non-zero `trust_grade` on each seeded hero worker so
/// the worker-detail Trust tab shows a real grade instead of the
/// `draft`-default unreviewed floor.
///
/// `personas` and `worker_ids` are positional-aligned (both come from
/// `DEMO_SEED_PERSONAS` order via `seed_demo_workers`), so index `i`
/// maps persona name → worker id.
///
/// Returns the number of workers whose grade was set. Idempotent: the
/// UPDATE re-applies the same value on re-seed.
pub(crate) async fn seed_trust_grades(
    store: &Arc<SessionStore>,
    tenant_id: TenantId,
    personas: &[DemoPersona],
    worker_ids: &[argo_identity::WorkerId],
) -> SeedResult<usize> {
    let identities = IdentityStore::new(store);
    let mut set = 0usize;
    for (persona, worker_id) in personas.iter().zip(worker_ids.iter()) {
        let Some(grade) = grade_for_persona(persona.name) else {
            continue;
        };
        identities
            .set_worker_trust_grade(tenant_id, *worker_id, grade)
            .await?;
        set += 1;
    }
    Ok(set)
}

#[cfg(test)]
#[path = "seed_trust_grades_tests.rs"]
mod tests;
