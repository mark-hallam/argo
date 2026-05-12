//! End-to-end + bullet-shaping tests for `argo-team-headline`.

use std::sync::Arc;

use argo_audit::{AuditKind, AuditSink};
use argo_identity::WorkerId;
use argo_store::{SessionCreate, SessionStore};
use argo_types::{SessionId, TenantId, WorkspaceId};
use chrono::Utc;
use uuid::Uuid;

use crate::compute::{activity_counts_for_worker, ActivityCounts};
use crate::kpi_table::{bullets_for_vertical, BulletKind, BulletSource};
use crate::tomorrow::next_duty_for_worker;
use crate::{generate_team_headline, render_bullets, MAX_BULLETS, MAX_BULLET_CHARS};

const ROLE_DUTY_SCHEMA: &str =
    include_str!("../../argo-store/migrations/28__role_duties.sql");

// Pure shaping tests (no DB)

#[test]
fn bullets_for_vertical_gtm_engineer_lists_three_bullets() {
    assert_eq!(bullets_for_vertical("gtm-engineer").len(), 3);
    assert_eq!(bullets_for_vertical("marketplace-seller").len(), 3);
}

#[test]
fn render_bullets_zero_activity_returns_no_activity_yet() {
    let bullets = render_bullets("gtm-engineer", &ActivityCounts::default());
    assert_eq!(bullets, vec!["No activity yet.".to_string()]);
}

#[test]
fn render_bullets_caps_at_max_bullets() {
    let counts = ActivityCounts { assistant_messages: 4, tool_calls: 5, pending_escalations: 6 };
    assert_eq!(render_bullets("recruiter", &counts).len(), MAX_BULLETS);
}

#[test]
fn render_bullets_drops_zero_counts() {
    let counts = ActivityCounts { assistant_messages: 0, tool_calls: 2, pending_escalations: 0 };
    assert_eq!(
        render_bullets("recruiter", &counts),
        vec!["2 interviews booked this week".to_string()]
    );
}

#[test]
fn render_bullets_customer_support_drops_avg_response_time() {
    let counts = ActivityCounts { assistant_messages: 0, tool_calls: 5, pending_escalations: 2 };
    assert_eq!(
        render_bullets("customer-support", &counts),
        vec![
            "5 tickets resolved today".to_string(),
            "2 pending escalations".to_string(),
        ]
    );
}

#[test]
fn render_bullets_fallback_renders_actions_taken_plus_pending() {
    let counts = ActivityCounts { assistant_messages: 1, tool_calls: 2, pending_escalations: 4 };
    let bullets = render_bullets("unknown-vertical", &counts);
    assert!(bullets.iter().any(|s| s == "3 actions taken today"));
    assert!(bullets.iter().any(|s| s == "4 pending approvals"));
}

#[test]
fn bullet_kind_format_picks_singular_plural_and_drops_zero() {
    let kind = BulletKind {
        source: BulletSource::Messages,
        singular: "1 reply sent",
        plural: "{n} replies sent",
    };
    let one = ActivityCounts { assistant_messages: 1, ..Default::default() };
    let many = ActivityCounts { assistant_messages: 7, ..Default::default() };
    assert_eq!(kind.format(&one), Some("1 reply sent".to_string()));
    assert_eq!(kind.format(&many), Some("7 replies sent".to_string()));
    assert!(kind.format(&ActivityCounts::default()).is_none());
}

#[test]
fn bullet_kind_format_not_computable_v1_is_always_none() {
    let kind = BulletKind {
        source: BulletSource::NotComputableV1,
        singular: "revenue this week",
        plural: "revenue this week",
    };
    let counts = ActivityCounts {
        assistant_messages: 100,
        tool_calls: 100,
        pending_escalations: 100,
    };
    assert!(kind.format(&counts).is_none());
}

#[test]
fn rendered_bullets_fit_max_char_budget() {
    let counts = ActivityCounts {
        assistant_messages: 0,
        tool_calls: 0,
        pending_escalations: 9_999_999_999,
    };
    let bullets = render_bullets("event-coordinator", &counts);
    assert!(bullets.iter().all(|b| b.chars().count() <= MAX_BULLET_CHARS));
}

#[test]
fn every_brief_vertical_has_a_kpi_row() {
    let verticals = [
        "gtm-engineer",
        "marketplace-seller",
        "recruiter",
        "customer-support",
        "ai-receptionist",
        "sales-sdr",
        "sales-follow-up",
        "product-manager",
        "tax-prep",
        "finance-ar",
        "healthcare-intake",
        "legal-paralegal",
        "event-coordinator",
        "real-estate-agent",
        "pr-checker",
        "receptionist",
    ];
    for v in verticals {
        let bullets = bullets_for_vertical(v);
        assert_eq!(bullets.len(), 3, "{v} should map to 3 KPI templates");
    }
}

// ---------------------------------------------------------------------------
// DB-backed integration tests
// ---------------------------------------------------------------------------

struct SeededWorker {
    store: Arc<SessionStore>,
    worker: WorkerId,
    session: SessionId,
}

async fn fresh_store() -> Arc<SessionStore> {
    let store = Arc::new(SessionStore::open(":memory:").await.unwrap());
    store.migrate().await.unwrap();
    store
}

async fn ensure_role_duty_schema(store: &Arc<SessionStore>) {
    for stmt in ROLE_DUTY_SCHEMA
        .split_terminator(';')
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        sqlx::query(stmt)
            .execute(store.pool())
            .await
            .unwrap();
    }
}

async fn insert_duty(
    seed: &SeededWorker,
    id: &str,
    cron: &str,
    description: &str,
    enabled: bool,
    paused_until: Option<chrono::DateTime<Utc>>,
) {
    sqlx::query(
        "INSERT INTO role_duty (id, tenant_id, worker_id, cron, entry_action, \
         description, enabled, paused_until, last_fired_at, created_at) \
         VALUES (?, ?, ?, ?, 'ping', ?, ?, ?, NULL, ?)",
    )
    .bind(id)
    .bind(Uuid::now_v7().as_bytes().as_slice())
    .bind(seed.worker.to_string())
    .bind(cron)
    .bind(description)
    .bind(i64::from(enabled))
    .bind(paused_until.map(|d| d.timestamp_micros()))
    .bind(Utc::now().timestamp_micros())
    .execute(seed.store.pool())
    .await
    .unwrap();
}

async fn seed_worker(store: &Arc<SessionStore>) -> SeededWorker {
    let tenant = TenantId(Uuid::now_v7());
    let ws = WorkspaceId(Uuid::now_v7());
    sqlx::query(
        "INSERT INTO workspace (id, tenant_id, name, root_path, created_at) \
         VALUES (?, ?, 'test-ws', '.', strftime('%s','now'))",
    )
    .bind(ws.0.as_bytes().as_slice())
    .bind(tenant.0.as_bytes().as_slice())
    .execute(store.pool())
    .await
    .unwrap();
    let worker_uuid = Uuid::now_v7();
    let worker = WorkerId(worker_uuid);
    let session = store
        .create_session_with_worker(SessionCreate {
            tenant_id: tenant,
            workspace_id: ws,
            model: "test-model",
            provider: "test-provider",
            title: Some("seed"),
            vertical_id: None,
            worker_id: Some(argo_store::WorkerId(worker_uuid)),
        })
        .await
        .unwrap();
    SeededWorker { store: store.clone(), worker, session }
}

#[tokio::test]
async fn activity_counts_zero_when_db_empty() {
    let store = fresh_store().await;
    let seed = seed_worker(&store).await;
    let counts = activity_counts_for_worker(&seed.store, seed.worker, Utc::now())
        .await
        .unwrap();
    assert_eq!(counts, ActivityCounts::default());
}

#[tokio::test]
async fn activity_counts_aggregates_seeded_audit_events() {
    let store = fresh_store().await;
    let seed = seed_worker(&store).await;
    let sink = AuditSink::new(seed.store.clone());
    for _ in 0..2 {
        sink.write(Some(seed.session), AuditKind::AssistantMessage, serde_json::json!({}), None)
            .await
            .unwrap();
    }
    for _ in 0..3 {
        sink.write(Some(seed.session), AuditKind::ToolCall, serde_json::json!({}), None)
            .await
            .unwrap();
    }
    for _ in 0..4 {
        sink.write(
            Some(seed.session),
            AuditKind::EscalationRequired,
            serde_json::json!({}),
            None,
        )
        .await
        .unwrap();
    }
    sink.write(Some(seed.session), AuditKind::ApprovalGranted, serde_json::json!({}), None)
        .await
        .unwrap();
    let counts = activity_counts_for_worker(&seed.store, seed.worker, Utc::now())
        .await
        .unwrap();
    assert_eq!(counts.assistant_messages, 2);
    assert_eq!(counts.tool_calls, 3);
    // 4 escalations - 1 grant = 3 pending.
    assert_eq!(counts.pending_escalations, 3);
}

#[tokio::test]
async fn activity_counts_ignores_other_workers() {
    let store = fresh_store().await;
    let mine = seed_worker(&store).await;
    let other = seed_worker(&store).await;
    let sink = AuditSink::new(store.clone());
    sink.write(Some(other.session), AuditKind::AssistantMessage, serde_json::json!({}), None)
        .await
        .unwrap();
    let counts = activity_counts_for_worker(&store, mine.worker, Utc::now())
        .await
        .unwrap();
    assert_eq!(counts, ActivityCounts::default());
}

#[tokio::test]
async fn activity_counts_drops_events_outside_24h_window() {
    let store = fresh_store().await;
    let seed = seed_worker(&store).await;
    let sink = AuditSink::new(seed.store.clone());
    sink.write(Some(seed.session), AuditKind::ToolCall, serde_json::json!({}), None)
        .await
        .unwrap();
    // Query as if 48 hours have passed — the seeded row is now
    // outside the 24-hour lookback.
    let future = Utc::now() + chrono::Duration::hours(48);
    let counts = activity_counts_for_worker(&store, seed.worker, future)
        .await
        .unwrap();
    assert_eq!(counts.tool_calls, 0);
}

#[tokio::test]
async fn generate_team_headline_no_activity_returns_single_fallback() {
    let store = fresh_store().await;
    let seed = seed_worker(&store).await;
    let headline = generate_team_headline(seed.worker, "gtm-engineer", &seed.store)
        .await
        .unwrap();
    assert_eq!(headline.bullets, vec!["No activity yet.".to_string()]);
    assert!(headline.tomorrow.is_none());
}

#[tokio::test]
async fn generate_team_headline_renders_pr_checker_bullets() {
    let store = fresh_store().await;
    let seed = seed_worker(&store).await;
    let sink = AuditSink::new(seed.store.clone());
    for _ in 0..6 {
        sink.write(Some(seed.session), AuditKind::ToolCall, serde_json::json!({}), None)
            .await
            .unwrap();
    }
    for _ in 0..2 {
        sink.write(Some(seed.session), AuditKind::AssistantMessage, serde_json::json!({}), None)
            .await
            .unwrap();
    }
    let headline = generate_team_headline(seed.worker, "pr-checker", &seed.store)
        .await
        .unwrap();
    assert!(headline.bullets.iter().any(|b| b == "6 PRs reviewed today"));
    assert!(headline.bullets.iter().any(|b| b == "2 issues flagged"));
    assert!(headline.tomorrow.is_none());
}

#[tokio::test]
async fn next_duty_returns_none_when_role_duty_table_missing() {
    let store = fresh_store().await;
    let seed = seed_worker(&store).await;
    let tomorrow = next_duty_for_worker(&seed.store, seed.worker, Utc::now())
        .await
        .unwrap();
    assert!(tomorrow.is_none());
}

#[tokio::test]
async fn next_duty_returns_next_fire_within_24h() {
    let store = fresh_store().await;
    let seed = seed_worker(&store).await;
    ensure_role_duty_schema(&seed.store).await;
    insert_duty(&seed, "d-1", "0 30 6 * * * *", "morning routine", true, None).await;
    let line = next_duty_for_worker(&seed.store, seed.worker, Utc::now())
        .await
        .unwrap()
        .expect("morning routine should be scheduled");
    assert!(line.starts_with("morning routine at 06:30"), "got {line}");
}

#[tokio::test]
async fn next_duty_skips_disabled_and_paused_rows() {
    let store = fresh_store().await;
    let seed = seed_worker(&store).await;
    ensure_role_duty_schema(&seed.store).await;
    let now = Utc::now();
    insert_duty(&seed, "d-off", "0 0 6 * * * *", "disabled duty", false, None).await;
    insert_duty(
        &seed,
        "d-pause",
        "0 0 6 * * * *",
        "paused duty",
        true,
        Some(now + chrono::Duration::hours(2)),
    )
    .await;
    let tomorrow = next_duty_for_worker(&seed.store, seed.worker, now)
        .await
        .unwrap();
    assert!(tomorrow.is_none(), "got {tomorrow:?}");
}

#[tokio::test]
async fn generate_team_headline_threads_tomorrow_line() {
    let store = fresh_store().await;
    let seed = seed_worker(&store).await;
    ensure_role_duty_schema(&seed.store).await;
    insert_duty(&seed, "d-2", "0 0 9 * * * *", "standup", true, None).await;
    let headline = generate_team_headline(seed.worker, "gtm-engineer", &seed.store)
        .await
        .unwrap();
    assert_eq!(headline.bullets, vec!["No activity yet.".to_string()]);
    let tomorrow = headline.tomorrow.expect("tomorrow line present");
    assert!(tomorrow.starts_with("standup at 09:00"), "got {tomorrow}");
}
