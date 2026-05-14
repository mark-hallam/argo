//! Schema migration v66 — gateway request audit table.
//!
//! DEPRECATED: see [`crate::gateway_request_audit`]. The
//! `gateway_request_audit` table is superseded by `audit_record`
//! `kind='http.request'` as of
//! `mission-gateway-request-audit-consolidate-to-audit-record`.
//! Planned deletion in v1.2; legacy rows remain queryable directly.
use argo_types::ArgoError;
use sqlx::Sqlite;

pub(super) async fn up<'a, 'b>(tx: &'a mut sqlx::Transaction<'b, Sqlite>) -> Result<(), ArgoError> {
    for sql in V66_UP {
        sqlx::query(sql)
            .execute(&mut **tx)
            .await
            .map_err(|e| ArgoError::Provider(format!("v66 up: {e}")))?;
    }
    Ok(())
}

pub(super) async fn down<'a, 'b>(
    tx: &'a mut sqlx::Transaction<'b, Sqlite>,
) -> Result<(), ArgoError> {
    for sql in V66_DOWN {
        sqlx::query(sql)
            .execute(&mut **tx)
            .await
            .map_err(|e| ArgoError::Provider(format!("v66 down: {e}")))?;
    }
    Ok(())
}

const V66_UP: &[&str] = &[
    "CREATE TABLE IF NOT EXISTS gateway_request_audit (
        request_id BLOB PRIMARY KEY NOT NULL,
        received_at INTEGER NOT NULL,
        principal TEXT,
        source_ip TEXT NOT NULL,
        method TEXT NOT NULL,
        path TEXT NOT NULL,
        status_code INTEGER NOT NULL,
        latency_ms INTEGER NOT NULL,
        request_body_sha256 BLOB,
        response_body_len INTEGER NOT NULL,
        user_agent TEXT
    )",
    "CREATE INDEX IF NOT EXISTS idx_gateway_request_audit_received_at
        ON gateway_request_audit(received_at)",
    "CREATE INDEX IF NOT EXISTS idx_gateway_request_audit_principal
        ON gateway_request_audit(principal) WHERE principal IS NOT NULL",
    "CREATE INDEX IF NOT EXISTS idx_gateway_request_audit_status
        ON gateway_request_audit(status_code)",
];

const V66_DOWN: &[&str] = &[
    "DROP INDEX IF EXISTS idx_gateway_request_audit_status",
    "DROP INDEX IF EXISTS idx_gateway_request_audit_principal",
    "DROP INDEX IF EXISTS idx_gateway_request_audit_received_at",
    "DROP TABLE IF EXISTS gateway_request_audit",
];
