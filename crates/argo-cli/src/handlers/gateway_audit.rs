//! `argo gateway audit` — read per-HTTP-request gateway audit rows
//! from `audit_record WHERE kind='http.request'`.
//!
//! Consolidates with the canonical audit chain
//! (`mission-gateway-request-audit-consolidate-to-audit-record`):
//! historical `gateway_request_audit` rows are copied into
//! `audit_record` by the v67 migration, and new rows land in
//! `audit_record` via the gateway's per-request hook. The CLI keeps
//! the same `GatewayRequestAuditRow` output shape so existing
//! operator tooling continues to parse unchanged.
use crate::cli::cli_gateway_cmds::GatewayAuditCmd;
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_store::gateway_request_audit::GatewayRequestAuditRow;
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use chrono::{DateTime, Duration, Utc};
use sqlx::{QueryBuilder, Sqlite, SqlitePool};
use uuid::Uuid;

/// Run `argo gateway audit ...`.
pub async fn run(cmd: GatewayAuditCmd) -> ArgoResult<()> {
    let store = open_store().await?;
    match cmd {
        GatewayAuditCmd::List {
            since,
            principal,
            status,
            json,
            limit,
        } => {
            let rows = list_http_request_rows(
                store.pool(),
                parse_since(since.as_deref())?,
                principal.as_deref(),
                status,
                limit,
            )
            .await?;
            render_list(&rows, json)
        }
        GatewayAuditCmd::Show { request_id, json } => {
            let Some(row) = get_http_request_row(store.pool(), &request_id).await? else {
                return Err(ArgoError::Provider(format!(
                    "gateway audit: no row for request_id {request_id}"
                )));
            };
            render_show(&row, json)
        }
    }
}

async fn open_store() -> ArgoResult<SessionStore> {
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Io(format!("cwd: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(db_path(&home)).await?;
    store.migrate().await?;
    Ok(store)
}

fn parse_since(raw: Option<&str>) -> ArgoResult<Option<i64>> {
    let Some(raw) = raw.map(str::trim).filter(|s| !s.is_empty()) else {
        return Ok(None);
    };
    if let Ok(ts) = DateTime::parse_from_rfc3339(raw) {
        return Ok(Some(ts.with_timezone(&Utc).timestamp_micros()));
    }
    let compact = raw.strip_suffix(" ago").unwrap_or(raw).trim();
    let duration = parse_duration(compact)?;
    Ok(Some((Utc::now() - duration).timestamp_micros()))
}

fn parse_duration(raw: &str) -> ArgoResult<Duration> {
    let (digits, unit) = raw.split_at(raw.len().saturating_sub(1));
    let value: i64 = digits
        .parse()
        .map_err(|e| ArgoError::Schema(format!("invalid --since '{raw}': {e}")))?;
    if value <= 0 {
        return Err(ArgoError::Provider(format!(
            "invalid --since '{raw}': must be positive"
        )));
    }
    match unit {
        "s" => Ok(Duration::seconds(value)),
        "m" => Ok(Duration::minutes(value)),
        "h" => Ok(Duration::hours(value)),
        "d" => Ok(Duration::days(value)),
        _ => Err(ArgoError::Provider(format!(
            "invalid --since '{raw}' (expected 30m, 1h ago, 7d, or RFC3339)"
        ))),
    }
}

async fn list_http_request_rows(
    pool: &SqlitePool,
    since_micros: Option<i64>,
    principal: Option<&str>,
    status_code: Option<u16>,
    limit: u32,
) -> ArgoResult<Vec<GatewayRequestAuditRow>> {
    let mut qb = QueryBuilder::<Sqlite>::new(
        "SELECT id, created_at, principal_id, payload_json \
         FROM audit_record WHERE kind = 'http.request'",
    );
    if let Some(since) = since_micros {
        qb.push(" AND created_at >= ");
        qb.push_bind(since);
    }
    if let Some(p) = principal {
        qb.push(" AND principal_id = ");
        qb.push_bind(p);
    }
    if let Some(s) = status_code {
        qb.push(" AND CAST(json_extract(payload_json, '$.status') AS INTEGER) = ");
        qb.push_bind(i64::from(s));
    }
    qb.push(" ORDER BY seq DESC LIMIT ");
    qb.push_bind(i64::from(limit.clamp(1, 1000)));
    let raw: Vec<(Vec<u8>, i64, Option<String>, String)> = qb
        .build_query_as()
        .fetch_all(pool)
        .await
        .map_err(|e| ArgoError::Provider(format!("audit_record list: {e}")))?;
    raw.into_iter().map(row_from_audit).collect()
}

async fn get_http_request_row(
    pool: &SqlitePool,
    request_id: &str,
) -> ArgoResult<Option<GatewayRequestAuditRow>> {
    let id = Uuid::parse_str(request_id)
        .map_err(|e| ArgoError::Provider(format!("gateway audit id: {e}")))?;
    let raw: Option<(Vec<u8>, i64, Option<String>, String)> = sqlx::query_as(
        "SELECT id, created_at, principal_id, payload_json \
         FROM audit_record WHERE kind = 'http.request' AND id = ?",
    )
    .bind(id.as_bytes().as_slice())
    .fetch_optional(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("audit_record get: {e}")))?;
    raw.map(row_from_audit).transpose()
}

fn row_from_audit(
    t: (Vec<u8>, i64, Option<String>, String),
) -> ArgoResult<GatewayRequestAuditRow> {
    let request_id = Uuid::from_slice(&t.0)
        .map_err(|e| ArgoError::Provider(format!("audit_record row id: {e}")))?
        .to_string();
    let payload: serde_json::Value = serde_json::from_str(&t.3)
        .map_err(|e| ArgoError::Provider(format!("audit_record payload: {e}")))?;
    let s = |k: &str| {
        payload
            .get(k)
            .and_then(|v| v.as_str())
            .map(String::from)
    };
    let u = |k: &str| payload.get(k).and_then(|v| v.as_u64());
    Ok(GatewayRequestAuditRow {
        request_id,
        received_at: t.1,
        principal: t.2,
        source_ip: s("source_ip").unwrap_or_default(),
        method: s("method").unwrap_or_default(),
        path: s("path").unwrap_or_default(),
        status_code: u("status").and_then(|v| u16::try_from(v).ok()).unwrap_or(0),
        latency_ms: u("duration_ms")
            .and_then(|v| u32::try_from(v).ok())
            .unwrap_or(0),
        request_body_sha256: s("body_sha256_hex"),
        response_body_len: 0,
        user_agent: s("user_agent_truncated"),
    })
}

fn render_list(rows: &[GatewayRequestAuditRow], json: bool) -> ArgoResult<()> {
    if json {
        let body = serde_json::json!({ "rows": rows });
        println!(
            "{}",
            serde_json::to_string(&body)
                .map_err(|e| ArgoError::Provider(format!("gateway audit list json: {e}")))?
        );
        return Ok(());
    }
    if rows.is_empty() {
        println!("argo gateway audit: no rows");
        return Ok(());
    }
    println!("request_id                            status  method  path");
    for r in rows {
        println!(
            "{:<36} {:>6}  {:<6}  {}",
            r.request_id, r.status_code, r.method, r.path
        );
    }
    Ok(())
}

fn render_show(row: &GatewayRequestAuditRow, _json: bool) -> ArgoResult<()> {
    println!(
        "{}",
        serde_json::to_string_pretty(row)
            .map_err(|e| ArgoError::Provider(format!("gateway audit show json: {e}")))?
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use argo_audit::{AuditKind, AuditSink};
    use std::sync::Arc;

    #[test]
    fn parse_since_accepts_relative_with_ago_suffix() {
        assert!(parse_since(Some("1h ago")).unwrap().is_some());
    }

    #[test]
    fn parse_since_accepts_rfc3339() {
        let got = parse_since(Some("2026-05-13T00:00:00Z")).unwrap().unwrap();
        assert_eq!(got, 1_778_630_400_000_000);
    }

    #[test]
    fn parse_since_rejects_garbage() {
        assert!(parse_since(Some("yesterday")).is_err());
    }

    #[tokio::test]
    async fn list_http_request_rows_maps_payload_keys() {
        let store = Arc::new(SessionStore::open(":memory:").await.unwrap());
        store.migrate().await.unwrap();
        let sink = AuditSink::new(Arc::clone(&store));
        let payload = serde_json::json!({
            "method": "POST",
            "path": "/sessions",
            "status": 201,
            "duration_ms": 12,
            "tenant_id": "acme",
            "source_ip": "203.0.113.5",
            "user_agent_truncated": "argo-cli/0.1",
            "body_sha256_hex": "abc123",
        });
        sink.write(None, AuditKind::HttpRequest, payload, Some("alice".into()))
            .await
            .unwrap();
        let rows = list_http_request_rows(store.pool(), None, None, None, 100)
            .await
            .unwrap();
        assert_eq!(rows.len(), 1);
        let r = &rows[0];
        assert_eq!(r.method, "POST");
        assert_eq!(r.path, "/sessions");
        assert_eq!(r.status_code, 201);
        assert_eq!(r.latency_ms, 12);
        assert_eq!(r.source_ip, "203.0.113.5");
        assert_eq!(r.user_agent.as_deref(), Some("argo-cli/0.1"));
        assert_eq!(r.request_body_sha256.as_deref(), Some("abc123"));
        assert_eq!(r.principal.as_deref(), Some("alice"));
    }

    #[tokio::test]
    async fn get_http_request_row_finds_by_id() {
        let store = Arc::new(SessionStore::open(":memory:").await.unwrap());
        store.migrate().await.unwrap();
        let sink = AuditSink::new(Arc::clone(&store));
        sink.write(
            None,
            AuditKind::HttpRequest,
            serde_json::json!({"method":"GET","path":"/x","status":200,"duration_ms":1}),
            None,
        )
        .await
        .unwrap();
        let id: Vec<u8> = sqlx::query_scalar(
            "SELECT id FROM audit_record WHERE kind = 'http.request' LIMIT 1",
        )
        .fetch_one(store.pool())
        .await
        .unwrap();
        let uuid = Uuid::from_slice(&id).unwrap().to_string();
        let got = get_http_request_row(store.pool(), &uuid).await.unwrap();
        assert!(got.is_some());
        assert!(
            get_http_request_row(store.pool(), "00000000-0000-0000-0000-000000000000")
                .await
                .unwrap()
                .is_none()
        );
    }
}
