use super::list_response;
use argo_store::workflow_definition as wd;
use argo_store::SessionStore;
use argo_types::TenantId;
use serde_json::Value;
use std::sync::Arc;

const TOML: &str = r#"
schema_version = "1"

[workflow]
name = "seed"

[[step]]
name = "scan"
kind = "skill_invoke"
skill = "reviewer"
"#;

async fn fresh_store() -> Arc<SessionStore> {
    let store = SessionStore::open(":memory:").await.expect("open");
    store.migrate().await.expect("migrate");
    Arc::new(store)
}

async fn seed_head(store: &Arc<SessionStore>, tenant: TenantId, vertical: &str, name: &str) {
    let id = wd::create(store.pool(), tenant, vertical, name, TOML, None, "test")
        .await
        .expect("create workflow");
    wd::set_head(store.pool(), tenant, vertical, name, &id)
        .await
        .expect("set head");
}

fn req(tenant: TenantId) -> String {
    format!("GET /api/v1/verticals HTTP/1.1\r\nHost: x\r\nX-Argo-Tenant-Id: {tenant}\r\n\r\n")
}

fn parse(body: &str) -> Value {
    serde_json::from_str(body).expect("json body")
}

#[tokio::test]
async fn empty_workspace_returns_empty_verticals_array() {
    let store = fresh_store().await;
    let tenant = TenantId::new();
    let (status, body) = list_response(&req(tenant), Some(&store)).await;
    assert_eq!(status, "200 OK");
    assert_eq!(parse(&body)["verticals"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn populated_workspace_returns_verticals_with_workflow_counts() {
    let store = fresh_store().await;
    let tenant = TenantId::new();
    seed_head(&store, tenant, "support", "triage").await;
    seed_head(&store, tenant, "support", "handoff").await;
    seed_head(&store, tenant, "finance-ar", "collect").await;

    let (status, body) = list_response(&req(tenant), Some(&store)).await;
    assert_eq!(status, "200 OK");
    let parsed = parse(&body);
    let rows = parsed["verticals"].as_array().unwrap();
    assert_eq!(rows.len(), 2, "body={body}");
    assert_eq!(rows[0]["id"], "finance-ar");
    assert_eq!(rows[0]["workflow_count"], 1);
    assert_eq!(rows[1]["id"], "support");
    assert_eq!(rows[1]["workflow_count"], 2);
    assert_eq!(rows[1]["name"], "Support");
}

#[tokio::test]
async fn tenant_b_verticals_do_not_include_tenant_a_rows() {
    let store = fresh_store().await;
    let tenant_a = TenantId::new();
    let tenant_b = TenantId::new();
    seed_head(&store, tenant_a, "support", "triage").await;
    seed_head(&store, tenant_b, "sales", "prospect").await;

    let (status, body) = list_response(&req(tenant_b), Some(&store)).await;
    assert_eq!(status, "200 OK");
    let parsed = parse(&body);
    let rows = parsed["verticals"].as_array().unwrap();
    assert_eq!(rows.len(), 1, "body={body}");
    assert_eq!(rows[0]["id"], "sales");
    assert_ne!(rows[0]["id"], "support", "tenant A row leaked: {body}");
}
