//! Gateway request audit table primitives.
//!
//! The table is intentionally separate from the append-only audit
//! chain: it captures one row per HTTP request for compliance
//! reporting without changing the chain hash inputs or retention
//! policy.
use crate::StoreResult;
use argo_types::ArgoError;
use serde::Serialize;
use sqlx::{QueryBuilder, Sqlite, SqlitePool};
use uuid::Uuid;

/// One row to insert into `gateway_request_audit`.
pub struct NewGatewayRequestAudit {
    /// Request correlation id.
    pub request_id: Uuid,
    /// Request receive timestamp in microseconds since Unix epoch.
    pub received_at: i64,
    /// Resolved principal id, or `None` for anonymous routes.
    pub principal: Option<String>,
    /// Source IP address as observed by the gateway.
    pub source_ip: String,
    /// HTTP method.
    pub method: String,
    /// Path with query string stripped.
    pub path: String,
    /// HTTP status code.
    pub status_code: u16,
    /// Request latency in milliseconds.
    pub latency_ms: u32,
    /// SHA-256 of the request body when configured for the method.
    pub request_body_sha256: Option<[u8; 32]>,
    /// Response body byte length.
    pub response_body_len: u32,
    /// User-Agent value, already truncated by the caller.
    pub user_agent: Option<String>,
}

/// Stored request-audit row rendered for CLI/API consumers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GatewayRequestAuditRow {
    /// Request correlation id.
    pub request_id: String,
    /// Request receive timestamp in microseconds since Unix epoch.
    pub received_at: i64,
    /// Resolved principal id, or `None` for anonymous routes.
    pub principal: Option<String>,
    /// Source IP address as observed by the gateway.
    pub source_ip: String,
    /// HTTP method.
    pub method: String,
    /// Path with query string stripped.
    pub path: String,
    /// HTTP status code.
    pub status_code: u16,
    /// Request latency in milliseconds.
    pub latency_ms: u32,
    /// Hex-encoded SHA-256 request body hash.
    pub request_body_sha256: Option<String>,
    /// Response body byte length.
    pub response_body_len: u32,
    /// User-Agent value.
    pub user_agent: Option<String>,
}

/// Query filter for [`list`].
#[derive(Debug, Clone, Default)]
pub struct GatewayRequestAuditFilter {
    /// Lower bound for `received_at`, in microseconds.
    pub since_micros: Option<i64>,
    /// Exact principal filter.
    pub principal: Option<String>,
    /// Exact HTTP status-code filter.
    pub status_code: Option<u16>,
    /// Maximum rows to return.
    pub limit: u32,
}

/// Insert one request-audit row.
///
/// # Errors
/// Returns [`ArgoError::Provider`] when SQLite rejects the insert.
pub async fn record(pool: &SqlitePool, entry: &NewGatewayRequestAudit) -> StoreResult<()> {
    sqlx::query(
        "INSERT INTO gateway_request_audit \
         (request_id, received_at, principal, source_ip, method, path, status_code, \
          latency_ms, request_body_sha256, response_body_len, user_agent) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(entry.request_id.as_bytes().as_slice())
    .bind(entry.received_at)
    .bind(entry.principal.as_deref())
    .bind(&entry.source_ip)
    .bind(&entry.method)
    .bind(&entry.path)
    .bind(i64::from(entry.status_code))
    .bind(i64::from(entry.latency_ms))
    .bind(entry.request_body_sha256.map(|b| b.to_vec()))
    .bind(i64::from(entry.response_body_len))
    .bind(entry.user_agent.as_deref())
    .execute(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("gateway_request_audit record: {e}")))?;
    Ok(())
}

/// List request-audit rows newest-first.
///
/// # Errors
/// Returns [`ArgoError::Provider`] on SQL failure or malformed UUID
/// bytes in the table.
pub async fn list(
    pool: &SqlitePool,
    filter: &GatewayRequestAuditFilter,
) -> StoreResult<Vec<GatewayRequestAuditRow>> {
    let mut qb = QueryBuilder::<Sqlite>::new(
        "SELECT request_id, received_at, principal, source_ip, method, path, \
         status_code, latency_ms, request_body_sha256, response_body_len, user_agent \
         FROM gateway_request_audit WHERE 1 = 1",
    );
    if let Some(since) = filter.since_micros {
        qb.push(" AND received_at >= ");
        qb.push_bind(since);
    }
    if let Some(principal) = filter.principal.as_deref() {
        qb.push(" AND principal = ");
        qb.push_bind(principal);
    }
    if let Some(status) = filter.status_code {
        qb.push(" AND status_code = ");
        qb.push_bind(i64::from(status));
    }
    qb.push(" ORDER BY received_at DESC LIMIT ");
    qb.push_bind(i64::from(filter.limit.clamp(1, 1000)));

    let rows: Vec<RowTuple> = qb
        .build_query_as()
        .fetch_all(pool)
        .await
        .map_err(|e| ArgoError::Provider(format!("gateway_request_audit list: {e}")))?;
    rows.into_iter().map(row_from_tuple).collect()
}

/// Fetch one request-audit row by id.
///
/// # Errors
/// Returns [`ArgoError::Provider`] on SQL failure, malformed input id,
/// or malformed UUID bytes in the table.
pub async fn get(
    pool: &SqlitePool,
    request_id: &str,
) -> StoreResult<Option<GatewayRequestAuditRow>> {
    let id = Uuid::parse_str(request_id)
        .map_err(|e| ArgoError::Provider(format!("gateway_request_audit id: {e}")))?;
    let row: Option<RowTuple> = sqlx::query_as(
        "SELECT request_id, received_at, principal, source_ip, method, path, \
         status_code, latency_ms, request_body_sha256, response_body_len, user_agent \
         FROM gateway_request_audit WHERE request_id = ?",
    )
    .bind(id.as_bytes().as_slice())
    .fetch_optional(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("gateway_request_audit get: {e}")))?;
    row.map(row_from_tuple).transpose()
}

type RowTuple = (
    Vec<u8>,
    i64,
    Option<String>,
    String,
    String,
    String,
    i64,
    i64,
    Option<Vec<u8>>,
    i64,
    Option<String>,
);

fn row_from_tuple(t: RowTuple) -> StoreResult<GatewayRequestAuditRow> {
    let request_id = Uuid::from_slice(&t.0)
        .map_err(|e| ArgoError::Provider(format!("gateway_request_audit row id: {e}")))?
        .to_string();
    Ok(GatewayRequestAuditRow {
        request_id,
        received_at: t.1,
        principal: t.2,
        source_ip: t.3,
        method: t.4,
        path: t.5,
        status_code: u16::try_from(t.6).unwrap_or(0),
        latency_ms: u32::try_from(t.7).unwrap_or(u32::MAX),
        request_body_sha256: t.8.as_deref().map(hex_bytes),
        response_body_len: u32::try_from(t.9).unwrap_or(u32::MAX),
        user_agent: t.10,
    })
}

fn hex_bytes(bytes: &[u8]) -> String {
    const LUT: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push(char::from(LUT[usize::from(b >> 4)]));
        out.push(char::from(LUT[usize::from(b & 0x0f)]));
    }
    out
}

#[cfg(test)]
#[path = "gateway_request_audit_tests.rs"]
mod tests;
