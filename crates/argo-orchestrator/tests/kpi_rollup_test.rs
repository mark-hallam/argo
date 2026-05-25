//! Regression tests for the periodic KPI rollup scheduler
//! (Stream `mission-kpi-rollup-from-audit-trail`, mission claim 2).
//!
//! The brief authors a 5-test acceptance suite that pins:
//!   1. `approvals_handled` advances when matched asked/replied pairs
//!      land in the audit trail.
//!   2. Cross-tenant isolation: each tenant's worker rolls only its
//!      own audit rows.
//!   3. One `WorkerKpiRolled` audit row per worker rolled per tick.
//!   4. The 24h window drops stale `tool_call` rows.
//!   5. Workers with no new audit activity since the last rollup are
//!      skipped on the next tick (idempotent watermark).

use std::sync::Arc;

use argo_audit::AuditSink;
use argo_identity::{IdentityStore, Persona, Role, Worker, WorkerId};
use argo_orchestrator::KpiRollupScheduler;
use argo_store::SessionStore;
use argo_types::TenantId;
use chrono::{TimeZone, Utc};
use uuid::Uuid;

const MICROS_PER_SECOND: i64 = 1_000_000;

async fn fresh_world() -> (Arc<SessionStore>, Arc<AuditSink>) {
    let store = Arc::new(SessionStore::open(":memory:").await.unwrap());
    store.migrate().await.unwrap();
    let sink = Arc::new(AuditSink::new_with_tenant(
        Arc::clone(&store),
        "test".into(),
    ));
    (store, sink)
}

async fn hire_worker(store: &SessionStore, tenant: TenantId) -> WorkerId {
    let identity = IdentityStore::new(store);
    identity.ensure_schema().await.unwrap();
    let worker = Worker::draft(
        tenant,
        "support",
        Persona::named("Mariah", "ops"),
        Role::new("Triage."),
    );
    let id = worker.id;
    identity.create_worker(&worker).await.unwrap();
    id
}

async fn seed_audit_row(
    store: &SessionStore,
    tenant_id: &str,
    worker_id: Option<&str>,
    session_id: Option<Uuid>,
    kind: &str,
    payload: serde_json::Value,
    created_at_micros: i64,
) {
    let id = Uuid::now_v7();
    let payload_str = payload.to_string();
    sqlx::query(
        "INSERT INTO audit_record \
         (id, session_id, kind, payload_json, parent_hash, self_hash, \
          created_at, principal_id, tenant_id, worker_id, channel, \
          action_type, outcome_status, approver_id, cost_band, \
          data_classification) \
         VALUES (?, ?, ?, ?, x'00', x'01', ?, NULL, ?, ?, NULL, NULL, \
                 NULL, NULL, NULL, NULL)",
    )
    .bind(id.as_bytes().as_slice())
    .bind(session_id.map(|s| s.as_bytes().to_vec()))
    .bind(kind)
    .bind(&payload_str)
    .bind(created_at_micros)
    .bind(tenant_id)
    .bind(worker_id)
    .execute(store.pool())
    .await
    .unwrap();
}

async fn worker_kpi_json(store: &SessionStore, worker_id: WorkerId) -> Option<serde_json::Value> {
    let row: Option<String> = sqlx::query_scalar("SELECT kpi_json FROM worker WHERE id = ?")
        .bind(worker_id.to_string())
        .fetch_one(store.pool())
        .await
        .unwrap();
    row.and_then(|s| serde_json::from_str(&s).ok())
}

async fn count_kind_rows(store: &SessionStore, kind: &str) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM audit_record WHERE kind = ?")
        .bind(kind)
        .fetch_one(store.pool())
        .await
        .unwrap()
}

/// Test 1: replied count tracks the joined asked/replied pairs for
/// the worker.
#[tokio::test]
async fn rollup_increments_approvals_handled_after_replied() {
    let (store, sink) = fresh_world().await;
    let tenant = TenantId::new();
    let worker = hire_worker(&store, tenant).await;
    let tenant_str = tenant.0.to_string();
    let worker_str = worker.to_string();

    let now = Utc.with_ymd_and_hms(2026, 5, 25, 18, 0, 0).unwrap();
    let base_micros = now.timestamp_micros() - 60 * MICROS_PER_SECOND;
    for i in 0..3 {
        let action_id = format!("a-{i}");
        seed_audit_row(
            &store,
            &tenant_str,
            Some(&worker_str),
            None,
            "approval.asked",
            serde_json::json!({"action_id": action_id, "worker_id": worker_str}),
            base_micros + (i * 1_000_000),
        )
        .await;
        seed_audit_row(
            &store,
            &tenant_str,
            None,
            None,
            "approval.replied",
            serde_json::json!({"action_id": action_id, "reply_kind": "approve"}),
            base_micros + (i * 1_000_000) + 500_000,
        )
        .await;
    }

    let scheduler = KpiRollupScheduler::new(Arc::clone(&store), sink);
    let report = scheduler.tick(now).await.unwrap();
    assert_eq!(report.workers_rolled, 1);

    let kpis = worker_kpi_json(&store, worker).await.expect("kpis set");
    assert_eq!(kpis["approvals_handled"], 3);
    assert!(kpis["mean_response_time_seconds"].as_f64().unwrap() > 0.0);
}

/// Test 2: workers with the same logical name in different tenants
/// get their own KPI roll without cross-tenant bleed.
#[tokio::test]
async fn rollup_tenant_isolated() {
    let (store, sink) = fresh_world().await;
    let tenant_a = TenantId::new();
    let tenant_b = TenantId::new();
    let worker_a = hire_worker(&store, tenant_a).await;
    let worker_b = hire_worker(&store, tenant_b).await;
    let now = Utc.with_ymd_and_hms(2026, 5, 25, 18, 0, 0).unwrap();
    let base = now.timestamp_micros() - 30 * MICROS_PER_SECOND;

    // Tenant A: 2 tool calls.
    for i in 0..2 {
        seed_audit_row(
            &store,
            &tenant_a.0.to_string(),
            Some(&worker_a.to_string()),
            Some(Uuid::now_v7()),
            "tool_call",
            serde_json::json!({"worker_id": worker_a.to_string()}),
            base + i * MICROS_PER_SECOND,
        )
        .await;
    }
    // Tenant B: 5 tool calls.
    for i in 0..5 {
        seed_audit_row(
            &store,
            &tenant_b.0.to_string(),
            Some(&worker_b.to_string()),
            Some(Uuid::now_v7()),
            "tool_call",
            serde_json::json!({"worker_id": worker_b.to_string()}),
            base + i * MICROS_PER_SECOND,
        )
        .await;
    }

    let scheduler = KpiRollupScheduler::new(Arc::clone(&store), sink);
    let report = scheduler.tick(now).await.unwrap();
    assert_eq!(report.workers_rolled, 2);

    let kpis_a = worker_kpi_json(&store, worker_a).await.unwrap();
    let kpis_b = worker_kpi_json(&store, worker_b).await.unwrap();
    assert_eq!(kpis_a["tool_calls_24h"], 2);
    assert_eq!(kpis_b["tool_calls_24h"], 5);
}

/// Test 3: exactly one `WorkerKpiRolled` audit row per worker rolled
/// per tick.
#[tokio::test]
async fn rollup_emits_audit_row_per_worker_per_tick() {
    let (store, sink) = fresh_world().await;
    let now = Utc.with_ymd_and_hms(2026, 5, 25, 18, 0, 0).unwrap();
    let base = now.timestamp_micros() - 30 * MICROS_PER_SECOND;

    let mut workers = Vec::new();
    for i in 0..5 {
        let tenant = TenantId::new();
        let worker = hire_worker(&store, tenant).await;
        seed_audit_row(
            &store,
            &tenant.0.to_string(),
            Some(&worker.to_string()),
            Some(Uuid::now_v7()),
            "tool_call",
            serde_json::json!({"worker_id": worker.to_string()}),
            base + i * MICROS_PER_SECOND,
        )
        .await;
        workers.push((tenant, worker));
    }

    let scheduler = KpiRollupScheduler::new(Arc::clone(&store), Arc::clone(&sink));
    let report = scheduler.tick(now).await.unwrap();
    sink.flush_pending().await.unwrap();

    assert_eq!(report.workers_rolled, 5);
    assert_eq!(count_kind_rows(&store, "worker.kpi_rolled").await, 5);
}

/// Test 4: the 24h window drops stale `tool_call` rows.
#[tokio::test]
async fn rollup_24h_window_drops_stale_rows() {
    let (store, sink) = fresh_world().await;
    let tenant = TenantId::new();
    let worker = hire_worker(&store, tenant).await;
    let now = Utc.with_ymd_and_hms(2026, 5, 25, 18, 0, 0).unwrap();

    // Stale: 25 hours ago (outside the 24h window).
    seed_audit_row(
        &store,
        &tenant.0.to_string(),
        Some(&worker.to_string()),
        Some(Uuid::now_v7()),
        "tool_call",
        serde_json::json!({"worker_id": worker.to_string()}),
        now.timestamp_micros() - 25 * 60 * 60 * MICROS_PER_SECOND,
    )
    .await;
    // Fresh: 1 hour ago.
    seed_audit_row(
        &store,
        &tenant.0.to_string(),
        Some(&worker.to_string()),
        Some(Uuid::now_v7()),
        "tool_call",
        serde_json::json!({"worker_id": worker.to_string()}),
        now.timestamp_micros() - 60 * 60 * MICROS_PER_SECOND,
    )
    .await;

    let scheduler = KpiRollupScheduler::new(Arc::clone(&store), sink);
    let _ = scheduler.tick(now).await.unwrap();
    let kpis = worker_kpi_json(&store, worker).await.unwrap();
    assert_eq!(kpis["tool_calls_24h"], 1);
}

/// Test 5: idempotent watermark — a second tick with no new audit
/// activity emits zero new `WorkerKpiRolled` rows.
#[tokio::test]
async fn rollup_skipped_if_no_audit_activity_since_last_rollup() {
    let (store, sink) = fresh_world().await;
    let tenant = TenantId::new();
    let worker = hire_worker(&store, tenant).await;
    let now = Utc.with_ymd_and_hms(2026, 5, 25, 18, 0, 0).unwrap();
    seed_audit_row(
        &store,
        &tenant.0.to_string(),
        Some(&worker.to_string()),
        Some(Uuid::now_v7()),
        "tool_call",
        serde_json::json!({"worker_id": worker.to_string()}),
        now.timestamp_micros() - 5 * MICROS_PER_SECOND,
    )
    .await;

    let scheduler = KpiRollupScheduler::new(Arc::clone(&store), Arc::clone(&sink));
    let first = scheduler.tick(now).await.unwrap();
    sink.flush_pending().await.unwrap();
    assert_eq!(first.workers_rolled, 1);

    let later = now + chrono::Duration::seconds(60);
    let second = scheduler.tick(later).await.unwrap();
    sink.flush_pending().await.unwrap();
    assert_eq!(
        second.workers_rolled, 0,
        "no new audit rows since first tick"
    );
    assert_eq!(
        count_kind_rows(&store, "worker.kpi_rolled").await,
        1,
        "second tick must not write a second WorkerKpiRolled row"
    );
}

/// Test 6: persisting KPI JSON must not clobber the existing worker
/// charter markdown.
#[tokio::test]
async fn rollup_does_not_clobber_existing_charter() {
    let (store, sink) = fresh_world().await;
    let tenant = TenantId::new();
    let worker = hire_worker(&store, tenant).await;
    let charter = "Mariah keeps marketplace listings fresh.";
    let identity = IdentityStore::new(&store);
    identity
        .update_charter(tenant, worker, Some(charter), None)
        .await
        .unwrap();

    let before = identity.get_worker(tenant, worker).await.unwrap().unwrap();
    assert_eq!(before.charter.as_deref(), Some(charter));
    assert!(before.kpis.is_none(), "precondition: rollup writes KPIs");

    let now = Utc.with_ymd_and_hms(2026, 5, 25, 18, 0, 0).unwrap();
    seed_audit_row(
        &store,
        &tenant.0.to_string(),
        Some(&worker.to_string()),
        Some(Uuid::now_v7()),
        "tool_call",
        serde_json::json!({"worker_id": worker.to_string()}),
        now.timestamp_micros() - 5 * MICROS_PER_SECOND,
    )
    .await;

    let scheduler = KpiRollupScheduler::new(Arc::clone(&store), sink);
    let report = scheduler.tick(now).await.unwrap();
    assert_eq!(report.workers_rolled, 1);

    let after = identity.get_worker(tenant, worker).await.unwrap().unwrap();
    assert_eq!(after.charter.as_deref(), Some(charter));
    assert_eq!(after.kpis.unwrap()["tool_calls_24h"], 1);
}
