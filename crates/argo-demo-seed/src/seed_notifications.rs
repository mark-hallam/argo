//! Bell-popover notification seeder for the canonical v1 demo.
//!
//! The notification bell renders on every authenticated screen, so an
//! empty bell is a camera-visible NO-THEATER gap: the
//! `GET /api/v1/notifications` endpoint exists and is correctly gated
//! (`routes_notifications.rs`), but the demo tenant had zero
//! `notifications` rows, so the SPA `NotificationsPopover` rendered its
//! "Nothing new." empty state on every walkthrough.
//!
//! This module inserts **real** broadcast `notifications` rows that
//! mirror events already seeded elsewhere in the canonical run — the
//! seeded approval requests (`seed::seed_approval_requests`), the hero
//! workers (`personas::DEMO_SEED_PERSONAS`), and the seeded
//! `customer_research_followup` workflow (`workflow.rs`). No mock, no
//! bypass: the rows the bell shows are genuine `notifications` table
//! rows the live endpoint serves.
//!
//! Why broadcast (`principal_id = NULL`): a notification addressed at a
//! specific principal is only visible to that principal
//! (`list_for_principal` filters `principal_id IS NULL OR principal_id =
//! ?`). The demo principal is resolved from the session JWT at request
//! time and is not knowable at seed time, so addressing a row to a
//! guessed principal would risk an invisible bell. Broadcast rows are
//! the correct model for "system-level workspace event" anyway (budget
//! alerts, onboarding milestones, approvals pending) and are visible to
//! every principal in the tenant.
//!
//! Idempotent: if any non-dismissed `notifications` row already exists
//! for the tenant the whole block is skipped (mirrors the approval-set
//! skip-if-exists guard), so re-seeding never duplicates the bell.
//!
//! Uses the canonical `argo_store::notifications::insert` writer (the
//! same path any runtime emitter would use) rather than raw SQL, so the
//! `ensure_schema` lazy-bootstrap + `INSERT OR IGNORE` idempotency the
//! endpoint relies on are shared.

use argo_store::notifications::{insert, list_for_principal, ListQuery, NotificationInsert};
use argo_store::SessionStore;
use argo_types::TenantId;
use chrono::Utc;
use std::sync::Arc;
use uuid::Uuid;

use crate::SeedResult;

/// One canonical demo notification. `minutes_ago` spreads the rows
/// across the recent past so the popover's Today / Yesterday / Earlier
/// grouping renders more than one section. `read` marks the older rows
/// read so the unread badge shows a believable non-zero-but-not-all
/// count (the bell badge reads `unread_count`).
struct DemoNotification {
    kind: &'static str,
    body: &'static str,
    severity: &'static str,
    minutes_ago: i64,
    read: bool,
}

/// Five broadcast notifications mirroring real seeded events:
///   1. Marcus (Marketing) drafted a reply — approval pending (ties to
///      the seeded approval inbox; Marcus is a draft-only teammate).
///   2. Mariah's listing-repost approval is waiting (one of the five
///      seeded pending approval requests).
///   3. The `customer_research_followup` workflow completed a run
///      (ties to the seeded workflow + the seeded run history).
///   4. Priya booked a first-round interview (a seeded recruiting duty).
///   5. A skill the workers learned this month was re-evaluated (ties
///      to the seeded skill-reeval history).
const DEMO_NOTIFICATIONS: &[DemoNotification] = &[
    DemoNotification {
        kind: "approval-pending",
        body: "Marcus drafted a reply for review — approval pending in your inbox.",
        severity: "info",
        minutes_ago: 8,
        read: false,
    },
    DemoNotification {
        kind: "approval-pending",
        body: "Mariah is waiting on approval to repost listing #4912 at $215.",
        severity: "warning",
        minutes_ago: 41,
        read: false,
    },
    DemoNotification {
        kind: "workflow-run-complete",
        body: "Workflow “customer_research_followup” finished a run — follow-up note dropped in Notes.",
        severity: "info",
        minutes_ago: 95,
        read: false,
    },
    DemoNotification {
        kind: "duty-output-ready",
        body: "Priya booked a first-round interview with Diego Alvarez for Thursday.",
        severity: "info",
        minutes_ago: 1_500, // ~yesterday
        read: true,
    },
    DemoNotification {
        kind: "skill-reeval",
        body: "Skill “competitor pricing scrape” passed its weekly re-evaluation (96%).",
        severity: "info",
        minutes_ago: 2_900, // ~2 days ago → "Earlier"
        read: true,
    },
];

/// Insert the canonical demo notifications as **broadcast** rows under
/// `tenant_id` so the bell-popover (`GET /api/v1/notifications`) and
/// the unread-count badge (`GET /api/v1/notifications/unread-count`)
/// render real rows instead of the SPA's "Nothing new." empty state.
///
/// Returns the number of rows inserted (`5` on first run, `0` on
/// re-seed). Idempotent: a tenant that already has any non-dismissed
/// notification is skipped wholesale.
pub(crate) async fn seed_notifications(
    store: &Arc<SessionStore>,
    tenant_id: TenantId,
) -> SeedResult<usize> {
    let pool = store.pool();
    let tenant = tenant_id.to_string();
    // Idempotency guard: broadcast rows are visible to every principal,
    // so a `None`-principal probe sees the full broadcast set. If any
    // exist, a prior run seeded the bell — skip the whole block.
    let existing = list_for_principal(pool, &tenant, None, &ListQuery::default()).await?;
    if !existing.items.is_empty() {
        return Ok(0);
    }
    let now = Utc::now().timestamp();
    let mut inserted = 0usize;
    for note in DEMO_NOTIFICATIONS {
        let id = format!("nt_{}", Uuid::now_v7());
        let created_at = now - note.minutes_ago * 60;
        insert(
            pool,
            &NotificationInsert {
                id: &id,
                tenant_id: &tenant,
                principal_id: None,
                kind: note.kind,
                body: note.body,
                created_at,
                related_audit_id: None,
                severity: note.severity,
            },
        )
        .await?;
        if note.read {
            mark_read(store, &tenant, &id, created_at).await?;
        }
        inserted += 1;
    }
    Ok(inserted)
}

/// Stamp `read_at` on a freshly-seeded row so the unread badge reads a
/// believable count (only the recent rows stay unread). There is no
/// public `mark_read` in the notifications store yet (the SPA's
/// mark-read endpoint is a sibling-brief stub), so the seed writes the
/// column directly — same table, same tenant scope as the canonical
/// writer.
async fn mark_read(
    store: &Arc<SessionStore>,
    tenant: &str,
    id: &str,
    now: i64,
) -> SeedResult<()> {
    sqlx::query(
        "UPDATE notifications SET read_at = ? WHERE id = ? AND tenant_id = ? AND read_at IS NULL",
    )
    .bind(now)
    .bind(id)
    .bind(tenant)
    .execute(store.pool())
    .await
    .map_err(|e| argo_types::ArgoError::Provider(format!("notifications mark_read: {e}")))?;
    Ok(())
}

#[cfg(test)]
#[path = "seed_notifications_tests.rs"]
mod tests;
