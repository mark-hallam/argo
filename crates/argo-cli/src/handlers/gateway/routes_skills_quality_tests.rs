//! Unit tests for `super::routes_skills_quality`.
//!
//! Drives `skill_quality_response` against an in-memory store
//! (mirrors the routes_skills.rs test pattern).
use super::*;
use argo_skill::{QualityInputs, SkillQualityStore, SkillStore};
use argo_store::SessionStore;
use std::sync::Arc;

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

async fn install_skill(
    skills: &SkillStore,
    name: &str,
    tenant: &argo_types::TenantId,
) -> argo_types::SkillId {
    let row = skills
        .install_for_tenant(&skill_md(name, "learned"), &tenant.to_string(), None)
        .await
        .expect("install");
    row.id
}

/// A minimal HTTP request line carrying an explicit `?tenant=` override
/// (no session cookie/bearer present), so `resolve_tenant_for_request`
/// resolves deterministically to `tenant` rather than the cwd fallback.
fn req_for_tenant(tenant: &argo_types::TenantId) -> String {
    format!("GET /skills/x/quality?tenant={tenant} HTTP/1.1\r\n\r\n")
}

#[tokio::test]
async fn unknown_skill_returns_404() {
    let store = session_store().await;
    let tenant = argo_types::TenantId::new();
    let (status, body) =
        skill_quality_response("does-not-exist", &req_for_tenant(&tenant), Some(&store), None).await;
    assert_eq!(status, "404 Not Found");
    assert!(body.contains("no active skill named 'does-not-exist'"));
}

#[tokio::test]
async fn no_quality_history_returns_200_with_null_latest() {
    let store = session_store().await;
    let skills = SkillStore::new(Arc::clone(&store));
    let tenant = argo_types::TenantId::new();
    install_skill(&skills, "alpha", &tenant).await;
    let (status, body) =
        skill_quality_response("alpha", &req_for_tenant(&tenant), Some(&store), None).await;
    assert_eq!(status, "200 OK");
    let v: serde_json::Value = serde_json::from_str(&body).expect("json");
    assert_eq!(v["skill_name"], "alpha");
    assert!(
        v.get("skill_id").and_then(|x| x.as_str()).is_some(),
        "expected skill_id present, got: {body}"
    );
    assert!(v["latest"].is_null(), "latest must be null, got: {body}");
    assert_eq!(v["history"].as_array().expect("history array").len(), 0);
    assert_eq!(v["history_len"], 0);
}

#[tokio::test]
async fn latest_score_is_returned_for_skill_with_history() {
    let store = session_store().await;
    let skills = SkillStore::new(Arc::clone(&store));
    let tenant = argo_types::TenantId::new();
    let id = install_skill(&skills, "beta", &tenant).await;
    let qstore = SkillQualityStore::new(Arc::clone(&store));
    qstore.ensure_schema().await.expect("schema");
    let inputs = QualityInputs {
        success_rate: 0.95,
        avg_latency_ms: 12.5,
        retry_count: 0,
        p99_latency_ms: 30.0,
    };
    qstore
        .record(id, &inputs, chrono::Utc::now())
        .await
        .expect("record");
    let (status, body) =
        skill_quality_response("beta", &req_for_tenant(&tenant), Some(&store), None).await;
    assert_eq!(status, "200 OK");
    let v: serde_json::Value = serde_json::from_str(&body).expect("json");
    assert_eq!(v["skill_name"], "beta");
    assert!(
        v["latest"].is_object(),
        "latest must be present, got: {body}"
    );
    assert!(
        v["latest"]["score"].is_number(),
        "latest.score must be number"
    );
    assert_eq!(v["history_len"], 0);
}

#[tokio::test]
async fn history_limit_returns_prior_scores_newest_first() {
    let store = session_store().await;
    let skills = SkillStore::new(Arc::clone(&store));
    let tenant = argo_types::TenantId::new();
    let id = install_skill(&skills, "gamma", &tenant).await;
    let qstore = SkillQualityStore::new(Arc::clone(&store));
    qstore.ensure_schema().await.expect("schema");
    for success in [0.80, 0.85, 0.90, 0.95] {
        let inputs = QualityInputs {
            success_rate: success,
            avg_latency_ms: 10.0,
            retry_count: 0,
            p99_latency_ms: 20.0,
        };
        qstore
            .record(id, &inputs, chrono::Utc::now())
            .await
            .expect("record");
        // tiny gap so timestamps are distinct
        tokio::time::sleep(std::time::Duration::from_millis(2)).await;
    }
    let (status, body) =
        skill_quality_response("gamma", &req_for_tenant(&tenant), Some(&store), Some(3)).await;
    assert_eq!(status, "200 OK");
    let v: serde_json::Value = serde_json::from_str(&body).expect("json");
    assert!(v["latest"].is_object(), "latest must be present");
    let history = v["history"].as_array().expect("history array");
    assert_eq!(
        history.len(),
        2,
        "limit=3 yields latest + 2 history rows; got {body}"
    );
    // Newest-first ordering: latest = success_rate 0.95;
    // history[0] = 0.90, history[1] = 0.85; the oldest (0.80) is
    // out of the window.
    assert!(
        (history[0]["inputs"]["success_rate"].as_f64().unwrap() - 0.90).abs() < 1e-9,
        "history[0] should be 0.90, got: {body}"
    );
    assert!(
        (history[1]["inputs"]["success_rate"].as_f64().unwrap() - 0.85).abs() < 1e-9,
        "history[1] should be 0.85, got: {body}"
    );
}

#[tokio::test]
async fn store_unavailable_returns_503() {
    let tenant = argo_types::TenantId::new();
    let (status, body) =
        skill_quality_response("alpha", &req_for_tenant(&tenant), None, None).await;
    assert!(
        status.starts_with("503"),
        "no-store path must return 503, got: {status} / {body}"
    );
}
