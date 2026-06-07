//! Tests for `routes_skill_ir` (Track L brief: `L4-gw`).
//!
//! Seeds an in-memory [`SessionStore`], applies the workspace
//! migration registry plus the `skill_ir` table's
//! [`argo_skill::ir::ensure_schema`], then writes a small
//! [`SkillGraph`] per tenant before driving the dispatcher
//! end-to-end. The bearer-auth gate runs upstream in
//! `routes::dispatch_route`; these tests target the handler-and-
//! dispatcher pair directly.

use super::{dispatch, dispatch_post};
use argo_skill::ir::SkillIrStore as _;
use argo_skill::ir::{
    canonical_hash_hex, ensure_schema, export_skill_ir_bundle, Node, NodeAnnotations, NodeId,
    NodeKind, SkillGraph, SqliteSkillIrStore,
};
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
    format!("GET {path} HTTP/1.1\r\nHost: x\r\nX-Argo-Tenant-Id: {tenant}\r\n\r\n",)
}

fn post_with_tenant(path: &str, tenant: TenantId, body: &Value) -> String {
    let body = body.to_string();
    format!(
        "POST {path} HTTP/1.1\r\nHost: x\r\nX-Argo-Tenant-Id: {tenant}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
        body.len(),
    )
}

async fn seed(
    store: &Arc<SessionStore>,
    tenant: TenantId,
    skill: SkillId,
    version: &str,
    label: &str,
) {
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
    // path (`?as_tenant=...` w/o the cross-tenant capability/role) is
    // what produces 403.
    let store = mem_store().await;
    let owner = TenantId::new();
    let attacker = TenantId::new();
    let skill = SkillId::new();
    seed(&store, owner, skill, "v1", "secret").await;

    let req = get_with_tenant(&format!("/api/v1/skills/{skill}/ir?version=v1"), attacker);
    let (status, _) = dispatch(&req, Some(&store)).await.expect("dispatched");
    assert_eq!(status, "404 Not Found");

    // Explicit cross-tenant override by a caller WITHOUT the cross-tenant
    // capability returns 403. WS5 change: the override is now gated by the
    // real RBAC `Action::AuditCrossTenant` capability (owner-only), not a
    // self-asserted `x-argo-audit-cross-tenant` header. The in-crate test
    // build defaults a header-less request to Owner (which HAS the
    // capability), so we must stamp a non-owner role (`X-Argo-Role: guest`)
    // to exercise the deny path — guest lacks `AuditCrossTenant`, so
    // `cross_cap=false` and the override is refused.
    let req = format!(
        "GET /api/v1/skills/{skill}/ir?version=v1&as_tenant={owner} HTTP/1.1\r\n\
         Host: x\r\nX-Argo-Tenant-Id: {attacker}\r\nX-Argo-Role: guest\r\n\r\n",
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

#[tokio::test]
async fn post_ir_saves_new_version() {
    let store = mem_store().await;
    let tenant = TenantId::new();
    let skill = SkillId::new();
    let graph = graph_with_label("edited");
    let req = post_with_tenant(
        &format!("/api/v1/skills/{skill}/ir"),
        tenant,
        &serde_json::json!({ "graph": graph }),
    );

    let (status, body) = dispatch_post(&req, Some(&store)).await.expect("dispatched");
    assert_eq!(status, "200 OK");
    let parsed: Value = serde_json::from_str(&body).expect("valid JSON");
    assert_eq!(parsed["skillId"].as_str().unwrap(), skill.to_string());
    assert_eq!(parsed["tenantId"].as_str().unwrap(), tenant.to_string());
    assert!(!parsed["abTestScheduled"].as_bool().unwrap());
    assert!(parsed["version"].as_str().unwrap().starts_with("canvas-"));
}

#[tokio::test]
async fn post_ir_rejects_invalid_topology() {
    let store = mem_store().await;
    let tenant = TenantId::new();
    let skill = SkillId::new();
    let mut graph = graph_with_label("broken");
    graph.nodes.push(Node {
        id: NodeId::from("orphan"),
        label: "Orphan".into(),
        kind: NodeKind::Action {
            instruction: "unreachable".into(),
        },
        annotations: NodeAnnotations::default(),
    });
    let req = post_with_tenant(
        &format!("/api/v1/skills/{skill}/ir"),
        tenant,
        &serde_json::json!({ "graph": graph }),
    );

    let (status, body) = dispatch_post(&req, Some(&store)).await.expect("dispatched");
    assert_eq!(status, "400 Bad Request");
    assert!(body.contains("orphaned"));
}

#[tokio::test]
async fn post_ir_second_save_reports_ab_schedule() {
    let store = mem_store().await;
    let tenant = TenantId::new();
    let skill = SkillId::new();
    seed(&store, tenant, skill, "v1", "old").await;
    let req = post_with_tenant(
        &format!("/api/v1/skills/{skill}/ir"),
        tenant,
        &serde_json::json!({ "graph": graph_with_label("new") }),
    );

    let (status, body) = dispatch_post(&req, Some(&store)).await.expect("dispatched");
    assert_eq!(status, "200 OK");
    let parsed: Value = serde_json::from_str(&body).expect("valid JSON");
    assert!(parsed["abTestScheduled"].as_bool().unwrap());
    assert!(parsed["abTestRunId"].as_str().unwrap().len() > 20);
}

#[tokio::test]
async fn export_endpoint_returns_portable_bundle_without_tenant() {
    let store = mem_store().await;
    let tenant = TenantId::new();
    let skill = SkillId::new();
    seed(&store, tenant, skill, "v1", "portable").await;

    let req = get_with_tenant(
        &format!("/api/v1/skills/{skill}/ir/export?version=v1"),
        tenant,
    );
    let (status, body) = dispatch(&req, Some(&store)).await.expect("dispatched");
    assert_eq!(status, "200 OK");
    let parsed: Value = serde_json::from_str(&body).expect("valid JSON");
    assert_eq!(
        parsed["schemaVersion"].as_str().unwrap(),
        "skycot.skill_ir.bundle.v1"
    );
    assert_eq!(parsed["sourceSkillId"].as_str().unwrap(), skill.to_string());
    assert_eq!(parsed["version"].as_str().unwrap(), "v1");
    assert_eq!(
        parsed["irHash"].as_str().unwrap(),
        canonical_hash_hex(&graph_with_label("portable")).unwrap()
    );
    assert!(!body.contains(&tenant.to_string()));
}

#[tokio::test]
async fn import_endpoint_adopts_bundle_under_target_skill() {
    let store = mem_store().await;
    let tenant = TenantId::new();
    let source = SkillId::new();
    let target = SkillId::new();
    let graph = graph_with_label("imported");
    let bundle = export_skill_ir_bundle(source, "v1", &graph).expect("bundle");
    let req = post_with_tenant(
        &format!("/api/v1/skills/{target}/ir/import"),
        tenant,
        &serde_json::json!({ "bundle": bundle }),
    );

    let (status, body) = dispatch_post(&req, Some(&store)).await.expect("dispatched");
    assert_eq!(status, "200 OK");
    let parsed: Value = serde_json::from_str(&body).expect("valid JSON");
    assert_eq!(parsed["skillId"].as_str().unwrap(), target.to_string());
    assert_eq!(
        parsed["importedHash"].as_str().unwrap(),
        canonical_hash_hex(&graph).unwrap()
    );
    let saved = SqliteSkillIrStore::new(&store)
        .latest(tenant, target)
        .await
        .expect("latest")
        .expect("saved")
        .1;
    assert_eq!(saved, graph);
}

/// Regression: the demo seed writes a real IR per marketplace catalog
/// skill, and `GET /api/v1/skills/<catalog-id>/ir` must return it (not
/// 404) when keyed on the SAME id the `/api/v1/skills` list surfaces.
///
/// This drives the exact dispatcher path the live SPA hits: seed the
/// catalog + IR under a tenant, decode the catalog ids the way
/// `routes_skills_list_get` returns them (the `skill.id` blob →
/// hyphenated UUID string), then issue the IR GET for each id with that
/// tenant header and assert a real `200` IR envelope.
///
/// Guards the id-space the 2026-05-30 "skill has no IR" bug note
/// suspected: the seed-IR writer and the IR GET reader agree on the
/// marketplace catalog id, end-to-end through the real dispatcher.
#[tokio::test]
async fn seeded_catalog_ir_is_found_by_catalog_id_via_get() {
    use argo_demo_seed::{demo_marketplace_catalog_size, seed_demo_marketplace_skills};

    let store = mem_store().await;
    let tenant = TenantId::new();

    // Seed the marketplace catalog + a real IR per catalog skill — the
    // exact two seed fns `run_canonical_v1_seed` invokes for this tenant.
    seed_demo_marketplace_skills(&store, tenant)
        .await
        .expect("seed marketplace catalog");
    let seeded = argo_demo_seed::seed_skill_ir(&store, tenant)
        .await
        .expect("seed skill IR");
    assert_eq!(
        seeded,
        demo_marketplace_catalog_size(),
        "one IR graph per catalog skill"
    );

    // Decode the catalog ids exactly as `GET /api/v1/skills` does:
    // the `skill.id` 16-byte blob → hyphenated UUID string. These are
    // the ids the SPA lists and then drills into for IR.
    let id_blobs: Vec<Vec<u8>> = sqlx::query_scalar(
        "SELECT id FROM skill \
         WHERE tenant_id = ? AND lifecycle_state = 'active' AND superseded_by IS NULL \
         ORDER BY name ASC",
    )
    .bind(tenant.to_string())
    .fetch_all(store.pool())
    .await
    .expect("read catalog ids");
    let catalog_ids: Vec<String> = id_blobs
        .iter()
        .filter(|b| b.len() == 16)
        .map(|b| {
            let mut a = [0u8; 16];
            a.copy_from_slice(b);
            uuid::Uuid::from_bytes(a).to_string()
        })
        .collect();
    assert_eq!(
        catalog_ids.len(),
        demo_marketplace_catalog_size(),
        "every seeded catalog row decodes to a UUID"
    );

    // Every catalog id must resolve to a real IR through the dispatcher
    // — the precise path + tenant header the SPA's IR drill-in uses.
    for id in &catalog_ids {
        let req = get_with_tenant(&format!("/api/v1/skills/{id}/ir"), tenant);
        let (status, body) = dispatch(&req, Some(&store)).await.expect("dispatched");
        assert_eq!(
            status, "200 OK",
            "GET /api/v1/skills/{id}/ir should return seeded IR, not 404 (body: {body})"
        );
        let parsed: Value = serde_json::from_str(&body).expect("valid JSON");
        assert_eq!(
            parsed["skillId"].as_str().unwrap(),
            id.as_str(),
            "envelope skillId echoes the catalog id"
        );
        // The seeded graph is the entry-only `Action` node — confirm it
        // is the real seeded IR, not an empty/placeholder shape.
        assert_eq!(
            parsed["graph"]["nodes"][0]["kind"]["type"].as_str().unwrap(),
            "action",
            "seeded IR carries the real entry Action node"
        );
    }
}

/// Regression for the live 2026-05-30 "skill has no IR" 404: when the
/// SPA omits `X-Argo-Tenant-Id`, the IR GET must fall back to the SAME
/// tenant the `/api/v1/skills` list uses (the cwd-bound workspace
/// tenant), NOT the nil dev-bypass default. Pre-fix the IR route
/// resolved the nil tenant on a header-less request while the list
/// resolved the cwd workspace tenant — so the seeded IR (written under
/// the workspace tenant) was invisible and the drill-in 404'd.
///
/// Drives the real dispatcher with NO tenant header and asserts the IR
/// seeded under the cwd-workspace tenant is found.
#[tokio::test]
async fn header_less_get_resolves_workspace_tenant_like_skills_list() {
    let store = mem_store().await;

    // Resolve the cwd-bound workspace tenant the way the `/api/v1/skills`
    // list endpoint does for a header-less request, then seed IR under
    // exactly that tenant (mirroring the live seed → list visibility).
    let cwd = std::env::current_dir().expect("cwd");
    let root = cwd.to_string_lossy();
    let name = cwd.file_name().and_then(|s| s.to_str()).unwrap_or("ws");
    let (tenant, _) = store.ensure_workspace(name, &root).await.expect("workspace");
    let skill = SkillId::new();
    seed(&store, tenant, skill, "v1", "workspace-scoped").await;

    // Header-less GET (no `X-Argo-Tenant-Id`) — the failure mode the
    // SPA hit. Must resolve the workspace tenant and find the IR.
    let req = format!("GET /api/v1/skills/{skill}/ir HTTP/1.1\r\nHost: x\r\n\r\n");
    let (status, body) = dispatch(&req, Some(&store)).await.expect("dispatched");
    assert_eq!(
        status, "200 OK",
        "header-less IR GET must resolve the cwd workspace tenant, not nil (body: {body})"
    );
    let parsed: Value = serde_json::from_str(&body).expect("valid JSON");
    assert_eq!(parsed["tenantId"].as_str().unwrap(), tenant.to_string());
}

#[tokio::test]
async fn import_endpoint_rejects_bad_bundle_hash() {
    let store = mem_store().await;
    let tenant = TenantId::new();
    let target = SkillId::new();
    let mut bundle =
        export_skill_ir_bundle(SkillId::new(), "v1", &graph_with_label("old")).unwrap();
    bundle.graph = graph_with_label("new");
    let req = post_with_tenant(
        &format!("/api/v1/skills/{target}/ir/import"),
        tenant,
        &serde_json::json!({ "bundle": bundle }),
    );

    let (status, body) = dispatch_post(&req, Some(&store)).await.expect("dispatched");
    assert_eq!(status, "400 Bad Request");
    assert!(body.contains("hash mismatch"));
}
