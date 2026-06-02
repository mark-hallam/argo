//! Seeds **session-linked**, recent activity per hero worker so the
//! `/team` headline cards render real bullets (e.g. "6 replies sent",
//! "6 tickets resolved today") instead of the "No activity yet."
//! fallback.
//!
//! ## Why this exists
//! The team-headline rollup
//! ([`argo_team_headline::activity_counts_for_worker`]) counts
//! `audit_record` rows of kind `assistant_message` / `tool_call`
//! JOINed `audit_record.session_id -> session.worker_id`, within the
//! last 24 hours. Every OTHER demo-seed audit write passes
//! `session_id = None` (see `seed.rs` Pass A/B, `audit_backstory.rs`),
//! so the rollup's JOIN drops them all → the cards read "No activity
//! yet." even though the rows exist (the sibling `/activity` endpoint
//! finds them by `payload.worker_id`, a different attribution path).
//!
//! This seeder closes that gap by creating ONE `session` per hero,
//! stamped with the worker's id via the canonical
//! [`SessionStore::create_session_with_worker`] (the only API that
//! writes `session.worker_id`), then writing a handful of
//! session-linked activity rows. `created_at` is stamped `Utc::now()`
//! by the sink, so a re-seed before recording always lands inside the
//! rollup's 24h window — the function is self-freshening.
//!
//! NB the existing backstory / Pass-A / Pass-B audit rows are left
//! untouched: they drive the `/audit` feed and the `/activity` tab via
//! payload-matching and must stay as-is. This adds the session-linked
//! rows the headline rollup specifically needs.

use std::sync::Arc;

use argo_audit::{AuditKind, AuditSink};
use argo_store::{SessionCreate, SessionStore};
use argo_types::{ArgoError, TenantId};

use crate::personas::DemoPersona;
use crate::SeedResult;

/// Per-worker `(assistant_message, tool_call)` counts to seed,
/// positional-aligned with `DEMO_SEED_PERSONAS`. Counts are varied so
/// no two cards show an identical number, and chosen so each worker's
/// `bullets_for_vertical` template (see `kpi_table.rs`) renders a
/// sensible real bullet:
///   - Mariah (marketplace→real-estate): "4 listings replied" + "3 viewings scheduled"
///   - Peta (project-coordination→fallback): "8 actions taken today"
///   - Priya (recruiting): "2 interviews booked" + "5 candidates screened"
///   - Jamal (support): "6 tickets resolved today"
///   - Nori (marketing→fallback): "6 actions taken today"
///   - Portia (pitch-agent→fallback): "6 actions taken today"
///   - Grant (gtm): "6 replies sent"
///   - Marcus (marketing-agent→fallback): "4 actions taken today"
const ACTIVITY_PLAN: &[(u32, u32)] = &[
    (4, 3), // Mariah
    (3, 5), // Peta
    (5, 2), // Priya
    (2, 6), // Jamal
    (5, 1), // Nori
    (4, 2), // Portia
    (6, 2), // Grant
    (3, 1), // Marcus
];

/// Seed session-linked recent activity for each hero worker so the
/// team-headline rollup counts real per-worker activity.
///
/// `personas` and `worker_ids` are positional-aligned (both walk the
/// `DEMO_SEED_PERSONAS` spine, like `seed_trust_grades`). Returns the
/// number of workers given fresh activity.
///
/// Re-running re-stamps fresh rows (kept inside the 24h window); it
/// adds another session's worth of rows rather than mutating prior
/// ones, which is acceptable for the demo's clean single-seed flow.
pub(crate) async fn seed_team_headline_activity(
    store: &Arc<SessionStore>,
    tenant_id: TenantId,
    personas: &[DemoPersona],
    worker_ids: &[argo_identity::WorkerId],
) -> SeedResult<usize> {
    // The `/team` endpoint resolves its workspace via
    // `ensure_workspace(cwd_name, cwd_root)` (routes_team_list.rs
    // `resolve_workspace_tenant`). Replicate it so the session lands in
    // the same `(tenant, workspace)` the rollup queries. `ensure_workspace`
    // is idempotent by root path, so this reuses the existing row.
    let cwd = std::env::current_dir()
        .map_err(|e| ArgoError::Io(format!("seed_team_headline_activity current_dir: {e}")))?;
    let name = cwd.file_name().and_then(|s| s.to_str()).unwrap_or("workspace");
    let root = cwd.to_string_lossy();
    let (_ws_tenant, workspace_id) = store.ensure_workspace(name, &root).await?;

    let sink = AuditSink::new_with_tenant(Arc::clone(store), tenant_id.to_string());
    let mut seeded = 0usize;
    for (i, (persona, worker_id)) in personas.iter().zip(worker_ids.iter()).enumerate() {
        let (messages, tool_calls) = ACTIVITY_PLAN.get(i).copied().unwrap_or((3, 2));
        let session_id = store
            .create_session_with_worker(SessionCreate {
                tenant_id,
                workspace_id,
                model: "demo-seed-headline",
                provider: "demo-seed",
                title: Some(persona.name),
                vertical_id: Some(persona.vertical_id),
                worker_id: Some(*worker_id),
            })
            .await?;

        for seq in 0..messages {
            let payload = serde_json::json!({
                "worker_id": worker_id.to_string(),
                "vertical_id": persona.vertical_id,
                "seq": seq,
                "source": "seed_team_headline_activity",
            });
            sink.write(Some(session_id), AuditKind::AssistantMessage, payload, None)
                .await?;
        }
        for seq in 0..tool_calls {
            let payload = serde_json::json!({
                "worker_id": worker_id.to_string(),
                "vertical_id": persona.vertical_id,
                "seq": seq,
                "source": "seed_team_headline_activity",
            });
            sink.write(Some(session_id), AuditKind::ToolCall, payload, None)
                .await?;
        }
        seeded += 1;
    }
    sink.flush_pending().await?;
    Ok(seeded)
}

#[cfg(test)]
#[path = "seed_team_headline_activity_tests.rs"]
mod tests;
