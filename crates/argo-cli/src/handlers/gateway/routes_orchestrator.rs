//! Tenant-scoped orchestrator session + handoff gateway routes.
//!
//! Handoff sub-resource handlers live in
//! [`super::routes_orchestrator_handoffs`] to keep this file under the
//! 500-LOC budget. They are re-exported below so the dispatcher (and
//! external callers, if any) resolve the same paths as before the split.
use argo_orchestrator::{
    HandoffStatus, OrchestratorStore, Session, SessionStatus,
};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, TenantId};
use chrono::{TimeZone, Utc};
use serde_json::{json, Value};
use sqlx::Row;
use std::sync::Arc;

pub(super) use super::routes_orchestrator_handoffs::{
    handoff_json, list_handoffs, patch_handoff, show_handoff,
};

pub(super) async fn dispatch_route(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> Option<(&'static str, String)> {
    let (method, target) = request_line(req)?;
    let path = target.split('?').next().unwrap_or(target);
    if !is_orchestrator_path(method, path, req) {
        return None;
    }
    let Some(store) = store else {
        return Some(super::store_unavailable());
    };
    Some(match route(method, target, req, store).await {
        Ok(resp) => resp,
        Err(resp) => resp,
    })
}

async fn route(
    method: &str,
    target: &str,
    req: &str,
    store: &Arc<SessionStore>,
) -> Result<(&'static str, String), (&'static str, String)> {
    let body = super::body_from_request(req);
    let body_json = parse_body(body)?;
    let tenant = tenant_from_req(target, body_json.as_ref())?;
    auth(req, tenant)?;
    let repo = OrchestratorStore::new(Arc::clone(store));
    repo.ensure_schema().await.map_err(server)?;
    let path = target.split('?').next().unwrap_or(target);
    match (method, path) {
        ("GET", "/sessions") => list_sessions(store, target, tenant).await,
        ("POST", "/sessions") => create_session(&repo, tenant, body_json.as_ref()).await,
        ("GET", p) if p.starts_with("/sessions/") => show_session(&repo, tenant, p).await,
        ("PATCH", p) if p.starts_with("/sessions/") && p.ends_with("/status") => {
            patch_session(&repo, tenant, p, body_json.as_ref()).await
        }
        ("GET", "/handoffs") => list_handoffs(store, target, tenant).await,
        ("GET", p) if p.starts_with("/handoffs/") => show_handoff(&repo, tenant, p).await,
        ("PATCH", p) if p.starts_with("/handoffs/") && p.ends_with("/status") => {
            patch_handoff(&repo, tenant, p, body_json.as_ref()).await
        }
        _ => Err(not_found()),
    }
}

async fn create_session(
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

async fn show_session(
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
    Ok(("200 OK", to_body(&json!({"session":session,"participants":session.participants,"pending_handoffs":pending}))))
}

async fn patch_session(
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

async fn list_sessions(
    store: &SessionStore,
    target: &str,
    tenant: TenantId,
) -> Result<(&'static str, String), (&'static str, String)> {
    let q = query(target, &["tenant", "status", "limit"])?;
    let status = q.get("status").map(String::as_str);
    let limit = q.get("limit").and_then(|v| v.parse::<i64>().ok()).unwrap_or(50).clamp(1, 500);
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
    let rows = stmt.bind(limit).fetch_all(store.pool()).await.map_err(server)?;
    let out: Vec<_> = rows.iter().map(session_row_json).collect::<ArgoResult<_>>().map_err(server)?;
    Ok(("200 OK", to_body(&json!({"sessions":out}))))
}

fn is_orchestrator_path(method: &str, path: &str, req: &str) -> bool {
    path.starts_with("/handoffs/")
        || path == "/handoffs"
        || method == "PATCH" && path.starts_with("/sessions/")
        || path.starts_with("/sessions/") && (target_has_tenant(req) || default_tenant().is_some())
        || path == "/sessions" && (target_has_tenant(req) || body_has_tenant(req) || default_tenant().is_some())
}

fn auth(req: &str, tenant: TenantId) -> Result<(), (&'static str, String)> {
    let token = header(req, "Authorization")
        .and_then(|h| h.strip_prefix("Bearer ").map(str::to_string))
        .or_else(|| cookie(req, "argo_operator_token"))
        .ok_or_else(|| unauthorized("missing bearer token"))?;
    if let Ok(expected) = argo_config::cred::get("ARGO_WEBINBOX_BEARER_TOKEN") {
        if !expected.is_empty() && token != expected {
            return Err(unauthorized("invalid bearer token"));
        }
    }
    if let Some(expected) = default_tenant() {
        if tenant.to_string() != expected {
            return Err(forbidden("tenant mismatch"));
        }
    }
    Ok(())
}

fn tenant_from_req(target: &str, body: Option<&Value>) -> Result<TenantId, (&'static str, String)> {
    let raw = query(target, &["tenant", "status", "limit", "session", "worker"])?
        .get("tenant")
        .cloned()
        .or_else(|| body.and_then(|v| v.get("tenant_id")).and_then(Value::as_str).map(str::to_string))
        .or_else(default_tenant)
        .ok_or_else(|| bad("missing tenant"))?;
    uuid::Uuid::parse_str(&raw).map(TenantId).map_err(|e| bad_owned(format!("invalid tenant '{raw}': {e}")))
}

fn request_line(req: &str) -> Option<(&str, &str)> {
    let mut parts = req.lines().next()?.split_whitespace();
    Some((parts.next()?, parts.next()?))
}

pub(super) fn query(target: &str, allowed: &[&str]) -> Result<std::collections::HashMap<String, String>, (&'static str, String)> {
    let Some((_, raw)) = target.split_once('?') else { return Ok(Default::default()); };
    let mut out = std::collections::HashMap::new();
    for kv in raw.split('&').filter(|s| !s.is_empty()) {
        let (k, v) = kv.split_once('=').unwrap_or((kv, ""));
        if !allowed.contains(&k) {
            return Err(bad_owned(format!("unknown query parameter '{k}'")));
        }
        out.insert(k.to_string(), v.to_string());
    }
    Ok(out)
}

fn parse_body(raw: &str) -> Result<Option<Value>, (&'static str, String)> {
    if raw.trim().is_empty() {
        return Ok(None);
    }
    serde_json::from_str(raw).map(Some).map_err(|e| bad_owned(format!("parse JSON body: {e}")))
}

pub(super) fn path_id<'a>(path: &'a str, prefix: &str) -> Result<&'a str, (&'static str, String)> {
    path.strip_prefix(prefix)
        .filter(|id| !id.is_empty() && !id.contains('/'))
        .ok_or_else(not_found)
}

fn session_row_json(row: &sqlx::sqlite::SqliteRow) -> ArgoResult<Value> {
    let id: Vec<u8> = row.try_get("id").map_err(db)?;
    let participants: String = row.try_get("participants").map_err(db)?;
    Ok(json!({"id":uuid_blob(&id)?,"title":row.try_get::<Option<String>,_>("title").map_err(db)?,"status":row.try_get::<String,_>("status").map_err(db)?,"participants":serde_json::from_str::<Value>(&participants).unwrap_or_else(|_| json!([])),"created_at":micros(row.try_get("created_at").map_err(db)?)?,"deadline":opt_micros(row.try_get("deadline").map_err(db)?)?}))
}

pub(super) fn uuid_blob(bytes: &[u8]) -> ArgoResult<String> {
    uuid::Uuid::from_slice(bytes).map(|u| u.to_string()).map_err(|e| ArgoError::Provider(format!("uuid blob: {e}")))
}

pub(super) fn micros(v: i64) -> ArgoResult<String> {
    Utc.timestamp_micros(v).single().map(|d| d.to_rfc3339()).ok_or_else(|| ArgoError::Provider("timestamp out of range".into()))
}

pub(super) fn opt_micros(v: Option<i64>) -> ArgoResult<Option<String>> {
    v.map(micros).transpose()
}

fn header<'a>(req: &'a str, key: &str) -> Option<&'a str> {
    req.lines().find_map(|l| l.split_once(':').and_then(|(k, v)| k.eq_ignore_ascii_case(key).then(|| v.trim())))
}

fn cookie(req: &str, key: &str) -> Option<String> {
    header(req, "Cookie")?.split(';').find_map(|part| {
        let (k, v) = part.trim().split_once('=')?;
        (k == key).then(|| v.to_string())
    })
}

fn default_tenant() -> Option<String> {
    std::env::var("ARGO_WEBINBOX_TENANT_ID").ok().filter(|s| !s.is_empty()).or_else(|| std::env::var("ARGO_DEFAULT_TENANT").ok().filter(|s| !s.is_empty()))
}

fn target_has_tenant(req: &str) -> bool {
    request_line(req)
        .and_then(|(_, target)| target.split_once('?').map(|(_, raw)| raw))
        .is_some_and(|raw| raw.split('&').any(|kv| kv.split_once('=').unwrap_or((kv, "")).0 == "tenant"))
}

fn body_has_tenant(req: &str) -> bool {
    serde_json::from_str::<Value>(super::body_from_request(req)).ok().and_then(|v| v.get("tenant_id").cloned()).is_some()
}

pub(super) fn to_body(v: &impl serde::Serialize) -> String {
    serde_json::to_string(v).unwrap_or_else(|_| "{}".to_string())
}

pub(super) fn bad(msg: &str) -> (&'static str, String) {
    ("400 Bad Request", json!({"error":msg}).to_string())
}
pub(super) fn bad_owned(msg: String) -> (&'static str, String) { ("400 Bad Request", json!({"error":msg}).to_string()) }
fn unauthorized(msg: &str) -> (&'static str, String) { ("401 Unauthorized", json!({"error":msg}).to_string()) }
fn forbidden(msg: &str) -> (&'static str, String) { ("403 Forbidden", json!({"error":msg}).to_string()) }
pub(super) fn not_found() -> (&'static str, String) { ("404 Not Found", json!({"error":"not found"}).to_string()) }
pub(super) fn server(e: impl std::fmt::Display) -> (&'static str, String) { ("500 Internal Server Error", json!({"error":e.to_string()}).to_string()) }
pub(super) fn db(e: impl std::fmt::Display) -> ArgoError { ArgoError::Provider(e.to_string()) }
pub(super) fn conflict_or_server(e: ArgoError) -> (&'static str, String) {
    if matches!(e, ArgoError::Invariant(_)) {
        ("409 Conflict", json!({"error":e.to_string()}).to_string())
    } else {
        server(e)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use argo_identity::WorkerId;
    use argo_orchestrator::{EscalationStep, Handoff};
    static ENV_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

    fn req(method: &str, target: &str, token: Option<&str>, body: &str) -> String {
        let auth = token.map(|t| format!("Authorization: Bearer {t}\r\n")).unwrap_or_default();
        format!("{method} {target} HTTP/1.1\r\n{auth}\r\n{body}")
    }

    async fn store() -> Arc<SessionStore> {
        let store = SessionStore::open(":memory:").await.unwrap();
        store.migrate().await.unwrap();
        Arc::new(store)
    }

    #[tokio::test]
    async fn orchestrator_routes_auth_and_lifecycle() {
        let _guard = ENV_LOCK.lock().await;
        std::env::set_var("ARGO_WEBINBOX_BEARER_TOKEN", "tok");
        std::env::set_var("ARGO_WEBINBOX_TENANT_ID", "019aff00-0000-7000-8000-000000000101");
        let store = store().await;
        let tenant = "019aff00-0000-7000-8000-000000000101";
        let other = "019aff00-0000-7000-8000-000000000102";
        assert_eq!(dispatch_route(&req("GET", &format!("/sessions?tenant={tenant}"), None, ""), Some(&store)).await.unwrap().0, "401 Unauthorized");
        assert_eq!(dispatch_route(&req("GET", &format!("/sessions?tenant={tenant}"), Some("bad"), ""), Some(&store)).await.unwrap().0, "401 Unauthorized");
        assert_eq!(dispatch_route(&req("GET", &format!("/sessions?tenant={other}"), Some("tok"), ""), Some(&store)).await.unwrap().0, "403 Forbidden");
        assert_eq!(dispatch_route(&req("GET", &format!("/sessions?tenant={tenant}&oops=1"), Some("tok"), ""), Some(&store)).await.unwrap().0, "400 Bad Request");
        let (status, body) = dispatch_route(&req("POST", "/sessions", Some("tok"), &json!({"tenant_id":tenant,"title":"demo"}).to_string()), Some(&store)).await.unwrap();
        assert_eq!(status, "201 Created");
        let sid = serde_json::from_str::<Value>(&body).unwrap()["id"].as_str().unwrap().to_string();
        let repo = OrchestratorStore::new(Arc::clone(&store));
        let from = WorkerId::new();
        let to = WorkerId::new();
        let h = Handoff::new(argo_orchestrator::SessionId::from_string(sid.clone()), from, EscalationStep::Worker(to), "finish", json!({"k":1}));
        repo.insert_handoff(TenantId(uuid::Uuid::parse_str(tenant).unwrap()), &h).await.unwrap();
        for target in [format!("/sessions?tenant={tenant}&status=active"), format!("/sessions/{sid}?tenant={tenant}"), format!("/handoffs?tenant={tenant}&session={sid}"), format!("/handoffs/{}?tenant={tenant}", h.id)] {
            assert_eq!(dispatch_route(&req("GET", &target, Some("tok"), ""), Some(&store)).await.unwrap().0, "200 OK");
        }
        assert_eq!(dispatch_route(&req("PATCH", &format!("/handoffs/{}/status?tenant={tenant}", h.id), Some("tok"), r#"{"status":"acknowledged"}"#), Some(&store)).await.unwrap().0, "200 OK");
        assert_eq!(dispatch_route(&req("PATCH", &format!("/sessions/{sid}/status?tenant={tenant}"), Some("tok"), r#"{"status":"resolved"}"#), Some(&store)).await.unwrap().0, "200 OK");
        assert_eq!(dispatch_route(&req("PATCH", &format!("/sessions/{sid}/status?tenant={tenant}"), Some("tok"), r#"{"status":"cancelled"}"#), Some(&store)).await.unwrap().0, "409 Conflict");
        std::env::remove_var("ARGO_WEBINBOX_BEARER_TOKEN");
        std::env::remove_var("ARGO_WEBINBOX_TENANT_ID");
    }
}
