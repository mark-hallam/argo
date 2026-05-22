//! Unit + handler-level coverage for the workplace-knowledge gateway
//! shipped in `mission-workplace-knowledge-gateway`. Mirrors the
//! `routes_workflows_tests.rs` pattern: drive the handler functions
//! directly (bypassing the bearer/principal gates) so the contracts
//! that matter — RBAC floor, cross-tenant 404, audit-row diff
//! payload, category filter — can be asserted without standing up a
//! full HTTP harness. Dispatcher wiring is covered by the
//! parse_route tests below.

use super::super::role_gate::{enforce_role, Action};
use super::super::routes_knowledge_handlers as hh;
use super::{parse_route, Route};
use argo_audit::AuditKind;
use argo_store::SessionStore;
use argo_types::TenantId;
use argo_workplace_knowledge::doc;
use serde_json::Value;
use sqlx::Row;
use std::sync::Arc;

// ── Route-parser tests ──────────────────────────────────────────

#[test]
fn parse_list_route() {
    let req = "GET /api/v1/knowledge HTTP/1.1\r\nHost: x\r\n\r\n";
    let route = parse_route(req, "GET ").expect("list must parse");
    assert!(matches!(
        route,
        Route::List { category: None }
    ));
}

#[test]
fn parse_list_route_with_category_filter() {
    let req = "GET /api/v1/knowledge?category=brand HTTP/1.1\r\nHost: x\r\n\r\n";
    let route = parse_route(req, "GET ").expect("list filter must parse");
    match route {
        Route::List { category: Some(c) } => assert_eq!(c, "brand"),
        other => panic!("expected list+category; got {other:?}"),
    }
}

#[test]
fn parse_get_route() {
    let req = "GET /api/v1/knowledge/refund-policy HTTP/1.1\r\nHost: x\r\n\r\n";
    match parse_route(req, "GET ").expect("get must parse") {
        Route::Get { slug } => assert_eq!(slug, "refund-policy"),
        other => panic!("expected get; got {other:?}"),
    }
}

#[test]
fn parse_revisions_route() {
    let req = "GET /api/v1/knowledge/refund-policy/revisions HTTP/1.1\r\nHost: x\r\n\r\n";
    match parse_route(req, "GET ").expect("revisions must parse") {
        Route::Revisions { slug } => assert_eq!(slug, "refund-policy"),
        other => panic!("expected revisions; got {other:?}"),
    }
}

#[test]
fn parse_create_route() {
    let req = "POST /api/v1/knowledge HTTP/1.1\r\nHost: x\r\n\r\n";
    assert!(matches!(
        parse_route(req, "POST ").expect("create must parse"),
        Route::Create
    ));
}

#[test]
fn parse_update_route() {
    let req = "PUT /api/v1/knowledge/refund-policy HTTP/1.1\r\nHost: x\r\n\r\n";
    match parse_route(req, "PUT ").expect("update must parse") {
        Route::Update { slug } => assert_eq!(slug, "refund-policy"),
        other => panic!("expected update; got {other:?}"),
    }
}

#[test]
fn parse_delete_route() {
    let req = "DELETE /api/v1/knowledge/refund-policy HTTP/1.1\r\nHost: x\r\n\r\n";
    match parse_route(req, "DELETE ").expect("delete must parse") {
        Route::Delete { slug } => assert_eq!(slug, "refund-policy"),
        other => panic!("expected delete; got {other:?}"),
    }
}

#[test]
fn parse_route_rejects_non_knowledge_prefix() {
    let req = "GET /api/v1/something-else HTTP/1.1\r\nHost: x\r\n\r\n";
    assert!(parse_route(req, "GET ").is_none());
}

// ── RBAC-floor tests ────────────────────────────────────────────

#[test]
fn rbac_create_with_user_role_returns_403() {
    let req = "POST /api/v1/knowledge HTTP/1.1\r\nX-Argo-Role: user\r\n\r\n";
    let deny = enforce_role(req, Action::KnowledgeWrite).expect("user must be denied write");
    assert!(deny.0.starts_with("403"));
    assert!(deny.1.contains("knowledge.write"));
}

#[test]
fn rbac_create_with_guest_role_returns_403() {
    let req = "POST /api/v1/knowledge HTTP/1.1\r\nX-Argo-Role: guest\r\n\r\n";
    let deny = enforce_role(req, Action::KnowledgeWrite).expect("guest must be denied write");
    assert!(deny.0.starts_with("403"));
}

#[test]
fn rbac_create_with_admin_role_passes() {
    let req = "POST /api/v1/knowledge HTTP/1.1\r\nX-Argo-Role: admin\r\n\r\n";
    assert!(enforce_role(req, Action::KnowledgeWrite).is_none());
}

#[test]
fn rbac_delete_with_admin_role_returns_403() {
    // Per design doc §5 brief 3: DELETE is owner-only; admin cannot
    // perform the destructive head removal.
    let req = "DELETE /api/v1/knowledge/x HTTP/1.1\r\nX-Argo-Role: admin\r\n\r\n";
    let deny = enforce_role(req, Action::KnowledgeDelete).expect("admin must be denied delete");
    assert!(deny.0.starts_with("403"));
    assert!(deny.1.contains("knowledge.delete"));
}

#[test]
fn rbac_delete_with_owner_role_passes() {
    let req = "DELETE /api/v1/knowledge/x HTTP/1.1\r\nX-Argo-Role: owner\r\n\r\n";
    assert!(enforce_role(req, Action::KnowledgeDelete).is_none());
}

#[test]
fn rbac_read_floor_is_guest() {
    // All four canonical roles must pass the read gate; the handbook
    // feeds every agent's system prompt and guest auditors need to
    // see it.
    for role in ["owner", "admin", "user", "guest"] {
        let req = format!("GET /api/v1/knowledge HTTP/1.1\r\nX-Argo-Role: {role}\r\n\r\n");
        assert!(
            enforce_role(&req, Action::KnowledgeRead).is_none(),
            "role {role} must pass KnowledgeRead"
        );
    }
}

// ── Handler-level tests (in-memory store) ──────────────────────

async fn fixture_store() -> (Arc<SessionStore>, TenantId, TenantId) {
    let store = SessionStore::open(":memory:").await.expect("open mem db");
    store.migrate().await.expect("migrate");
    let store = Arc::new(store);
    let tenant_a = TenantId::new();
    let tenant_b = TenantId::new();
    (store, tenant_a, tenant_b)
}

#[tokio::test]
async fn list_returns_empty_for_fresh_tenant() {
    let (store, tenant, _) = fixture_store().await;
    let (status, body) = hh::handle_list(&store, tenant, None).await;
    assert!(status.starts_with("200"));
    let parsed: Value = serde_json::from_str(&body).expect("parse json");
    assert_eq!(parsed["count"], 0);
    assert_eq!(parsed["items"].as_array().expect("items").len(), 0);
}

#[tokio::test]
async fn create_then_list_returns_doc() {
    let (store, tenant, _) = fixture_store().await;
    let body = r#"{"slug":"brand-voice","title":"Brand Voice","category":"brand","body_markdown":"Be calm, be clear."}"#;
    let (status, _) = hh::handle_create(&store, tenant, body, "principal-1").await;
    assert!(status.starts_with("201"));
    let (status, list_body) = hh::handle_list(&store, tenant, None).await;
    assert!(status.starts_with("200"));
    let parsed: Value = serde_json::from_str(&list_body).expect("parse list");
    assert_eq!(parsed["count"], 1);
    assert_eq!(parsed["items"][0]["slug"], "brand-voice");
}

#[tokio::test]
async fn create_then_get_returns_doc() {
    let (store, tenant, _) = fixture_store().await;
    let body = r#"{"slug":"refund-policy","title":"Refund Policy","category":"business-rules","body_markdown":"30 days."}"#;
    let _ = hh::handle_create(&store, tenant, body, "principal-1").await;
    let (status, get_body) = hh::handle_get(&store, tenant, "refund-policy").await;
    assert!(status.starts_with("200"));
    let parsed: Value = serde_json::from_str(&get_body).expect("parse get");
    assert_eq!(parsed["slug"], "refund-policy");
    assert_eq!(parsed["title"], "Refund Policy");
    assert_eq!(parsed["category"], "business-rules");
    assert_eq!(parsed["version"], 1);
}

#[tokio::test]
async fn create_with_duplicate_slug_returns_409() {
    let (store, tenant, _) = fixture_store().await;
    let body = r#"{"slug":"dup","title":"A","category":"faq","body_markdown":"x"}"#;
    let _ = hh::handle_create(&store, tenant, body, "principal-1").await;
    let (status, body) = hh::handle_create(&store, tenant, body, "principal-1").await;
    assert!(
        status.starts_with("409"),
        "expected 409, got {status} body: {body}"
    );
}

#[tokio::test]
async fn update_increments_version_and_emits_audit_row() {
    let (store, tenant, _) = fixture_store().await;
    let body = r#"{"slug":"policy","title":"Policy","category":"compliance","body_markdown":"v1 body"}"#;
    let _ = hh::handle_create(&store, tenant, body, "principal-1").await;
    let update = r#"{"body_markdown":"v2 body — clarified scope","edit_reason":"clarity"}"#;
    let (status, resp_body) = hh::handle_update(&store, tenant, "policy", update, "principal-1").await;
    assert!(status.starts_with("200"), "status: {status}");
    let parsed: Value = serde_json::from_str(&resp_body).expect("parse update resp");
    assert_eq!(parsed["version"], 2);
    assert_eq!(parsed["body_markdown"], "v2 body — clarified scope");
    // Verify the audit row landed with the diff payload.
    let row = fetch_latest_audit_row(&store, tenant).await;
    assert_eq!(row.kind, AuditKind::WorkplaceKnowledgeDocUpdated.label());
    let payload: Value = serde_json::from_str(&row.payload_json).expect("payload json");
    assert_eq!(payload["slug"], "policy");
    assert_eq!(payload["version"], 2);
    let diff = payload["diff"].as_str().expect("diff field present");
    assert!(diff.contains("v1 body"), "diff must show prior: {diff}");
    assert!(diff.contains("v2 body"), "diff must show new: {diff}");
}

#[tokio::test]
async fn update_with_wrong_tenant_returns_404() {
    let (store, tenant_a, tenant_b) = fixture_store().await;
    let body = r#"{"slug":"only-in-a","title":"A","category":"brand","body_markdown":"x"}"#;
    let _ = hh::handle_create(&store, tenant_a, body, "principal-1").await;
    let update = r#"{"body_markdown":"hostile rewrite"}"#;
    let (status, _) = hh::handle_update(&store, tenant_b, "only-in-a", update, "tenant-b").await;
    assert!(
        status.starts_with("404"),
        "cross-tenant update must 404, got {status}"
    );
}

#[tokio::test]
async fn delete_emits_audit_row_and_subsequent_get_returns_404() {
    let (store, tenant, _) = fixture_store().await;
    let body = r#"{"slug":"to-delete","title":"X","category":"internal","body_markdown":"x"}"#;
    let _ = hh::handle_create(&store, tenant, body, "principal-1").await;
    let (status, _) = hh::handle_delete(&store, tenant, "to-delete", "principal-1").await;
    assert!(status.starts_with("204"));
    let (status, _) = hh::handle_get(&store, tenant, "to-delete").await;
    assert!(status.starts_with("404"));
    let row = fetch_latest_audit_row(&store, tenant).await;
    assert_eq!(row.kind, AuditKind::WorkplaceKnowledgeDocDeleted.label());
}

#[tokio::test]
async fn revisions_returns_history_after_three_edits() {
    let (store, tenant, _) = fixture_store().await;
    let body = r#"{"slug":"rev-test","title":"R","category":"faq","body_markdown":"v1"}"#;
    let _ = hh::handle_create(&store, tenant, body, "p").await;
    for new in ["v2", "v3", "v4"] {
        let update = format!(r#"{{"body_markdown":"{new}"}}"#);
        let _ = hh::handle_update(&store, tenant, "rev-test", &update, "p").await;
    }
    let (status, body) = hh::handle_revisions(&store, tenant, "rev-test").await;
    assert!(status.starts_with("200"), "status: {status}");
    let parsed: Value = serde_json::from_str(&body).expect("parse json");
    // Three updates produce three revision rows (one per prior body).
    assert_eq!(parsed["count"], 3);
    let items = parsed["items"].as_array().expect("items");
    // Newest first per substrate ORDER BY version DESC.
    assert_eq!(items[0]["version"], 3);
    assert_eq!(items[1]["version"], 2);
    assert_eq!(items[2]["version"], 1);
}

#[tokio::test]
async fn cross_tenant_get_returns_404_not_403() {
    let (store, tenant_a, tenant_b) = fixture_store().await;
    let body = r#"{"slug":"a-only","title":"A","category":"brand","body_markdown":"x"}"#;
    let _ = hh::handle_create(&store, tenant_a, body, "p-a").await;
    let (status, body) = hh::handle_get(&store, tenant_b, "a-only").await;
    assert!(
        status.starts_with("404"),
        "cross-tenant get must 404 (no existence leak), got {status}"
    );
    assert!(!body.contains("forbidden"));
}

#[tokio::test]
async fn category_filter_returns_only_matching_docs() {
    let (store, tenant, _) = fixture_store().await;
    let docs = [
        ("brand-1", "brand"),
        ("faq-1", "faq"),
        ("brand-2", "brand"),
        ("comp-1", "compliance"),
    ];
    for (slug, cat) in &docs {
        let body = format!(
            r#"{{"slug":"{slug}","title":"t","category":"{cat}","body_markdown":"b"}}"#
        );
        let _ = hh::handle_create(&store, tenant, &body, "p").await;
    }
    let (status, body) = hh::handle_list(&store, tenant, Some("brand")).await;
    assert!(status.starts_with("200"));
    let parsed: Value = serde_json::from_str(&body).expect("parse json");
    assert_eq!(parsed["count"], 2);
    let slugs: Vec<&str> = parsed["items"]
        .as_array()
        .expect("items")
        .iter()
        .map(|i| i["slug"].as_str().expect("slug"))
        .collect();
    assert!(slugs.contains(&"brand-1"));
    assert!(slugs.contains(&"brand-2"));
}

#[tokio::test]
async fn audit_row_payload_includes_diff_on_update() {
    // Pillar 1 acceptance: the WorkplaceKnowledgeDocUpdated row's
    // payload MUST carry a `diff` field per the design doc §7.
    let (store, tenant, _) = fixture_store().await;
    let body = r#"{"slug":"diffed","title":"D","category":"brand","body_markdown":"calm and clear"}"#;
    let _ = hh::handle_create(&store, tenant, body, "p").await;
    let update = r#"{"body_markdown":"calm, clear, kind"}"#;
    let _ = hh::handle_update(&store, tenant, "diffed", update, "p").await;
    let row = fetch_latest_audit_row(&store, tenant).await;
    assert_eq!(row.kind, AuditKind::WorkplaceKnowledgeDocUpdated.label());
    let payload: Value = serde_json::from_str(&row.payload_json).expect("payload");
    assert!(payload["diff"].is_string(), "diff field must be present");
    let diff = payload["diff"].as_str().expect("diff str");
    assert!(diff.contains("calm and clear"));
    assert!(diff.contains("calm, clear, kind"));
}

#[tokio::test]
async fn cross_tenant_slugs_are_independent() {
    // Substrate guarantee: (tenant_id, slug) is UNIQUE — two tenants
    // may both have a `brand-voice` doc. The gateway must surface
    // both without spurious 409.
    let (store, tenant_a, tenant_b) = fixture_store().await;
    let body = r#"{"slug":"brand-voice","title":"BV-A","category":"brand","body_markdown":"a"}"#;
    let (status, _) = hh::handle_create(&store, tenant_a, body, "p-a").await;
    assert!(status.starts_with("201"));
    let body_b = r#"{"slug":"brand-voice","title":"BV-B","category":"brand","body_markdown":"b"}"#;
    let (status, _) = hh::handle_create(&store, tenant_b, body_b, "p-b").await;
    assert!(
        status.starts_with("201"),
        "tenant B must be able to claim the same slug; got {status}"
    );
    // Verify both heads coexist.
    let a = doc::get(store.pool(), &tenant_a, "brand-voice")
        .await
        .expect("get a")
        .expect("doc a present");
    let b = doc::get(store.pool(), &tenant_b, "brand-voice")
        .await
        .expect("get b")
        .expect("doc b present");
    assert_eq!(a.title, "BV-A");
    assert_eq!(b.title, "BV-B");
}

#[tokio::test]
async fn create_with_unknown_category_returns_400() {
    let (store, tenant, _) = fixture_store().await;
    let body = r#"{"slug":"x","title":"X","category":"made-up","body_markdown":"x"}"#;
    let (status, body) = hh::handle_create(&store, tenant, body, "p").await;
    assert!(status.starts_with("400"), "status: {status}");
    assert!(body.contains("unknown category"));
}

#[tokio::test]
async fn update_with_missing_body_returns_400() {
    let (store, tenant, _) = fixture_store().await;
    let create_body = r#"{"slug":"u","title":"U","category":"brand","body_markdown":"x"}"#;
    let _ = hh::handle_create(&store, tenant, create_body, "p").await;
    // `edit_reason` only — no `body_markdown`.
    let update = r#"{"edit_reason":"meta only"}"#;
    let (status, _) = hh::handle_update(&store, tenant, "u", update, "p").await;
    assert!(status.starts_with("400"), "missing body must 400; got {status}");
}

// ── Audit-row probe helper ─────────────────────────────────────

struct AuditRowProbe {
    kind: &'static str,
    payload_json: String,
}

async fn fetch_latest_audit_row(store: &Arc<SessionStore>, tenant: TenantId) -> AuditRowProbe {
    // Flush any pending batched audit writes so this read sees them
    // synchronously. Build a sink with the matching tenant just to
    // pump the global writer queue.
    let sink = argo_audit::AuditSink::new_with_tenant(Arc::clone(store), tenant.to_string());
    sink.flush_pending().await.expect("flush audit");
    let tenant_str = tenant.to_string();
    let row = sqlx::query(
        "SELECT kind, payload_json FROM audit_record WHERE tenant_id = ? \
         ORDER BY seq DESC LIMIT 1",
    )
    .bind(&tenant_str)
    .fetch_one(store.pool())
    .await
    .expect("fetch audit row");
    let kind_str: String = row.try_get("kind").expect("kind col");
    let payload_json: String = row.try_get("payload_json").expect("payload col");
    let kind_static: &'static str = match kind_str.as_str() {
        "workplace_knowledge.doc.created" => AuditKind::WorkplaceKnowledgeDocCreated.label(),
        "workplace_knowledge.doc.updated" => AuditKind::WorkplaceKnowledgeDocUpdated.label(),
        "workplace_knowledge.doc.deleted" => AuditKind::WorkplaceKnowledgeDocDeleted.label(),
        other => panic!("unexpected audit kind: {other}"),
    };
    AuditRowProbe {
        kind: kind_static,
        payload_json,
    }
}
