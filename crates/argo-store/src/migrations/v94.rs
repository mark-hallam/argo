//! Schema migration v94 — the `capability_request` table backing the
//! connect-on-demand grant card (Stream `tg1-emit-capability-request-on-
//! fenced-tool-deny`, tiered-gating backend).
//!
//! When a teammate's run hits a tool/connector its capability fence
//! denies, the deny was SILENT — the user never learned "Nora needed
//! Salesforce", so connect-on-demand could not happen. This table makes
//! the deny LEGIBLE: the runtime persists one pending row per
//! `(tenant, worker, tool)` at the deny point, and the SPA's grant card
//! (`option-G2-elevated`) reads it via
//! `GET /api/v1/workers/<id>/capability-requests`.
//!
//! ## Dedup + tenant isolation
//!
//! The `UNIQUE (tenant_id, worker_id, tool_or_connector)` constraint is
//! the dedup spine: a re-deny of the SAME tool bumps `last_seen_utc` on
//! the existing row (`ON CONFLICT … DO UPDATE`) rather than inserting a
//! duplicate, and tenant A's request key-space is disjoint from tenant
//! B's so a cross-tenant `worker_id` collision can never surface A's
//! request under B. `run_id` is nullable (a deny may fire outside a
//! tracked run); `status` defaults to `pending` and is advanced by the
//! grant flow (tg3), never by this emission path (observe-only).
//!
//! Columns are TEXT (not BLOB) because the runtime keys are string
//! forms: `tenant_id` is the `argo_config::workspace_tenant_id` string,
//! `worker_id` is the `WorkerId` UUID string, and `id` is a fresh UUID
//! string — same convention as v92 `approval_vote`.
//!
//! UP creates the table + one lookup index. Idempotent via
//! `IF NOT EXISTS`. DOWN drops the index then the table.
use argo_types::ArgoError;
use sqlx::Sqlite;

pub(super) async fn up<'a, 'b>(tx: &'a mut sqlx::Transaction<'b, Sqlite>) -> Result<(), ArgoError> {
    for sql in V94_UP {
        sqlx::query(sql)
            .execute(&mut **tx)
            .await
            .map_err(|e| ArgoError::Provider(format!("v94 up: {e}")))?;
    }
    Ok(())
}

pub(super) async fn down<'a, 'b>(
    tx: &'a mut sqlx::Transaction<'b, Sqlite>,
) -> Result<(), ArgoError> {
    for sql in V94_DOWN {
        sqlx::query(sql)
            .execute(&mut **tx)
            .await
            .map_err(|e| ArgoError::Provider(format!("v94 down: {e}")))?;
    }
    Ok(())
}

const V94_UP: &[&str] = &[
    "CREATE TABLE IF NOT EXISTS capability_request (
        id                TEXT PRIMARY KEY,
        tenant_id         TEXT NOT NULL,
        worker_id         TEXT NOT NULL,
        tool_or_connector TEXT NOT NULL,
        requested_access  TEXT NOT NULL,
        run_id            TEXT,
        status            TEXT NOT NULL DEFAULT 'pending',
        created_utc       TEXT NOT NULL,
        last_seen_utc     TEXT NOT NULL,
        UNIQUE (tenant_id, worker_id, tool_or_connector)
    )",
    "CREATE INDEX IF NOT EXISTS idx_capability_request_pending \
     ON capability_request(tenant_id, worker_id, status)",
];

const V94_DOWN: &[&str] = &[
    "DROP INDEX IF EXISTS idx_capability_request_pending",
    "DROP TABLE IF EXISTS capability_request",
];
