//! Session lifecycle gateway routes — split from `routes_orchestrator.rs` to
//! keep that file under the 500-LOC budget. The dispatcher in
//! `routes_orchestrator.rs` still owns auth, schema-ensure, and method/path
//! matching; this module owns the `/sessions` sub-resource handlers and the
//! row-to-JSON helper.

use super::routes_orchestrator::{
    bad, conflict_or_server, db, handoff_json, micros, not_found, opt_micros, path_id, query,
    server, to_body, uuid_blob,
};
use argo_orchestrator::{HandoffStatus, OrchestratorStore, Session, SessionStatus};
use argo_store::SessionStore;
use argo_types::{ArgoResult, TenantId};
use serde_json::{json, Value};
use sqlx::Row;

pub(super) async fn create_session(
    repo: &OrchestratorStore,
    tenant: TenantId,
    body: Option<&Value>,
) -> Result<(&'static str, String), (&'static str, String)> {
    let title = body
        .and_then(|v| v.get("title"))
        .and_then(Value::as_str)
        .ok_or_else(|| bad("missing title"))?;
    let session = Session::new(tenant, title);
    repo.insert_session(&session).await.map_err(server)?;
    Ok(("201 Created", to_body(&session)))
}

pub(super) async fn show_session(
    repo: &OrchestratorStore,
    tenant: TenantId,
    path: &str,
) -> Result<(&'static str, String), (&'static str, String)> {
    let id = path_id(path, "/sessions/")?;
    let sid = argo_orchestrator::SessionId::from_string(id.to_string());
    let session = repo
        .find_session(tenant, &sid)
        .await
        .map_err(server)?
        .ok_or_else(not_found)?;
    let pending: Vec<_> = repo
        .list_handoffs_for_session(tenant, &sid)
        .await
        .map_err(server)?
        .into_iter()
        .filter(|h| h.status == HandoffStatus::Pending)
        .map(|h| handoff_json(&h))
        .collect();
    Ok((
        "200 OK",
        to_body(
            &json!({"session":session,"participants":session.participants,"pending_handoffs":pending}),
        ),
    ))
}

pub(super) async fn patch_session(
    repo: &OrchestratorStore,
    tenant: TenantId,
    path: &str,
    body: Option<&Value>,
) -> Result<(&'static str, String), (&'static str, String)> {
    let id = path_id(path.trim_end_matches("/status"), "/sessions/")?;
    let status = match body.and_then(|v| v.get("status")).and_then(Value::as_str) {
        Some("resolved") => SessionStatus::Resolved,
        Some("cancelled") => SessionStatus::Cancelled,
        Some(_) => return Err(bad("status must be resolved|cancelled")),
        None => return Err(bad("missing status")),
    };
    let sid = argo_orchestrator::SessionId::from_string(id.to_string());
    repo.update_session_status(tenant, &sid, status)
        .await
        .map_err(conflict_or_server)?;
    show_session(repo, tenant, &format!("/sessions/{id}")).await
}

pub(super) async fn list_sessions(
    store: &SessionStore,
    target: &str,
    tenant: TenantId,
) -> Result<(&'static str, String), (&'static str, String)> {
    let q = query(target, &["tenant", "status", "limit"])?;
    let status = q.get("status").map(String::as_str);
    let limit = q
        .get("limit")
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(50)
        .clamp(1, 500);
    let mut sql = "SELECT id,title,status,participants,created_at,deadline FROM session WHERE tenant_id=? AND provider='orchestrator'".to_string();
    if status.is_some() {
        sql.push_str(" AND status=?");
    }
    sql.push_str(" ORDER BY created_at DESC LIMIT ?");
    let mut stmt = sqlx::query(&sql).bind(tenant.0.as_bytes().as_slice());
    if let Some(s) = status {
        if !matches!(s, "active" | "resolved" | "cancelled" | "expired") {
            return Err(bad("status must be active|resolved|cancelled|expired"));
        }
        stmt = stmt.bind(s);
    }
    let rows = stmt
        .bind(limit)
        .fetch_all(store.pool())
        .await
        .map_err(server)?;
    let out: Vec<_> = rows
        .iter()
        .map(session_row_json)
        .collect::<ArgoResult<_>>()
        .map_err(server)?;
    Ok(("200 OK", to_body(&json!({"sessions":out}))))
}

fn session_row_json(row: &sqlx::sqlite::SqliteRow) -> ArgoResult<Value> {
    let id: Vec<u8> = row.try_get("id").map_err(db)?;
    let participants: String = row.try_get("participants").map_err(db)?;
    Ok(
        json!({"id":uuid_blob(&id)?,"title":row.try_get::<Option<String>,_>("title").map_err(db)?,"status":row.try_get::<String,_>("status").map_err(db)?,"participants":serde_json::from_str::<Value>(&participants).unwrap_or_else(|_| json!([])),"created_at":micros(row.try_get("created_at").map_err(db)?)?,"deadline":opt_micros(row.try_get("deadline").map_err(db)?)?}),
    )
}
