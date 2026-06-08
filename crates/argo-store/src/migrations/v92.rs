//! Schema migration v92 — the `approval_vote` table backing N-of-M
//! threshold approvals (Stream `n-of-m-multi-step-approval`, v1.1
//! governance).
//!
//! Approvals were single-approver: one operator reply settled an
//! action. A threshold approval declares `approval.threshold = N` plus
//! an optional `approval.approvers` allowlist in the action payload;
//! each approver's decision is recorded as one row here, and the action
//! stays HELD until `N` distinct approve rows exist. This is a **vote
//! ledger** mirroring the `approval_bundle_member` shape from v68.
//!
//! Tenant isolation + idempotency are both enforced by the
//! `UNIQUE (tenant_id, approval_request_id, approver)` constraint:
//!   * the same approver voting twice is a no-op (`INSERT OR IGNORE`),
//!     so a double-click never counts twice toward the threshold; and
//!   * tenant A's votes are a different key-space from tenant B's, so a
//!     cross-tenant approval_request_id collision can never let B's vote
//!     count toward A's threshold.
//!
//! Columns are TEXT (not BLOB) because the runtime keys are string
//! forms: `approval_request_id` is the `ActionRecord.action_id` string,
//! `approver` is the resolved settle actor, and `tenant_id` is the
//! `argo_config::workspace_tenant_id` string (same convention as v91).
//!
//! UP creates the table + one lookup index. Idempotent via
//! `IF NOT EXISTS`. DOWN drops the index then the table.
use argo_types::ArgoError;
use sqlx::Sqlite;

pub(super) async fn up<'a, 'b>(tx: &'a mut sqlx::Transaction<'b, Sqlite>) -> Result<(), ArgoError> {
    for sql in V92_UP {
        sqlx::query(sql)
            .execute(&mut **tx)
            .await
            .map_err(|e| ArgoError::Provider(format!("v92 up: {e}")))?;
    }
    Ok(())
}

pub(super) async fn down<'a, 'b>(
    tx: &'a mut sqlx::Transaction<'b, Sqlite>,
) -> Result<(), ArgoError> {
    for sql in V92_DOWN {
        sqlx::query(sql)
            .execute(&mut **tx)
            .await
            .map_err(|e| ArgoError::Provider(format!("v92 down: {e}")))?;
    }
    Ok(())
}

const V92_UP: &[&str] = &[
    "CREATE TABLE IF NOT EXISTS approval_vote (
        tenant_id           TEXT NOT NULL,
        approval_request_id TEXT NOT NULL,
        approver            TEXT NOT NULL,
        decision            TEXT NOT NULL,
        voted_at            TEXT NOT NULL,
        UNIQUE (tenant_id, approval_request_id, approver)
    )",
    "CREATE INDEX IF NOT EXISTS idx_approval_vote_tally \
     ON approval_vote(tenant_id, approval_request_id, decision)",
];

const V92_DOWN: &[&str] = &[
    "DROP INDEX IF EXISTS idx_approval_vote_tally",
    "DROP TABLE IF EXISTS approval_vote",
];
