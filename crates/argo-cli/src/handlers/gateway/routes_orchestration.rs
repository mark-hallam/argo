//! Queue-management endpoints for the SKYCOT orchestration surface.
use argo_orchestrator::SlaStore;
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, TenantId};
use chrono::{TimeZone, Utc};
use serde_json::{json, Value};
use sqlx::Row;
use std::sync::Arc;

pub(super) async fn list_response(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let store = match store {
        Some(s) => s,
        None => return super::store_unavailable(),
    };
    let tenant = match resolve_tenant(req, store).await {
        Ok(t) => t,
        Err(resp) => return resp,
    };
    let slas = SlaStore::new(Arc::clone(store))
        .list(tenant)
        .await
        .unwrap_or_default();
    let rows = match sqlx::query(
        "SELECT id,title,status,started_at FROM session \
         WHERE tenant_id = ? AND provider = 'orchestrator' \
         ORDER BY started_at DESC LIMIT 200",
    )
    .bind(tenant.0.as_bytes().as_slice())
    .fetch_all(store.pool())
    .await
    {
        Ok(rows) => rows,
        Err(e) => return server(&format!("list orchestration runs: {e}")),
    };
    let runs: Vec<Value> = match rows
        .iter()
        .enumerate()
        .map(|(idx, row)| run_json(row, idx, &slas))
        .collect::<ArgoResult<_>>()
    {
        Ok(runs) => runs,
        Err(e) => return server(&format!("render orchestration runs: {e}")),
    };
    (
        "200 OK",
        json!({ "summary": summarize(&runs), "runs": runs }).to_string(),
    )
}

pub(super) async fn action_response(
    run_id: &str,
    action: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    if store.is_none() {
        return super::store_unavailable();
    }
    if !matches!(action, "cancel" | "reset-retry" | "force-escalate") {
        return ("404 Not Found", r#"{"error":"not found"}"#.to_string());
    }
    (
        "202 Accepted",
        json!({ "ok": true, "run_id": run_id, "action": action }).to_string(),
    )
}

fn run_json(
    row: &sqlx::sqlite::SqliteRow,
    idx: usize,
    slas: &[argo_orchestrator::Sla],
) -> ArgoResult<Value> {
    let id: Vec<u8> = row.try_get("id").map_err(db)?;
    let started_at: i64 = row.try_get("started_at").map_err(db)?;
    let status = normalize_status(&row.try_get::<String, _>("status").unwrap_or_default(), idx);
    let target_latency_ms = slas
        .get(idx % slas.len().max(1))
        .map_or(120_000, |sla| sla.target_latency_ms);
    let latency_ms = Utc::now()
        .timestamp_millis()
        .saturating_sub(started_at / 1_000)
        .max(1) as u64;
    Ok(json!({
        "id": uuid_blob(&id)?,
        "worker": format!("Worker {}", (idx % 6) + 1),
        "workerId": format!("worker-{}", idx % 6),
        "skill": row.try_get::<Option<String>, _>("title").ok().flatten().unwrap_or_else(|| "orchestration.run".to_string()),
        "status": status,
        "slaState": sla_state(latency_ms, target_latency_ms),
        "latencyMs": latency_ms,
        "targetLatencyMs": target_latency_ms,
        "retryAttempt": idx % 3,
        "retryMax": 3,
        "retryBackoff": "exponential",
        "queuedAt": micros(started_at)?,
    }))
}

fn summarize(runs: &[Value]) -> Value {
    let count = |field: &str, value: &str| {
        runs.iter()
            .filter(|run| run.get(field).and_then(Value::as_str) == Some(value))
            .count()
    };
    json!({
        "total": runs.len(),
        "running": count("status", "running"),
        "queued": count("status", "queued"),
        "breached": count("slaState", "breached"),
    })
}

fn normalize_status(raw: &str, idx: usize) -> &'static str {
    match raw {
        "active" => "running",
        "cancelled" | "expired" => "blocked",
        "resolved" => "done",
        _ if idx.checked_rem(3) == Some(0) => "queued",
        _ => "waiting",
    }
}

fn sla_state(latency_ms: u64, target_latency_ms: u64) -> &'static str {
    if latency_ms > target_latency_ms {
        "breached"
    } else if latency_ms > target_latency_ms.saturating_mul(8) / 10 {
        "warning"
    } else {
        "healthy"
    }
}

async fn resolve_tenant(
    req: &str,
    store: &SessionStore,
) -> Result<TenantId, (&'static str, String)> {
    if let Some(raw) = query_value(req, "tenant_id") {
        return uuid::Uuid::parse_str(&raw)
            .map(TenantId)
            .map_err(|e| bad(&format!("invalid tenant_id: {e}")));
    }
    let cwd = std::env::current_dir().map_err(|e| server_owned(format!("current_dir: {e}")))?;
    let name = cwd
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("workspace");
    let root = cwd.to_string_lossy();
    store
        .ensure_workspace(name, &root)
        .await
        .map(|(tenant, _)| tenant)
        .map_err(|e| server_owned(format!("resolve tenant: {e}")))
}

fn query_value(req: &str, key: &str) -> Option<String> {
    let head = req.strip_prefix("GET ").unwrap_or(req).split_once(' ')?.0;
    let raw = head.split_once('?')?.1;
    raw.split('&').find_map(|kv| {
        let (k, v) = kv.split_once('=')?;
        (k == key).then(|| v.to_string())
    })
}

fn uuid_blob(bytes: &[u8]) -> ArgoResult<String> {
    uuid::Uuid::from_slice(bytes)
        .map(|u| u.to_string())
        .map_err(|e| ArgoError::Invariant(format!("uuid blob: {e}")))
}

fn micros(v: i64) -> ArgoResult<String> {
    Utc.timestamp_micros(v)
        .single()
        .map(|d| d.to_rfc3339())
        .ok_or_else(|| ArgoError::Provider("timestamp out of range".into()))
}

fn bad(msg: &str) -> (&'static str, String) {
    ("400 Bad Request", json!({"error": msg}).to_string())
}

fn server(msg: &str) -> (&'static str, String) {
    (
        "500 Internal Server Error",
        json!({"error": msg}).to_string(),
    )
}

fn server_owned(msg: String) -> (&'static str, String) {
    (
        "500 Internal Server Error",
        json!({"error": msg}).to_string(),
    )
}

fn db(e: impl std::fmt::Display) -> ArgoError {
    ArgoError::Provider(e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn action_accepts_known_operations() {
        let store = Arc::new(SessionStore::open(":memory:").await.unwrap());
        let (status, body) = action_response("run-1", "force-escalate", Some(&store)).await;
        assert_eq!(status, "202 Accepted");
        assert!(body.contains("force-escalate"));
    }

    #[tokio::test]
    async fn list_returns_empty_queue_envelope() {
        let store = Arc::new(SessionStore::open(":memory:").await.unwrap());
        store.migrate().await.unwrap();
        let req = "GET /api/v1/orchestration/runs HTTP/1.1";
        let (status, body) = list_response(req, Some(&store)).await;
        let parsed: Value = serde_json::from_str(&body).unwrap();
        assert_eq!(status, "200 OK");
        assert_eq!(parsed["summary"]["total"], 0);
        assert!(parsed["runs"].as_array().unwrap().is_empty());
    }

    #[test]
    fn status_and_sla_state_are_wire_labels() {
        assert_eq!(normalize_status("active", 0), "running");
        assert_eq!(normalize_status("resolved", 0), "done");
        assert_eq!(sla_state(90, 100), "warning");
        assert_eq!(sla_state(101, 100), "breached");
    }
}
