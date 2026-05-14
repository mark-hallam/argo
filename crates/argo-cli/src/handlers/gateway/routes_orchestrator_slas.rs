//! SLA sub-resource handlers for `routes_orchestrator`.
use super::routes_orchestrator::{
    bad, bad_owned, conflict_or_server, not_found, path_id, server, to_body,
};
use argo_orchestrator::sla::{BreachAction, EscalationPolicy, Sla, SlaStore, SlaTarget};
use argo_store::SessionStore;
use argo_types::TenantId;
use chrono::Utc;
use serde_json::{json, Value};
use std::sync::Arc;

pub(super) fn is_sla_path(method: &str, path: &str) -> bool {
    path == "/slas" || (path.starts_with("/slas/") && matches!(method, "GET" | "PATCH" | "DELETE"))
}

pub(super) async fn dispatch(
    method: &str,
    path: &str,
    store: &Arc<SessionStore>,
    tenant: TenantId,
    body: Option<&Value>,
) -> Result<(&'static str, String), (&'static str, String)> {
    match (method, path) {
        ("GET", "/slas") => list_slas(store, tenant).await,
        ("POST", "/slas") => create_sla(store, tenant, body).await,
        ("GET", p) if p.starts_with("/slas/") => show_sla(store, tenant, p).await,
        ("PATCH", p) if p.starts_with("/slas/") => patch_sla(store, tenant, p, body).await,
        ("DELETE", p) if p.starts_with("/slas/") => delete_sla(store, tenant, p).await,
        _ => Err(not_found()),
    }
}

async fn list_slas(
    store: &Arc<SessionStore>,
    tenant: TenantId,
) -> Result<(&'static str, String), (&'static str, String)> {
    let slas = SlaStore::new(Arc::clone(store))
        .list(tenant)
        .await
        .map_err(server)?;
    Ok(("200 OK", to_body(&json!({"slas": slas}))))
}

async fn create_sla(
    store: &Arc<SessionStore>,
    tenant: TenantId,
    body: Option<&Value>,
) -> Result<(&'static str, String), (&'static str, String)> {
    let sla = sla_from_body(tenant, body)?;
    SlaStore::new(Arc::clone(store))
        .create(&sla)
        .await
        .map_err(server)?;
    Ok(("201 Created", to_body(&json!({"sla": sla}))))
}

async fn show_sla(
    store: &Arc<SessionStore>,
    tenant: TenantId,
    path: &str,
) -> Result<(&'static str, String), (&'static str, String)> {
    let id = path_id(path, "/slas/")?;
    let sla = SlaStore::new(Arc::clone(store))
        .get(tenant, id)
        .await
        .map_err(server)?
        .ok_or_else(not_found)?;
    Ok(("200 OK", to_body(&json!({"sla": sla}))))
}

async fn patch_sla(
    store: &Arc<SessionStore>,
    tenant: TenantId,
    path: &str,
    body: Option<&Value>,
) -> Result<(&'static str, String), (&'static str, String)> {
    let id = path_id(path, "/slas/")?;
    let repo = SlaStore::new(Arc::clone(store));
    let mut sla = repo
        .get(tenant, id)
        .await
        .map_err(server)?
        .ok_or_else(not_found)?;
    let body = body.ok_or_else(|| bad("missing body"))?;
    if let Some(v) = body.get("target_latency_ms").and_then(Value::as_u64) {
        sla.target_latency_ms = v;
    }
    if let Some(v) = body.get("enabled").and_then(Value::as_bool) {
        sla.enabled = v;
    }
    if body.get("escalation_policy").is_some() {
        sla.escalation_policy = parse_policy(body)?;
    }
    if body.get("breach_action").is_some() {
        sla.breach_action = parse_action(body)?;
    }
    sla.updated_at = Utc::now();
    repo.update(&sla).await.map_err(conflict_or_server)?;
    Ok(("200 OK", to_body(&json!({"sla": sla}))))
}

async fn delete_sla(
    store: &Arc<SessionStore>,
    tenant: TenantId,
    path: &str,
) -> Result<(&'static str, String), (&'static str, String)> {
    let id = path_id(path, "/slas/")?;
    if !SlaStore::new(Arc::clone(store))
        .delete(tenant, id)
        .await
        .map_err(server)?
    {
        return Err(not_found());
    }
    Ok(("200 OK", to_body(&json!({"deleted": true}))))
}

fn sla_from_body(tenant: TenantId, body: Option<&Value>) -> Result<Sla, (&'static str, String)> {
    let body = body.ok_or_else(|| bad("missing body"))?;
    let target = parse_target(body)?;
    let latency = body
        .get("target_latency_ms")
        .and_then(Value::as_u64)
        .ok_or_else(|| bad("missing target_latency_ms"))?;
    Ok(Sla::new(
        tenant,
        target,
        latency,
        parse_policy(body)?,
        parse_action(body)?,
    ))
}

fn parse_target(body: &Value) -> Result<SlaTarget, (&'static str, String)> {
    let kind = body
        .get("target_kind")
        .and_then(Value::as_str)
        .ok_or_else(|| bad("missing target_kind"))?;
    let id = body
        .get("target_id")
        .and_then(Value::as_str)
        .ok_or_else(|| bad("missing target_id"))?;
    SlaTarget::from_parts(kind, id).map_err(|e| bad_owned(e.to_string()))
}

fn parse_policy(body: &Value) -> Result<EscalationPolicy, (&'static str, String)> {
    let raw = body
        .get("escalation_policy")
        .and_then(Value::as_str)
        .unwrap_or("notify");
    let target = body
        .get("escalation_target")
        .and_then(Value::as_str)
        .unwrap_or("operator");
    Ok(match raw {
        "notify" => EscalationPolicy::Notify(target.to_string()),
        "reroute" => EscalationPolicy::Reroute(target.to_string()),
        "fail" => EscalationPolicy::Fail,
        other if other.contains(':') => EscalationPolicy::decode(other),
        _ => return Err(bad("escalation_policy must be notify|reroute|fail")),
    })
}

fn parse_action(body: &Value) -> Result<BreachAction, (&'static str, String)> {
    match body
        .get("breach_action")
        .and_then(Value::as_str)
        .unwrap_or("notify")
    {
        "notify" => Ok(BreachAction::Notify),
        "reroute" => Ok(BreachAction::Reroute),
        "fail" => Ok(BreachAction::Fail),
        _ => Err(bad("breach_action must be notify|reroute|fail")),
    }
}
