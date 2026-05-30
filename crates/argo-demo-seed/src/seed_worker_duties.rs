//! Per-worker duty (schedule) seeder for the canonical v1 demo.
//!
//! Scene 2 of the v1 demo runbook
//! (`docs/skycot/demos/v1-mission-mvp-runbook.md`) lands the camera on
//! Mariah's overview and the talk track names **four** responsibilities.
//! Before this seeder the demo tenant had zero `role_duty` rows, so the
//! Responsibilities / Schedules card rendered the empty state
//! ("No schedules yet") while the narration claimed four duties — a
//! camera-visible theater gap.
//!
//! This module inserts **real** `role_duty` rows (the same table the
//! live worker-overview endpoint reads via
//! `routes_worker_overview_kpis::duties_for_worker` and the duty
//! scheduler fires from). No mock, no bypass: the duties the operator
//! sees on camera are genuine scheduler rows.
//!
//! Raw SQL (not the `argo-orchestrator::DutyStore` API) matches the
//! existing seed-crate convention — every sibling seeder
//! (`seed_worker_skill_assignments`, `seed_retrain_records`,
//! `seed_demo_marketplace_skills`) writes via `sqlx::query` and the
//! seed crate deliberately avoids the heavyweight `argo-orchestrator`
//! → `argo-runtime` dependency. The column set mirrors
//! `crates/argo-store/migrations/93__role_duties.sql` exactly.
//!
//! Idempotent: if any `role_duty` row already exists for a worker the
//! whole worker is skipped, so re-seeding (e.g. to add a teammate)
//! never duplicates the card.

use argo_store::SessionStore;
use argo_types::TenantId;
use chrono::Utc;
use std::sync::Arc;
use uuid::Uuid;

use crate::personas::DemoPersona;
use crate::SeedResult;

/// One canonical demo duty for a hero worker.
///
/// `cron` is the 6-field form (`sec min hour dom mon dow`) the `cron`
/// crate parses — the same form the gateway's `next_fire_for` evaluates
/// and the duty scheduler fires on.
struct DemoDuty {
    cron: &'static str,
    entry_action: &'static str,
    description: &'static str,
}

/// Mariah's four responsibilities, verbatim from the Scene-2 runbook
/// talk track. `description` is the operator-visible label the
/// Responsibilities card renders (the gateway surfaces `description`
/// as the duty `label`); `entry_action` is the worker action-vocabulary
/// handler name.
const MARIAH_DUTIES: &[DemoDuty] = &[
    DemoDuty {
        // "as items arrive" → event-shaped cadence; modelled as a
        // frequent poll so the card reads as always-on. Every 15 min.
        cron: "0 */15 * * * *",
        entry_action: "list_new_inventory",
        description: "List new inventory with photos + tags",
    },
    DemoDuty {
        // "daily 06:30"
        cron: "0 30 6 * * *",
        entry_action: "reprice_stale_listings",
        description: "Re-price stale listings against competitor data",
    },
    DemoDuty {
        // "as messages arrive" → frequent poll. Every 10 min.
        cron: "0 */10 * * * *",
        entry_action: "answer_buyer_questions",
        description: "Respond to buyer questions on active listings",
    },
    DemoDuty {
        // "daily 06:30"
        cron: "0 30 6 * * *",
        entry_action: "flag_below_floor",
        description: "Flag items dropping below floor price",
    },
];

const PRIYA_DUTIES: &[DemoDuty] = &[
    DemoDuty {
        cron: "0 */10 * * * *",
        entry_action: "screen_inbound_applicants",
        description: "Screen inbound applicants against the role",
    },
    DemoDuty {
        cron: "0 0 9 * * 1-5",
        entry_action: "book_first_round_interviews",
        description: "Book first-round interviews from the shortlist",
    },
];

const JAMAL_DUTIES: &[DemoDuty] = &[
    DemoDuty {
        cron: "0 */5 * * * *",
        entry_action: "triage_support_tickets",
        description: "Triage and answer Tier-1 support tickets",
    },
    DemoDuty {
        cron: "0 0 17 * * 1-5",
        entry_action: "summarize_open_tickets",
        description: "Summarise open tickets for the daily handoff",
    },
];

const NORI_DUTIES: &[DemoDuty] = &[
    DemoDuty {
        cron: "0 0 9 * * 1",
        entry_action: "draft_weekly_comms",
        description: "Draft the weekly product comms for review",
    },
    DemoDuty {
        cron: "0 0 8 * * *",
        entry_action: "track_announcement_reach",
        description: "Track reach on published announcements",
    },
];

const PETA_DUTIES: &[DemoDuty] = &[
    DemoDuty {
        cron: "0 0 9 * * 1-5",
        entry_action: "check_delivery_dates",
        description: "Check delivery dates and surface slippage",
    },
    DemoDuty {
        cron: "0 0 16 * * 1-5",
        entry_action: "unblock_ics",
        description: "Chase blockers so ICs stay unblocked",
    },
];

/// A small, real duty set for each scripted hero so any worker the
/// camera might wander onto also shows genuine responsibilities (no
/// empty-state surprise off the scripted path). Keyed by persona name
/// to stay aligned with `DEMO_SEED_PERSONAS` without depending on slot
/// order. Workers not listed here simply get no duties (acceptable —
/// only Mariah is the scripted Scene-2 hero).
fn duties_for_persona(name: &str) -> &'static [DemoDuty] {
    match name {
        "Mariah" => MARIAH_DUTIES,
        "Priya" => PRIYA_DUTIES,
        "Jamal" => JAMAL_DUTIES,
        "Nori" => NORI_DUTIES,
        "Peta" => PETA_DUTIES,
        _ => &[],
    }
}

/// Insert canonical `role_duty` rows for the hero workers so the
/// Responsibilities / Schedules card on each worker's overview renders
/// real duties. Mariah gets the four runbook responsibilities; the
/// other scripted heroes get a small real set.
///
/// `personas` and `worker_ids` are positional-aligned (both come from
/// `DEMO_SEED_PERSONAS` order via `seed_demo_workers`), so index `i`
/// maps persona name → worker id.
///
/// Idempotent per worker: a worker that already has any `role_duty`
/// row is skipped.
pub(crate) async fn seed_worker_duties(
    store: &Arc<SessionStore>,
    tenant_id: TenantId,
    personas: &[DemoPersona],
    worker_ids: &[argo_identity::WorkerId],
) -> SeedResult<usize> {
    let now = Utc::now().timestamp_micros();
    let mut inserted = 0usize;
    for (persona, worker_id) in personas.iter().zip(worker_ids.iter()) {
        let duties = duties_for_persona(persona.name);
        if duties.is_empty() {
            continue;
        }
        if worker_has_duties(store, tenant_id, *worker_id).await? {
            continue;
        }
        for duty in duties {
            insert_duty(store, tenant_id, *worker_id, duty, now).await?;
            inserted += 1;
        }
    }
    Ok(inserted)
}

async fn worker_has_duties(
    store: &Arc<SessionStore>,
    tenant_id: TenantId,
    worker_id: argo_identity::WorkerId,
) -> SeedResult<bool> {
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM role_duty WHERE tenant_id = ? AND worker_id = ?")
            .bind(tenant_id.0.as_bytes().as_slice())
            .bind(worker_id.to_string())
            .fetch_one(store.pool())
            .await
            .map_err(|e| argo_types::ArgoError::Provider(format!("role_duty probe: {e}")))?;
    Ok(count > 0)
}

async fn insert_duty(
    store: &Arc<SessionStore>,
    tenant_id: TenantId,
    worker_id: argo_identity::WorkerId,
    duty: &DemoDuty,
    now: i64,
) -> SeedResult<()> {
    sqlx::query(
        "INSERT INTO role_duty \
         (id, tenant_id, worker_id, cron, entry_action, description, \
          enabled, paused_until, last_fired_at, created_at) \
         VALUES (?, ?, ?, ?, ?, ?, 1, NULL, NULL, ?)",
    )
    .bind(Uuid::now_v7().to_string())
    .bind(tenant_id.0.as_bytes().as_slice())
    .bind(worker_id.to_string())
    .bind(duty.cron)
    .bind(duty.entry_action)
    .bind(duty.description)
    .bind(now)
    .execute(store.pool())
    .await
    .map_err(|e| argo_types::ArgoError::Provider(format!("insert role_duty: {e}")))?;
    Ok(())
}

#[cfg(test)]
#[path = "seed_worker_duties_tests.rs"]
mod tests;
