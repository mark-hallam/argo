//! Regression tests for [`super::routes_skill_refine`].
//!
//! The audit found three missing wires on `POST /skills/<name>/refine`:
//!
//! 1. No RBAC gate — any caller that passes the upstream bearer gate
//!    can mutate skill rows. Other mutating gateway routes call
//!    `enforce_role(req, Action::<X>)`; `skill_refine_response` does
//!    not.
//! 2. No audit row — the handler supersedes the active skill row but
//!    writes nothing to `audit_record`. Mission claim 6 requires
//!    "audit chain on every action".
//! 3. No prior test coverage — the source file has no `#[cfg(test)]`
//!    module, so the missing wires never tripped CI.
//!
use super::skill_refine_response;
use argo_audit::AuditKind;
use argo_skill::SkillStore;
use argo_store::SessionStore;
use serde_json::{json, Value};
use std::sync::Arc;

async fn session_store() -> Arc<SessionStore> {
    let store = SessionStore::open(":memory:").await.expect("open mem db");
    store.migrate().await.expect("migrate");
    Arc::new(store)
}

fn skill_md(name: &str, source: &str) -> String {
    format!(
        "---\nname: {name}\nversion: \"0.1.0\"\ndescription: {name} description\n\
         provenance:\n  source: {source}\n  hash: sha256-deadbeef\n---\nbody of {name}\n"
    )
}

async fn install(store: &Arc<SessionStore>, name: &str, source: &str) {
    SkillStore::new(Arc::clone(store))
        .install(&skill_md(name, source), None)
        .await
        .expect("install skill");
}

async fn install_for_tenant(store: &Arc<SessionStore>, name: &str, source: &str, tenant_id: &str) {
    SkillStore::new(Arc::clone(store))
        .install_for_tenant(&skill_md(name, source), tenant_id, None)
        .await
        .expect("install tenant skill");
}

fn refine_post(name: &str, role_header: Option<&str>, hint: &str) -> String {
    refine_post_for_tenant(name, role_header, None, hint)
}

fn refine_post_for_tenant(
    name: &str,
    role_header: Option<&str>,
    tenant_header: Option<&str>,
    hint: &str,
) -> String {
    let body = json!({ "hint": hint }).to_string();
    let role_line = role_header
        .map(|r| format!("X-Argo-Role: {r}\r\n"))
        .unwrap_or_default();
    let tenant_line = tenant_header
        .map(|t| format!("X-Argo-Tenant-Id: {t}\r\n"))
        .unwrap_or_default();
    format!(
        "POST /skills/{name}/refine HTTP/1.1\r\nHost: x\r\n{role_line}{tenant_line}\
         Content-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    )
}

async fn audit_row_count(store: &Arc<SessionStore>) -> i64 {
    sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM audit_record")
        .fetch_one(store.pool())
        .await
        .unwrap_or(0)
}

async fn latest_audit_row(store: &Arc<SessionStore>) -> (String, String, Value) {
    let (kind, tenant_id, payload_json): (String, String, String) = sqlx::query_as(
        "SELECT kind, tenant_id, payload_json FROM audit_record ORDER BY seq DESC LIMIT 1",
    )
    .fetch_one(store.pool())
    .await
    .expect("latest audit row");
    let payload = serde_json::from_str(&payload_json).expect("audit payload json");
    (kind, tenant_id, payload)
}

#[tokio::test]
async fn refine_with_guest_role_returns_403() {
    let store = session_store().await;
    install(&store, "alpha", "learned").await;

    let req = refine_post("alpha", Some("guest"), "tighten the close phrase");
    let (status, body) = skill_refine_response("alpha", &req, Some(&store)).await;

    assert_eq!(
        status, "403 Forbidden",
        "guest must be refused (post-fix); today body={body}"
    );
    let parsed: serde_json::Value = serde_json::from_str(&body).expect("json");
    assert_eq!(parsed["rbac"]["denied"], true);
}

#[tokio::test]
async fn refine_emits_audit_row() {
    let store = session_store().await;
    install_for_tenant(&store, "alpha", "learned", "tenant-refine").await;
    let before = audit_row_count(&store).await;

    let req = refine_post_for_tenant(
        "alpha",
        None,
        Some("tenant-refine"),
        "tighten the close phrase",
    );
    let (status, _body) = skill_refine_response("alpha", &req, Some(&store)).await;
    assert_eq!(status, "200 OK");

    let after = audit_row_count(&store).await;
    assert!(
        after > before,
        "successful refine must append ≥1 audit row (post-fix); before={before} after={after}"
    );
    let (kind, tenant_id, payload) = latest_audit_row(&store).await;
    assert_eq!(kind, AuditKind::HttpRequest.label());
    assert_eq!(tenant_id, "tenant-refine");
    assert_eq!(payload["tenant_id"], "tenant-refine");
    assert_eq!(payload["endpoint"], "skill.refined");
    assert_eq!(payload["name"], "alpha");
    assert_eq!(payload["prev_version"], "0.1.0");
    assert_eq!(payload["new_version"], "0.1.1");
    assert!(payload["previous_skill_id"].is_string());
    assert!(payload["skill_id"].is_string());
}

#[tokio::test]
async fn refine_cross_tenant_skill_returns_404_and_no_audit() {
    let store = session_store().await;
    install_for_tenant(&store, "alpha", "learned", "tenant-a").await;
    let before = audit_row_count(&store).await;

    let req = refine_post_for_tenant("alpha", None, Some("tenant-b"), "rewrite the examples");
    let (status, body) = skill_refine_response("alpha", &req, Some(&store)).await;

    assert_eq!(status, "404 Not Found");
    assert!(body.contains("no active skill named 'alpha'"));
    assert_eq!(audit_row_count(&store).await, before);

    let active = SkillStore::new(Arc::clone(&store))
        .find_active_by_name("alpha")
        .await
        .expect("find active")
        .expect("active skill remains");
    assert_eq!(active.frontmatter.version, "0.1.0");
}
