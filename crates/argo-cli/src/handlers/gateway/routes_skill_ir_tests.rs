//! Tests for `routes_skill_ir` (Track L brief: `L4-gw`).
//!
//! Seeds an in-memory [`SessionStore`], applies the workspace
//! migration registry plus the `skill_ir` table's
//! [`argo_skill::ir::ensure_schema`], then writes a small
//! [`SkillGraph`] per tenant before driving the dispatcher
//! end-to-end. The bearer-auth gate runs upstream in
//! `routes::dispatch_route`; these tests target the handler-and-
//! dispatcher pair directly.

use super::dispatch;
use argo_skill::ir::{
    Node, NodeAnnotations, NodeId, NodeKind, SkillGraph, SqliteSkillIrStore, canonical_hash_hex,
    ensure_schema,
};
use argo_skill::ir::SkillIrStore as _;
use argo_store::SessionStore;
use argo_types::{SkillId, TenantId};
use serde_json::Value;
use std::sync::Arc;

async fn mem_store() -> Arc<SessionStore> {
    let store = SessionStore::open(":memory:").await.expect("open mem db");
    store.migrate().await.expect("migrate");
    ensure_schema(&store).await.expect("ensure skill_ir schema");
    Arc::new(store)
}

fn graph_with_label(label: &str) -> SkillGraph {
    SkillGraph {
        entry: NodeId::from("n1"),
        nodes: vec![Node {
            id: NodeId::from("n1"),
            label: label.into(),
            kind: NodeKind::Action {
                instruction: format!("do {label}"),
            },
            annotations: NodeAnnotations::default(),
        }],
        edges: vec![],
        variables: vec![],
        metadata: Default::default(),
    }
}

fn get_with_tenant(path: &str, tenant: TenantId) -> String {
    format!(
        "GET {path} HTTP/1.1\r\nHost: x\r\nX-Argo-Tenant-Id: {tenant}\r\n\r\n",
    )
}

async fn seed(store: &Arc<SessionStore>, tenant: TenantId, skill: SkillId, version: &str, label: &str) {
    let ir = SqliteSkillIrStore::new(store);
    ir.put(tenant, skill, version, &graph_with_label(label))
        .await
        .expect("put IR");
}

#[tokio::test]
async fn ir_endpoint_returns_camelcase_envelope_for_existing_version() {
    let store = mem_store().await;
    let tenant = TenantId::new();
    let skill = SkillId::new();
    seed(&store, tenant, skill, "v1", "alpha").await;

    let req = get_with_tenant(&format!("/api/v1/skills/{skill}/ir?version=v1"), tenant);
    let (status, body) = dispatch(&req, Some(&store)).await.expect("dispatched");
    assert_eq!(status, "200 OK");
    let parsed: Value = serde_json::from_str(&body).expect("valid JSON");
    assert_eq!(parsed["skillId"].as_str().unwrap(), skill.to_string());
    assert_eq!(parsed["tenantId"].as_str().unwrap(), tenant.to_string());
    assert_eq!(parsed["version"].as_str().unwrap(), "v1");
    let expected_hash = canonical_hash_hex(&graph_with_label("alpha")).unwrap();
    assert_eq!(parsed["irHash"].as_str().unwrap(), expected_hash);
    // Graph round-trip is camelCase (TS-compatible): the `Action`
    // variant tag carries the kebab-cased `type` discriminator, and
    // node-level fields use camelCase.
    let graph = &parsed["graph"];
    assert_eq!(graph["entry"].as_str().unwrap(), "n1");
    let first_node = &graph["nodes"][0];
    assert_eq!(first_node["id"].as_str().unwrap(), "n1");
    assert_eq!(first_node["kind"]["type"].as_str().unwrap(), "action");
    assert_eq!(
        first_node["kind"]["instruction"].as_str().unwrap(),
        "do alpha"
    );
}

#[tokio::test]
async fn ir_endpoint_without_version_returns_latest() {
    let store = mem_store().await;
    let tenant = TenantId::new();
    let skill = SkillId::new();
    seed(&store, tenant, skill, "v1", "old").await;
    // Force monotonic created_at so the latest tie-break is by
    // version DESC, matching the L2 store's ordering.
    tokio::time::sleep(std::time::Duration::from_millis(2)).await;
    seed(&store, tenant, skill, "v2", "new").await;

    let req = get_with_tenant(&format!("/api/v1/skills/{skill}/ir"), tenant);
    let (status, body) = dispatch(&req, Some(&store)).await.expect("dispatched");
    assert_eq!(status, "200 OK");
    let parsed: Value = serde_json::from_str(&body).expect("valid JSON");
    assert_eq!(parsed["version"].as_str().unwrap(), "v2");
}

#[tokio::test]
async fn ir_endpoint_returns_404_for_missing_skill_and_missing_version() {
    let store = mem_store().await;
    let tenant = TenantId::new();
    let known = SkillId::new();
    seed(&store, tenant, known, "v1", "only").await;

    // Missing skill entirely.
    let unknown = SkillId::new();
    let req = get_with_tenant(&format!("/api/v1/skills/{unknown}/ir"), tenant);
    let (status, _) = dispatch(&req, Some(&store)).await.expect("dispatched");
    assert_eq!(status, "404 Not Found");

    // Known skill, missing version.
    let req = get_with_tenant(
        &format!("/api/v1/skills/{known}/ir?version=does-not-exist"),
        tenant,
    );
    let (status, _) = dispatch(&req, Some(&store)).await.expect("dispatched");
    assert_eq!(status, "404 Not Found");
}

#[tokio::test]
async fn ir_endpoint_returns_404_on_cross_tenant_read() {
    // Cross-tenant access without capability resolves to 404 — the
    // store's tenant-scoped lookup returns None and we deliberately
    // don't leak existence across tenants. The explicit override
    // path (`?as_tenant=...` w/o the capability header) is what
    // produces 403.
    let store = mem_store().await;
    let owner = TenantId::new();
    let attacker = TenantId::new();
    let skill = SkillId::new();
    seed(&store, owner, skill, "v1", "secret").await;

    let req = get_with_tenant(&format!("/api/v1/skills/{skill}/ir?version=v1"), attacker);
    let (status, _) = dispatch(&req, Some(&store)).await.expect("dispatched");
    assert_eq!(status, "404 Not Found");

    // Explicit cross-tenant override without the capability header
    // returns 403.
    let req = format!(
        "GET /api/v1/skills/{skill}/ir?version=v1&as_tenant={owner} HTTP/1.1\r\n\
         Host: x\r\nX-Argo-Tenant-Id: {attacker}\r\n\r\n",
    );
    let (status, _) = dispatch(&req, Some(&store)).await.expect("dispatched");
    assert_eq!(status, "403 Forbidden");
}

#[tokio::test]
async fn versions_endpoint_returns_versions_with_hashes_newest_first() {
    let store = mem_store().await;
    let tenant = TenantId::new();
    let skill = SkillId::new();
    seed(&store, tenant, skill, "v1", "one").await;
    tokio::time::sleep(std::time::Duration::from_millis(2)).await;
    seed(&store, tenant, skill, "v2", "two").await;

    let req = get_with_tenant(&format!("/api/v1/skills/{skill}/ir/versions"), tenant);
    let (status, body) = dispatch(&req, Some(&store)).await.expect("dispatched");
    assert_eq!(status, "200 OK");
    let parsed: Value = serde_json::from_str(&body).expect("valid JSON");
    let items = parsed["versions"].as_array().expect("versions array");
    assert_eq!(items.len(), 2);
    assert_eq!(items[0]["version"].as_str().unwrap(), "v2");
    assert_eq!(items[1]["version"].as_str().unwrap(), "v1");
    assert_eq!(
        items[0]["irHash"].as_str().unwrap(),
        canonical_hash_hex(&graph_with_label("two")).unwrap()
    );
    assert_eq!(
        items[1]["irHash"].as_str().unwrap(),
        canonical_hash_hex(&graph_with_label("one")).unwrap()
    );

    // Unknown skill returns 404 on the versions path too.
    let unknown = SkillId::new();
    let req = get_with_tenant(&format!("/api/v1/skills/{unknown}/ir/versions"), tenant);
    let (status, _) = dispatch(&req, Some(&store)).await.expect("dispatched");
    assert_eq!(status, "404 Not Found");
}
