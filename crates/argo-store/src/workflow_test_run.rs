//! Workflow test-run persistence (Stream
//! `claude-followup-pr-2021-test-run-gateway-persist`, re-author of
//! closed PR #2021).
//!
//! #1971 landed the NDJSON streaming endpoint that drives the
//! builder's "Test run" overlay; the tick-24 history panel keeps the
//! per-browser run list in `localStorage`. This module adds the
//! server-side history: one row per completed run, tenant-scoped via
//! the canonical pattern (every read filters on `tenant_id` first,
//! cross-tenant lookups return `Ok(vec![])` so the gateway returns an
//! empty list rather than a 404 / leak of existence).
//!
//! Column shape matches `useTestRun.ts::CompletedTestRun`:
//! `started_at` / `ended_at` are Unix-millis numbers; `status` is the
//! hook's run phase (`"complete"` | `"failed"`); `summary_json` holds
//! the final `RunSummary` blob; `step_states_json` is the per-step
//! `Record<stepId, StepRunState>` snapshot; `transport_error` is the
//! hook's `transportError` string (non-null on stream failure).

use argo_types::{ArgoError, ArgoResult, TenantId};
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Row, Sqlite};
use uuid::Uuid;

/// Strongly-typed handle for one persisted test run.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorkflowTestRunId(pub String);

impl WorkflowTestRunId {
    /// Fresh id, generated as a v7 UUID.
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::now_v7().to_string())
    }

    /// View as a borrowed `&str` for sqlx bindings.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for WorkflowTestRunId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for WorkflowTestRunId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Insert payload — mirrors `useTestRun.ts::CompletedTestRun` 1:1
/// (timestamps in Unix milliseconds; `status` is the run-phase string;
/// summary/step-state blobs are caller-supplied opaque JSON).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowTestRunRecord {
    /// `"complete"` | `"failed"`. The CRUD layer accepts any non-empty
    /// string so callers can extend later; the gateway enforces the
    /// closed set.
    pub status: String,
    /// Unix milliseconds when the run started.
    pub started_at: i64,
    /// Unix milliseconds when the run ended (terminal phase).
    pub ended_at: i64,
    /// Opaque JSON-encoded `RunSummary` (`'null'` when the run aborted
    /// before a `run:summary` event arrived).
    pub summary_json: String,
    /// Opaque JSON object — `Record<stepId, StepRunState>` snapshot.
    pub step_states_json: String,
    /// Non-null when the NDJSON fetch failed mid-stream.
    pub transport_error: Option<String>,
}

/// One full row from `workflow_test_run`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkflowTestRun {
    /// Stable row id.
    pub id: WorkflowTestRunId,
    /// Tenant that owns this run row.
    pub tenant_id: TenantId,
    /// Vertical id (e.g. `pr_checker`).
    pub vertical_id: String,
    /// Workflow name within the vertical.
    pub workflow_name: String,
    /// Unix milliseconds when the run started.
    pub started_at: i64,
    /// Unix milliseconds when the run ended.
    pub ended_at: i64,
    /// `"complete"` | `"failed"`.
    pub status: String,
    /// Opaque JSON-encoded `RunSummary` blob, byte-identical to what
    /// the caller supplied (`'null'` when the run had no summary).
    pub summary_json: String,
    /// Opaque JSON object — `Record<stepId, StepRunState>` snapshot.
    pub step_states_json: String,
    /// Non-null when the NDJSON fetch failed mid-stream.
    pub transport_error: Option<String>,
}

/// Persist one completed test run. Returns the freshly-minted id so
/// callers (and downstream audit rows) can reference the row.
///
/// # Errors
///
/// - [`ArgoError::Schema`] — required string fields are empty or
///   timestamps are non-positive.
/// - [`ArgoError::Provider`] — sqlite write failures.
pub async fn record(
    pool: &Pool<Sqlite>,
    tenant_id: TenantId,
    vertical_id: &str,
    workflow_name: &str,
    record: &WorkflowTestRunRecord,
) -> ArgoResult<WorkflowTestRunId> {
    if vertical_id.is_empty() {
        return Err(ArgoError::Schema(
            "workflow_test_run: vertical_id must not be empty".into(),
        ));
    }
    if workflow_name.is_empty() {
        return Err(ArgoError::Schema(
            "workflow_test_run: workflow_name must not be empty".into(),
        ));
    }
    if record.status.is_empty() {
        return Err(ArgoError::Schema(
            "workflow_test_run: status must not be empty".into(),
        ));
    }
    if record.started_at <= 0 || record.ended_at <= 0 {
        return Err(ArgoError::Schema(
            "workflow_test_run: started_at and ended_at must be positive Unix-millis".into(),
        ));
    }
    if record.summary_json.is_empty() {
        return Err(ArgoError::Schema(
            "workflow_test_run: summary_json must not be empty (use 'null')".into(),
        ));
    }
    if record.step_states_json.is_empty() {
        return Err(ArgoError::Schema(
            "workflow_test_run: step_states_json must not be empty (use '{}')".into(),
        ));
    }

    let id = WorkflowTestRunId::new();
    let tenant_str = tenant_id.to_string();
    sqlx::query(
        "INSERT INTO workflow_test_run \
            (id, tenant_id, vertical_id, workflow_name, started_at, ended_at, \
             status, summary_json, step_states_json, transport_error) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(id.as_str())
    .bind(&tenant_str)
    .bind(vertical_id)
    .bind(workflow_name)
    .bind(record.started_at)
    .bind(record.ended_at)
    .bind(&record.status)
    .bind(&record.summary_json)
    .bind(&record.step_states_json)
    .bind(record.transport_error.as_deref())
    .execute(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("workflow_test_run insert: {e}")))?;
    Ok(id)
}

/// Newest-first list of up to `limit` runs for `(tenant, vertical,
/// workflow_name)`. Empty when no rows exist OR the caller is from a
/// different tenant — the gateway maps both to an empty `items` list
/// so cross-tenant callers cannot probe existence.
///
/// `limit` is clamped to `[1, 100]`.
///
/// # Errors
///
/// - [`ArgoError::Provider`] — sqlite read failures.
pub async fn list_recent(
    pool: &Pool<Sqlite>,
    tenant_id: TenantId,
    vertical_id: &str,
    workflow_name: &str,
    limit: i64,
) -> ArgoResult<Vec<WorkflowTestRun>> {
    let tenant_str = tenant_id.to_string();
    let clamped = limit.clamp(1, 100);
    let rows = sqlx::query(
        "SELECT id, tenant_id, vertical_id, workflow_name, started_at, ended_at, \
                status, summary_json, step_states_json, transport_error \
         FROM workflow_test_run \
         WHERE tenant_id = ? AND vertical_id = ? AND workflow_name = ? \
         ORDER BY ended_at DESC \
         LIMIT ?",
    )
    .bind(&tenant_str)
    .bind(vertical_id)
    .bind(workflow_name)
    .bind(clamped)
    .fetch_all(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("workflow_test_run list_recent: {e}")))?;
    rows.into_iter().map(decode_row).collect()
}

fn decode_row(row: sqlx::sqlite::SqliteRow) -> ArgoResult<WorkflowTestRun> {
    let id: String = row
        .try_get("id")
        .map_err(|e| ArgoError::Provider(format!("workflow_test_run decode id: {e}")))?;
    let tenant_str: String = row
        .try_get("tenant_id")
        .map_err(|e| ArgoError::Provider(format!("workflow_test_run decode tenant_id: {e}")))?;
    let tenant_uuid = Uuid::parse_str(&tenant_str).map_err(|e| {
        ArgoError::Provider(format!(
            "workflow_test_run decode tenant_id '{tenant_str}': {e}"
        ))
    })?;
    Ok(WorkflowTestRun {
        id: WorkflowTestRunId(id),
        tenant_id: TenantId(tenant_uuid),
        vertical_id: row
            .try_get("vertical_id")
            .map_err(|e| ArgoError::Provider(format!("workflow_test_run decode vertical_id: {e}")))?,
        workflow_name: row.try_get("workflow_name").map_err(|e| {
            ArgoError::Provider(format!("workflow_test_run decode workflow_name: {e}"))
        })?,
        started_at: row.try_get("started_at").map_err(|e| {
            ArgoError::Provider(format!("workflow_test_run decode started_at: {e}"))
        })?,
        ended_at: row
            .try_get("ended_at")
            .map_err(|e| ArgoError::Provider(format!("workflow_test_run decode ended_at: {e}")))?,
        status: row
            .try_get("status")
            .map_err(|e| ArgoError::Provider(format!("workflow_test_run decode status: {e}")))?,
        summary_json: row.try_get("summary_json").map_err(|e| {
            ArgoError::Provider(format!("workflow_test_run decode summary_json: {e}"))
        })?,
        step_states_json: row.try_get("step_states_json").map_err(|e| {
            ArgoError::Provider(format!("workflow_test_run decode step_states_json: {e}"))
        })?,
        transport_error: row.try_get("transport_error").map_err(|e| {
            ArgoError::Provider(format!("workflow_test_run decode transport_error: {e}"))
        })?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SessionStore;

    async fn fresh_pool() -> Pool<Sqlite> {
        let store = SessionStore::open(":memory:").await.unwrap();
        store.migrate().await.unwrap();
        store.pool().clone()
    }

    fn sample_record() -> WorkflowTestRunRecord {
        WorkflowTestRunRecord {
            status: "complete".to_string(),
            started_at: 1_716_336_600_000,
            ended_at: 1_716_336_605_000,
            summary_json: r#"{"ok":3,"fail":0,"durationMs":5000}"#.to_string(),
            step_states_json: r#"{"s1":{"status":"success"}}"#.to_string(),
            transport_error: None,
        }
    }

    #[tokio::test]
    async fn record_then_list_round_trip() {
        let pool = fresh_pool().await;
        let tenant = TenantId::new();
        let rec = sample_record();
        let id = record(&pool, tenant, "support", "greet", &rec)
            .await
            .unwrap();
        let rows = list_recent(&pool, tenant, "support", "greet", 10)
            .await
            .unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id, id);
        assert_eq!(rows[0].tenant_id, tenant);
        assert_eq!(rows[0].vertical_id, "support");
        assert_eq!(rows[0].workflow_name, "greet");
        assert_eq!(rows[0].status, "complete");
        assert_eq!(rows[0].started_at, 1_716_336_600_000);
        assert_eq!(rows[0].ended_at, 1_716_336_605_000);
        assert!(rows[0].transport_error.is_none());
        assert_eq!(
            rows[0].summary_json,
            r#"{"ok":3,"fail":0,"durationMs":5000}"#
        );
    }

    #[tokio::test]
    async fn list_recent_orders_newest_first() {
        let pool = fresh_pool().await;
        let tenant = TenantId::new();
        let mut rec = sample_record();

        rec.ended_at = 1_716_336_000_000;
        record(&pool, tenant, "support", "greet", &rec)
            .await
            .unwrap();

        rec.ended_at = 1_716_336_900_000;
        record(&pool, tenant, "support", "greet", &rec)
            .await
            .unwrap();

        rec.ended_at = 1_716_337_500_000;
        record(&pool, tenant, "support", "greet", &rec)
            .await
            .unwrap();

        let rows = list_recent(&pool, tenant, "support", "greet", 10)
            .await
            .unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].ended_at, 1_716_337_500_000);
        assert_eq!(rows[1].ended_at, 1_716_336_900_000);
        assert_eq!(rows[2].ended_at, 1_716_336_000_000);
    }

    #[tokio::test]
    async fn list_recent_respects_limit() {
        let pool = fresh_pool().await;
        let tenant = TenantId::new();
        let rec = sample_record();
        for _ in 0..5 {
            record(&pool, tenant, "support", "greet", &rec)
                .await
                .unwrap();
        }
        let rows = list_recent(&pool, tenant, "support", "greet", 2)
            .await
            .unwrap();
        assert_eq!(rows.len(), 2);
    }

    #[tokio::test]
    async fn list_recent_clamps_above_max() {
        let pool = fresh_pool().await;
        let tenant = TenantId::new();
        let rec = sample_record();
        record(&pool, tenant, "support", "greet", &rec)
            .await
            .unwrap();
        // 1_000 should clamp to 100; we only inserted 1 row so we still
        // get 1 — the assertion is that the call succeeds without
        // sqlite rejecting the LIMIT value.
        let rows = list_recent(&pool, tenant, "support", "greet", 1_000)
            .await
            .unwrap();
        assert_eq!(rows.len(), 1);
    }

    #[tokio::test]
    async fn cross_tenant_lookup_returns_empty() {
        let pool = fresh_pool().await;
        let tenant_a = TenantId::new();
        let tenant_b = TenantId::new();
        let rec = sample_record();
        record(&pool, tenant_a, "support", "greet", &rec)
            .await
            .unwrap();

        let rows = list_recent(&pool, tenant_b, "support", "greet", 10)
            .await
            .unwrap();
        assert!(rows.is_empty(), "tenant B must not see tenant A's runs");
    }

    #[tokio::test]
    async fn list_recent_returns_empty_for_unknown_workflow() {
        let pool = fresh_pool().await;
        let tenant = TenantId::new();
        // No insert — directly query for an unknown (vertical, name).
        let rows = list_recent(&pool, tenant, "support", "no_such_workflow", 10)
            .await
            .unwrap();
        assert!(rows.is_empty(), "unknown workflow must list as empty");
    }

    #[tokio::test]
    async fn record_rejects_empty_vertical() {
        let pool = fresh_pool().await;
        let tenant = TenantId::new();
        let rec = sample_record();
        let err = record(&pool, tenant, "", "greet", &rec).await.unwrap_err();
        assert!(matches!(err, ArgoError::Schema(_)));
    }

    #[tokio::test]
    async fn record_rejects_non_positive_timestamps() {
        let pool = fresh_pool().await;
        let tenant = TenantId::new();
        let mut rec = sample_record();
        rec.started_at = 0;
        let err = record(&pool, tenant, "support", "greet", &rec)
            .await
            .unwrap_err();
        assert!(matches!(err, ArgoError::Schema(_)));
    }

    #[tokio::test]
    async fn migration_is_idempotent_on_second_invocation() {
        let store = SessionStore::open(":memory:").await.unwrap();
        store.migrate().await.unwrap();
        // A second `migrate()` should be a no-op against an
        // already-up-to-date schema — confirming the v78 wrapper does
        // not re-run its CREATE statements.
        store.migrate().await.unwrap();
        let pool = store.pool().clone();
        let tenant = TenantId::new();
        let rec = sample_record();
        record(&pool, tenant, "support", "greet", &rec)
            .await
            .unwrap();
        let rows = list_recent(&pool, tenant, "support", "greet", 10)
            .await
            .unwrap();
        assert_eq!(rows.len(), 1);
    }

    #[tokio::test]
    async fn record_with_transport_error_round_trips() {
        let pool = fresh_pool().await;
        let tenant = TenantId::new();
        let mut rec = sample_record();
        rec.status = "failed".to_string();
        rec.transport_error = Some("network: connection reset".to_string());
        record(&pool, tenant, "support", "greet", &rec)
            .await
            .unwrap();
        let rows = list_recent(&pool, tenant, "support", "greet", 10)
            .await
            .unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(
            rows[0].transport_error.as_deref(),
            Some("network: connection reset")
        );
        assert_eq!(rows[0].status, "failed");
    }
}
