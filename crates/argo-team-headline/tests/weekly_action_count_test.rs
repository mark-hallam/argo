//! Integration pins for `weekly_action_count` — the team-wide "actions
//! this week" rollup the `/team` hero KPI renders.
//!
//! These pin the N1 / F12 fix from the 2026-06-13 platform shakedown:
//! the hero's "actions this week" and a teammate card's "actions taken
//! today" disagreed ("19 today cannot fit inside 7 this week") because
//! they came from two different counters. `weekly_action_count` uses the
//! SAME audit-kind definition of "action" (assistant_message +
//! tool_call) the per-worker card rollup
//! ([`argo_team_headline::activity_counts_for_worker`]) uses, over a
//! 7-day window that strictly contains the card's 24-hour one — so
//! header-week >= card-today holds by construction.

use std::sync::Arc;

use argo_audit::{AuditKind, AuditSink};
use argo_store::{SessionCreate, SessionStore};
use argo_team_headline::{activity_counts_for_worker, weekly_action_count};
use argo_types::{SessionId, TenantId, WorkspaceId};
use chrono::Utc;
use uuid::Uuid;

async fn fresh_store() -> Arc<SessionStore> {
    let store = Arc::new(SessionStore::open(":memory:").await.expect("open store"));
    store.migrate().await.expect("migrate store");
    store
}

/// Insert a workspace for `tenant` and return its id.
async fn seed_workspace(store: &Arc<SessionStore>, tenant: TenantId) -> WorkspaceId {
    let ws = WorkspaceId(Uuid::now_v7());
    sqlx::query(
        "INSERT INTO workspace (id, tenant_id, name, root_path, created_at) \
         VALUES (?, ?, 'weekly-test', '.', strftime('%s','now'))",
    )
    .bind(ws.0.as_bytes().as_slice())
    .bind(tenant.0.as_bytes().as_slice())
    .execute(store.pool())
    .await
    .expect("insert workspace");
    ws
}

/// Create a worker + session in `tenant`/`workspace`, returning the
/// session id (the audit-row anchor) and the worker id.
async fn seed_worker(
    store: &Arc<SessionStore>,
    tenant: TenantId,
    workspace: WorkspaceId,
) -> (argo_identity::WorkerId, SessionId) {
    let worker_uuid = Uuid::now_v7();
    let session = store
        .create_session_with_worker(SessionCreate {
            tenant_id: tenant,
            workspace_id: workspace,
            model: "test-model",
            provider: "test-provider",
            title: Some("seed"),
            vertical_id: None,
            worker_id: Some(argo_store::WorkerId(worker_uuid)),
        })
        .await
        .expect("create session");
    (argo_identity::WorkerId(worker_uuid), session)
}

async fn write_kinds(store: &Arc<SessionStore>, session: SessionId, kinds: &[AuditKind]) {
    let sink = AuditSink::new_with_tenant(Arc::clone(store), "weekly-test".to_string());
    for kind in kinds {
        sink.write(Some(session), *kind, serde_json::json!({}), None)
            .await
            .expect("write audit row");
    }
}

/// `weekly_action_count` sums `assistant_message` + `tool_call` rows
/// across EVERY worker in the tenant, excludes non-action kinds
/// (escalations), and never leaks another tenant's rows.
#[tokio::test]
async fn aggregates_actions_across_the_team_and_stays_tenant_scoped() {
    let store = fresh_store().await;
    let tenant = TenantId(Uuid::now_v7());
    let ws = seed_workspace(&store, tenant).await;
    // Two teammates in the SAME tenant — the rollup is team-wide.
    let (_alice, alice_session) = seed_worker(&store, tenant, ws).await;
    let (_bob, bob_session) = seed_worker(&store, tenant, ws).await;
    // A separate tenant whose rows must NOT leak into the total.
    let other_tenant = TenantId(Uuid::now_v7());
    let other_ws = seed_workspace(&store, other_tenant).await;
    let (_carol, carol_session) = seed_worker(&store, other_tenant, other_ws).await;

    // Alice: 2 messages + 1 tool call = 3 actions.
    write_kinds(
        &store,
        alice_session,
        &[
            AuditKind::AssistantMessage,
            AuditKind::AssistantMessage,
            AuditKind::ToolCall,
        ],
    )
    .await;
    // Bob: 3 tool calls + 2 escalations (escalations are NOT actions) = 3.
    write_kinds(
        &store,
        bob_session,
        &[
            AuditKind::ToolCall,
            AuditKind::ToolCall,
            AuditKind::ToolCall,
            AuditKind::EscalationRequired,
            AuditKind::EscalationRequired,
        ],
    )
    .await;
    // Carol (other tenant): 4 tool calls.
    write_kinds(
        &store,
        carol_session,
        &[
            AuditKind::ToolCall,
            AuditKind::ToolCall,
            AuditKind::ToolCall,
            AuditKind::ToolCall,
        ],
    )
    .await;

    let team_week = weekly_action_count(&store, tenant, Utc::now())
        .await
        .expect("weekly count");
    assert_eq!(
        team_week, 6,
        "sums across teammates (3 + 3); excludes escalations + other tenant"
    );

    let other_week = weekly_action_count(&store, other_tenant, Utc::now())
        .await
        .expect("weekly count other tenant");
    assert_eq!(other_week, 4, "tenant-scoped: only the other tenant's rows");
}

/// The load-bearing invariant from the shakedown (N1 / F12): a worker's
/// "actions today" (card) can never exceed the team's "actions this
/// week" (hero). The week window strictly contains the day window AND
/// counts the same kinds — including a row aged past 24h but inside the
/// 7-day week, which the card drops but the weekly rollup keeps.
#[tokio::test]
async fn weekly_count_is_never_less_than_card_today() {
    let store = fresh_store().await;
    let tenant = TenantId(Uuid::now_v7());
    let ws = seed_workspace(&store, tenant).await;
    let (worker, session) = seed_worker(&store, tenant, ws).await;
    let now = Utc::now();

    // Two "earlier this week" rows (between 24h and 7d ago): the daily
    // card does NOT count them, but the weekly rollup does. Write them
    // first, then backdate every row currently in the table to 3 days
    // ago (the same backdating technique `argo-audit`'s archive tests
    // use — `AuditSink` has no timestamp-override write API).
    write_kinds(
        &store,
        session,
        &[AuditKind::AssistantMessage, AuditKind::ToolCall],
    )
    .await;
    sqlx::query("UPDATE audit_record SET created_at = ?")
        .bind((now - chrono::Duration::days(3)).timestamp_micros())
        .execute(store.pool())
        .await
        .expect("backdate rows");

    // 4 actions "today" (within 24h), stamped now.
    write_kinds(
        &store,
        session,
        &[
            AuditKind::ToolCall,
            AuditKind::ToolCall,
            AuditKind::ToolCall,
            AuditKind::ToolCall,
        ],
    )
    .await;

    let counts = activity_counts_for_worker(&store, tenant, worker, now)
        .await
        .expect("daily counts");
    let card_today = counts.assistant_messages + counts.tool_calls;
    let header_week = weekly_action_count(&store, tenant, now)
        .await
        .expect("weekly count");

    assert_eq!(card_today, 4, "only the 4 within-24h rows count for today");
    assert_eq!(header_week, 6, "all 6 within-7d rows count for the week");
    assert!(
        header_week >= card_today,
        "header-week ({header_week}) must be >= card-today ({card_today})"
    );
}

/// Rows older than the 7-day window are excluded from the weekly rollup.
#[tokio::test]
async fn drops_events_outside_the_7d_window() {
    let store = fresh_store().await;
    let tenant = TenantId(Uuid::now_v7());
    let ws = seed_workspace(&store, tenant).await;
    let (_worker, session) = seed_worker(&store, tenant, ws).await;
    write_kinds(&store, session, &[AuditKind::ToolCall]).await;

    // Query as if 8 days have passed — the seeded row is now outside the
    // 7-day lookback.
    let future = Utc::now() + chrono::Duration::days(8);
    let week = weekly_action_count(&store, tenant, future)
        .await
        .expect("weekly count");
    assert_eq!(week, 0);
}
