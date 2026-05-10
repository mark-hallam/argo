//! Handoff GET/PATCH gateway routes — split from
//! `routes_orchestrator.rs` to keep that file under the 500-LOC budget.
//! The dispatcher in `routes_orchestrator.rs` still owns auth, schema
//! ensure, and method/path matching; this module owns the handoff-
//! sub-resource handlers and their JSON-shape helpers.

use super::routes_orchestrator::{
    bad, bad_owned, conflict_or_server, db, micros, not_found, opt_micros, path_id, query, server,
    to_body, uuid_blob,
};
use argo_identity::WorkerId;
use argo_orchestrator::{EscalationStep, Handoff, HandoffId, HandoffStatus, OrchestratorStore};
use argo_store::SessionStore;
use argo_types::{ArgoResult, TenantId};
use serde_json::{json, Value};
use sqlx::Row;
use std::str::FromStr;

pub(super) async fn show_handoff(
    repo: &OrchestratorStore,
    tenant: TenantId,
    path: &str,
) -> Result<(&'static str, String), (&'static str, String)> {
    let id = path_id(path, "/handoffs/")?;
    let handoff = repo
        .find_handoff(tenant, &HandoffId::from_string(id.to_string()))
        .await
        .map_err(server)?
        .ok_or_else(not_found)?;
    Ok(("200 OK", to_body(&handoff_json(&handoff))))
}

pub(super) async fn patch_handoff(
    repo: &OrchestratorStore,
    tenant: TenantId,
    path: &str,
    body: Option<&Value>,
) -> Result<(&'static str, String), (&'static str, String)> {
    let id = path_id(path.trim_end_matches("/status"), "/handoffs/")?;
    let status = match body.and_then(|v| v.get("status")).and_then(Value::as_str) {
        Some("acknowledged") => HandoffStatus::Acknowledged,
        Some("settled") => HandoffStatus::Settled,
        Some(_) => return Err(bad("status must be acknowledged|settled")),
        None => return Err(bad("missing status")),
    };
    let hid = HandoffId::from_string(id.to_string());
    repo.update_handoff_status(tenant, &hid, status)
        .await
        .map_err(conflict_or_server)?;
    show_handoff(repo, tenant, &format!("/handoffs/{id}")).await
}

pub(super) async fn list_handoffs(
    store: &SessionStore,
    target: &str,
    tenant: TenantId,
) -> Result<(&'static str, String), (&'static str, String)> {
    let q = query(target, &["tenant", "session", "worker", "status", "limit"])?;
    let status = q.get("status").map(String::as_str);
    let limit = q.get("limit").and_then(|v| v.parse::<i64>().ok()).unwrap_or(50).clamp(1, 500);
    let mut sql = "SELECT h.id,h.session_id,h.from_worker_id,h.\"to\",h.expected_outcome,h.payload,h.status,h.created_at,h.deadline FROM handoff h JOIN session s ON s.id=h.session_id WHERE s.tenant_id=? AND s.provider='orchestrator'".to_string();
    if q.contains_key("session") { sql.push_str(" AND h.session_id=?"); }
    if q.contains_key("worker") { sql.push_str(" AND (h.from_worker_id=? OR h.\"to\" LIKE ?)"); }
    if status.is_some() { sql.push_str(" AND h.status=?"); }
    sql.push_str(" ORDER BY h.created_at DESC LIMIT ?");
    let mut stmt = sqlx::query(&sql).bind(tenant.0.as_bytes().as_slice());
    if let Some(s) = q.get("session") { stmt = stmt.bind(session_blob(s)?); }
    if let Some(w) = q.get("worker") {
        WorkerId::from_str(w).map_err(|e| bad_owned(format!("invalid worker id '{w}': {e}")))?;
        stmt = stmt.bind(w).bind(format!("%{w}%"));
    }
    if let Some(s) = status {
        if !matches!(s, "pending" | "acknowledged" | "settled" | "expired") {
            return Err(bad("status must be pending|acknowledged|settled|expired"));
        }
        stmt = stmt.bind(s);
    }
    let rows = stmt.bind(limit).fetch_all(store.pool()).await.map_err(server)?;
    let out: Vec<_> = rows.iter().map(handoff_row_json).collect::<ArgoResult<_>>().map_err(server)?;
    Ok(("200 OK", to_body(&json!({"handoffs":out}))))
}

pub(super) fn handoff_json(h: &Handoff) -> Value {
    let to = match &h.to {
        EscalationStep::Worker(id) => json!({"kind":"worker","worker_id":id.to_string()}),
        EscalationStep::Human(addr) => json!({"kind":"human","addr":addr}),
    };
    json!({"id":h.id.to_string(),"session_id":h.session_id.to_string(),"from":h.from.to_string(),"to":to,"expected_outcome":h.expected_outcome,"payload":h.payload,"status":handoff_status(h.status),"created_at":h.created_at.to_rfc3339(),"deadline":h.deadline.map(|d| d.to_rfc3339())})
}

fn handoff_row_json(row: &sqlx::sqlite::SqliteRow) -> ArgoResult<Value> {
    let sid: Vec<u8> = row.try_get("session_id").map_err(db)?;
    let to: String = row.try_get("to").map_err(db)?;
    let payload: String = row.try_get("payload").map_err(db)?;
    Ok(json!({"id":row.try_get::<String,_>("id").map_err(db)?,"session_id":uuid_blob(&sid)?,"from":row.try_get::<String,_>("from_worker_id").map_err(db)?,"to":serde_json::from_str::<Value>(&to).unwrap_or(Value::Null),"expected_outcome":row.try_get::<String,_>("expected_outcome").map_err(db)?,"payload":serde_json::from_str::<Value>(&payload).unwrap_or(Value::Null),"status":row.try_get::<String,_>("status").map_err(db)?,"created_at":micros(row.try_get("created_at").map_err(db)?)?,"deadline":opt_micros(row.try_get("deadline").map_err(db)?)?}))
}

fn handoff_status(s: HandoffStatus) -> &'static str {
    match s {
        HandoffStatus::Pending => "pending",
        HandoffStatus::Acknowledged => "acknowledged",
        HandoffStatus::Settled => "settled",
        HandoffStatus::Expired => "expired",
        HandoffStatus::Cancelled => "cancelled",
    }
}

fn session_blob(raw: &str) -> Result<Vec<u8>, (&'static str, String)> {
    uuid::Uuid::parse_str(raw).map(|u| u.as_bytes().to_vec()).map_err(|e| bad_owned(format!("invalid session id '{raw}': {e}")))
}
