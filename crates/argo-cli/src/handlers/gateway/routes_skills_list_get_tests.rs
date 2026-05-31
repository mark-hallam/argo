//! Tests for [`super`] — specifically the `/api/v1/skills/<id-or-name>/usage`
//! drill-in route's name→id tolerance (Frontier-B backend-route batch).
//!
//! Before this batch the usage route only accepted the catalog UUID and
//! returned `404 "invalid skill id"` when the SPA passed the skill NAME.
//! The route now resolves either form; these tests pin both paths plus
//! the genuine-miss 404.

use super::*;
use argo_skill::SkillStore;
use argo_store::SessionStore;
use std::sync::Arc;
use uuid::Uuid;

async fn session_store() -> Arc<SessionStore> {
    let store = SessionStore::open(":memory:").await.expect("store");
    store.migrate().await.expect("migrate");
    Arc::new(store)
}

fn skill_md(name: &str, source: &str) -> String {
    format!(
        "---\n\
         name: {name}\n\
         version: \"0.1.0\"\n\
         description: {name} description\n\
         provenance:\n  \
         source: {source}\n  \
         hash: sha256-deadbeef\n\
         ---\n\
         body of {name}\n"
    )
}

/// Install one skill under an explicit tenant UUID; return that tenant.
async fn seed_skill(store: &Arc<SessionStore>, name: &str, source: &str) -> Uuid {
    let tenant = Uuid::now_v7();
    let skills = SkillStore::new(Arc::clone(store));
    skills
        .install_for_tenant(&skill_md(name, source), &tenant.to_string(), None)
        .await
        .expect("install skill for tenant");
    tenant
}

fn usage_req(segment: &str, tenant: Uuid) -> String {
    format!(
        "GET /api/v1/skills/{segment}/usage HTTP/1.1\r\n\
         X-Argo-Tenant-Id: {tenant}\r\nHost: x\r\n\r\n"
    )
}

#[tokio::test]
async fn usage_resolves_by_name_not_just_uuid() {
    let store = session_store().await;
    let tenant = seed_skill(&store, "approval.route", "published:verified:team").await;

    // The SPA passes the NAME — the pre-fix route 404'd here.
    let req = usage_req("approval.route", tenant);
    let (status, body) = dispatch_get(&req, Some(&store))
        .await
        .expect("usage route must dispatch");
    assert_eq!(status, "200 OK", "name-keyed usage must resolve: {body}");
    let parsed: serde_json::Value = serde_json::from_str(&body).expect("json");
    assert_eq!(parsed["skill_name"], "approval.route");
    // skill_id must be a real UUID even though we queried by name.
    let skill_id = parsed["skill_id"].as_str().expect("skill_id present");
    assert!(Uuid::parse_str(skill_id).is_ok(), "skill_id is a uuid");
    assert_eq!(parsed["trust_grade"], "published_verified");
    assert!(parsed["workers_using"].is_array(), "workers_using array");
    assert_eq!(parsed["workers_using_count"], 0, "no workers seeded");
    // No secret/key field leaks into the usage envelope.
    assert!(body.to_lowercase().find("secret").is_none(), "no secret in body");
}

#[tokio::test]
async fn usage_still_resolves_by_uuid() {
    let store = session_store().await;
    let tenant = seed_skill(&store, "deal.qualify", "learned").await;

    // First resolve the catalog list to grab the real UUID.
    let list_req = format!(
        "GET /api/v1/skills HTTP/1.1\r\nX-Argo-Tenant-Id: {tenant}\r\nHost: x\r\n\r\n"
    );
    let (_s, list_body) = dispatch_get(&list_req, Some(&store)).await.expect("list");
    let list: serde_json::Value = serde_json::from_str(&list_body).expect("list json");
    let id = list["skills"][0]["id"].as_str().expect("skill id").to_string();

    let req = usage_req(&id, tenant);
    let (status, body) = dispatch_get(&req, Some(&store)).await.expect("usage by id");
    assert_eq!(status, "200 OK", "uuid-keyed usage must resolve: {body}");
    let parsed: serde_json::Value = serde_json::from_str(&body).expect("json");
    assert_eq!(parsed["skill_id"], id);
    assert_eq!(parsed["skill_name"], "deal.qualify");
}

#[tokio::test]
async fn usage_404s_on_genuinely_unknown_name() {
    let store = session_store().await;
    let tenant = seed_skill(&store, "support.triage", "learned").await;
    let req = usage_req("no.such.skill", tenant);
    let (status, body) = dispatch_get(&req, Some(&store)).await.expect("dispatch");
    assert_eq!(status, "404 Not Found", "unknown skill must 404: {body}");
}
