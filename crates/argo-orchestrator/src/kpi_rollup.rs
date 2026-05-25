//! KPI rollup scheduler — derive per-worker KPI JSON from the audit
//! trail and persist into `worker.kpi_json` via
//! [`IdentityStore::set_kpis`] (Stream
//! `mission-kpi-rollup-from-audit-trail`, mission claim 2 evidence).
//!
//! The Mariah-card / `/workers/{id}` UI binds to `Worker::kpis`. Today
//! every worker stores `kpis: None` at hire and never advances; this
//! scheduler ticks once every 5 minutes (default), enumerates each
//! `(tenant_id, worker_id)` whose audit-row watermark advanced since
//! the last tick, runs 5 aggregation queries, persists the resulting
//! JSON, and emits one [`AuditKind::WorkerKpiRolled`] row per worker
//! rolled.
//!
//! All audit-row queries are tenant-bound via the `audit_record.tenant_id`
//! denorm column (migration v65) and `audit_record.worker_id`
//! (migration v67 / raw filename 77). The `kpis_rolled_at` watermark
//! lives on the `worker` row (migration v82).
//!
//! ```
//! use argo_orchestrator::kpi_rollup::KPI_ROLLUP_SCHEMA_VERSION;
//! assert_eq!(KPI_ROLLUP_SCHEMA_VERSION, 1);
//! ```

use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::Row;
use tokio::task::JoinHandle;
use uuid::Uuid;

use argo_audit::{AuditKind, AuditSink};
use argo_identity::{IdentityStore, WorkerId};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, TenantId};

/// On-disk JSON schema version persisted into `kpis.rollup_schema_version`.
/// Consumers MAY read newer schemas and ignore unknown fields; rollers
/// always emit the current version.
pub const KPI_ROLLUP_SCHEMA_VERSION: i64 = 1;

/// Default rollup cadence (5 minutes). Overridable from the gateway
/// boot via `ARGO_KPI_ROLLUP_SECS`.
pub const DEFAULT_KPI_ROLLUP_INTERVAL_SECS: u64 = 300;

const ONE_DAY_MICROS: i64 = 24 * 60 * 60 * 1_000_000;
const KIND_APPROVAL_ASKED: &str = "approval.asked";
const KIND_APPROVAL_REPLIED: &str = "approval.replied";
const KIND_TURN_START: &str = "turn_start";
const KIND_TOOL_CALL: &str = "tool_call";

/// On-disk KPI payload shape persisted into `worker.kpi_json` and
/// echoed into the [`AuditKind::WorkerKpiRolled`] audit row.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KpiPayload {
    /// Count of `approval.replied` rows whose `action_id` matches an
    /// `approval.asked` row attributed to this worker.
    pub approvals_handled: i64,
    /// Approval asks attributed to this worker that ended without a
    /// human reply, divided by all asks. `0.0` when no asks are
    /// recorded.
    pub auto_approval_rate: f64,
    /// Mean wall-clock seconds between the matched `approval.asked`
    /// `created_at` and `approval.replied` `created_at`. `0.0` when no
    /// pairs are recorded.
    pub mean_response_time_seconds: f64,
    /// Distinct `session_id` count for `turn_start` rows in the last
    /// 24 hours.
    pub sessions_24h: i64,
    /// `tool_call` row count for this worker in the last 24 hours.
    pub tool_calls_24h: i64,
    /// RFC-3339 UTC stamp of the tick that produced this snapshot.
    pub rollup_window_end_utc: String,
    /// See [`KPI_ROLLUP_SCHEMA_VERSION`].
    pub rollup_schema_version: i64,
}

/// One tick's outcome. `workers_rolled` is the number of distinct
/// `(tenant_id, worker_id)` tuples persisted; `rows_scanned` is the
/// sum of audit rows the SELECTs touched, surfaced into the
/// `WorkerKpiRolled` row's payload for forensic reads.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct KpiRollupReport {
    /// Number of workers whose `worker.kpi_json` was persisted on
    /// this tick.
    pub workers_rolled: usize,
    /// Total audit rows scanned across this tick's aggregation
    /// queries.
    pub rows_scanned: i64,
}

/// Periodic KPI rollup scheduler. Sibling of
/// [`crate::DutyScheduler`]; the gateway holds an `Arc` for the
/// process lifetime.
pub struct KpiRollupScheduler {
    store: Arc<SessionStore>,
    audit_sink: Arc<AuditSink>,
}

impl KpiRollupScheduler {
    /// Build a scheduler over the session store + audit sink. The
    /// sink's bound tenant id is ignored — every row is written
    /// under the worker's own tenant via
    /// [`AuditSink::write_with_tenant`].
    #[must_use]
    pub fn new(store: Arc<SessionStore>, audit_sink: Arc<AuditSink>) -> Self {
        Self { store, audit_sink }
    }

    /// Run one tick. Enumerates due workers (those with audit
    /// activity strictly after their `worker.kpis_rolled_at`
    /// watermark, or with the watermark NULL), aggregates their
    /// 5 KPIs, persists the JSON, advances the watermark, and emits
    /// one `WorkerKpiRolled` audit row per worker.
    pub async fn tick(&self, now: DateTime<Utc>) -> ArgoResult<KpiRollupReport> {
        let identity = IdentityStore::new(&self.store);
        identity.ensure_schema().await?;

        let due = list_due_workers(&self.store).await?;
        let mut report = KpiRollupReport::default();
        let now_micros = now.timestamp_micros();
        for (tenant_id_str, worker_id_str) in due {
            let (payload, rows_scanned) =
                compute_kpis(&self.store, &tenant_id_str, &worker_id_str, now, now_micros).await?;
            let tenant = Uuid::parse_str(&tenant_id_str).map(TenantId).map_err(|e| {
                ArgoError::Schema(format!("kpi rollup tenant_id parse '{tenant_id_str}': {e}"))
            })?;
            let worker = WorkerId::from_str(&worker_id_str).map_err(|e| {
                ArgoError::Schema(format!("kpi rollup worker_id parse '{worker_id_str}': {e}"))
            })?;
            let kpis_value = serde_json::to_value(&payload)
                .map_err(|e| ArgoError::Schema(format!("kpi payload serialize: {e}")))?;
            identity.set_kpis(tenant, worker, &kpis_value).await?;
            mark_rolled(&self.store, &tenant_id_str, &worker_id_str, now_micros).await?;
            self.audit_sink
                .write_with_tenant(
                    None,
                    AuditKind::WorkerKpiRolled,
                    json!({
                        "worker_id": worker_id_str,
                        "tenant_id": tenant_id_str,
                        "kpis_json": kpis_value,
                        "rolled_at": now.to_rfc3339(),
                        "rows_scanned": rows_scanned,
                    }),
                    None,
                    &tenant_id_str,
                )
                .await?;
            report.workers_rolled += 1;
            report.rows_scanned += rows_scanned;
        }
        Ok(report)
    }
}

/// Spawn a background tokio task that ticks the scheduler every
/// `interval_duration`. The returned `JoinHandle` is normally held
/// by `AbortOnDrop` in the gateway boot module so graceful
/// shutdown aborts the loop.
pub fn spawn_kpi_rollup_scheduler(
    scheduler: Arc<KpiRollupScheduler>,
    interval_duration: Duration,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(interval_duration);
        // First tick fires immediately; skip it so boot doesn't race
        // schema migration.
        ticker.tick().await;
        loop {
            ticker.tick().await;
            if let Err(e) = scheduler.tick(Utc::now()).await {
                tracing::warn!(
                    target: "orchestrator::kpi_rollup",
                    error = %e,
                    "kpi rollup tick failed; will retry on next interval"
                );
            }
        }
    })
}

async fn list_due_workers(store: &SessionStore) -> ArgoResult<Vec<(String, String)>> {
    // A worker is "due" when at least one audit_record row for this
    // (tenant, worker) tuple has created_at strictly greater than the
    // worker's kpis_rolled_at watermark — or the watermark is NULL.
    // The inner EXISTS is the tenant-bound guard.
    let rows = sqlx::query(
        "SELECT w.tenant_id AS tenant_id, w.id AS worker_id \
         FROM worker w \
         WHERE EXISTS ( \
             SELECT 1 FROM audit_record a \
             WHERE a.tenant_id = w.tenant_id \
               AND a.worker_id = w.id \
               AND (w.kpis_rolled_at IS NULL OR a.created_at > w.kpis_rolled_at) \
         )",
    )
    .fetch_all(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("list_due_workers: {e}")))?;
    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let tenant: String = row
            .try_get("tenant_id")
            .map_err(|e| ArgoError::Provider(format!("list_due_workers tenant_id: {e}")))?;
        let worker: String = row
            .try_get("worker_id")
            .map_err(|e| ArgoError::Provider(format!("list_due_workers worker_id: {e}")))?;
        out.push((tenant, worker));
    }
    Ok(out)
}

async fn compute_kpis(
    store: &SessionStore,
    tenant_id: &str,
    worker_id: &str,
    now: DateTime<Utc>,
    now_micros: i64,
) -> ArgoResult<(KpiPayload, i64)> {
    let window_start = now_micros - ONE_DAY_MICROS;

    let asks: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_record \
         WHERE tenant_id = ? AND worker_id = ? AND kind = ?",
    )
    .bind(tenant_id)
    .bind(worker_id)
    .bind(KIND_APPROVAL_ASKED)
    .fetch_one(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("count approval.asked: {e}")))?;

    let replied: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_record \
         WHERE tenant_id = ? AND kind = ? \
           AND json_extract(payload_json, '$.action_id') IN ( \
               SELECT json_extract(payload_json, '$.action_id') \
               FROM audit_record \
               WHERE tenant_id = ? AND worker_id = ? AND kind = ? \
           )",
    )
    .bind(tenant_id)
    .bind(KIND_APPROVAL_REPLIED)
    .bind(tenant_id)
    .bind(worker_id)
    .bind(KIND_APPROVAL_ASKED)
    .fetch_one(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("count approval.replied: {e}")))?;

    let mean_response_micros: Option<f64> = sqlx::query_scalar(
        "SELECT AVG(replied.created_at - asked.created_at) \
         FROM audit_record asked \
         INNER JOIN audit_record replied \
           ON replied.tenant_id = asked.tenant_id \
          AND replied.kind = ? \
          AND json_extract(replied.payload_json, '$.action_id') \
              = json_extract(asked.payload_json, '$.action_id') \
         WHERE asked.tenant_id = ? \
           AND asked.worker_id = ? \
           AND asked.kind = ?",
    )
    .bind(KIND_APPROVAL_REPLIED)
    .bind(tenant_id)
    .bind(worker_id)
    .bind(KIND_APPROVAL_ASKED)
    .fetch_one(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("avg response time: {e}")))?;

    let sessions_24h: i64 = sqlx::query_scalar(
        "SELECT COUNT(DISTINCT session_id) FROM audit_record \
         WHERE tenant_id = ? AND worker_id = ? AND kind = ? \
           AND created_at > ?",
    )
    .bind(tenant_id)
    .bind(worker_id)
    .bind(KIND_TURN_START)
    .bind(window_start)
    .fetch_one(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("count sessions_24h: {e}")))?;

    let tool_calls_24h: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_record \
         WHERE tenant_id = ? AND worker_id = ? AND kind = ? \
           AND created_at > ?",
    )
    .bind(tenant_id)
    .bind(worker_id)
    .bind(KIND_TOOL_CALL)
    .bind(window_start)
    .fetch_one(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("count tool_calls_24h: {e}")))?;

    let auto_approval_rate = if asks > 0 {
        let auto_resolved = (asks - replied).max(0);
        auto_resolved as f64 / asks as f64
    } else {
        0.0
    };
    let mean_response_time_seconds = mean_response_micros.map(|m| m / 1_000_000.0).unwrap_or(0.0);

    let payload = KpiPayload {
        approvals_handled: replied,
        auto_approval_rate,
        mean_response_time_seconds,
        sessions_24h,
        tool_calls_24h,
        rollup_window_end_utc: now.to_rfc3339(),
        rollup_schema_version: KPI_ROLLUP_SCHEMA_VERSION,
    };

    let rows_scanned = asks + replied + sessions_24h + tool_calls_24h;
    Ok((payload, rows_scanned))
}

async fn mark_rolled(
    store: &SessionStore,
    tenant_id: &str,
    worker_id: &str,
    now_micros: i64,
) -> ArgoResult<()> {
    sqlx::query(
        "UPDATE worker SET kpis_rolled_at = ? \
         WHERE id = ? AND tenant_id = ?",
    )
    .bind(now_micros)
    .bind(worker_id)
    .bind(tenant_id)
    .execute(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("mark kpis_rolled_at: {e}")))?;
    Ok(())
}
