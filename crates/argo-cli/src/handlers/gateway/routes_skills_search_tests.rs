//! Tests for [`super`] — the `/api/v1/skills/search` server-side ranked
//! search over the caller's REAL tenant skill rows.
//!
//! Pins: (1) ranking puts name-hits ahead of description-only hits,
//! (2) the route only returns the caller's tenant rows (no cross-tenant
//! leak), (3) the `category`/`source`/`trust_min` facet filters apply,
//! (4) the `facets` block is computed from the real catalog (honest
//! counts, no fabricated taxonomy), (5) an empty `q` ranks-returns all.

use super::*;
use argo_store::SessionStore;
use serde_json::Value;
use std::sync::Arc;
use uuid::Uuid;

async fn session_store() -> Arc<SessionStore> {
    let store = SessionStore::open(":memory:").await.expect("store");
    store.migrate().await.expect("migrate");
    Arc::new(store)
}

/// Insert one active skill row directly (full control over tenant /
/// name / source / frontmatter). Mirrors `argo_skill::version` tests.
async fn insert_skill(
    store: &Arc<SessionStore>,
    tenant: &str,
    name: &str,
    description: &str,
    source: &str,
    created_at: i64,
) {
    let id = Uuid::now_v7().as_bytes().to_vec();
    let frontmatter = serde_json::json!({
        "name": name,
        "version": "1.0.0",
        "description": description,
        "risk": "medium",
        "tags": ["demo"],
    })
    .to_string();
    sqlx::query(
        "INSERT INTO skill (id, name, version, body, frontmatter, source, hash, \
         learned_in, superseded_by, created_at, lifecycle_state, tenant_id) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 'active', ?)",
    )
    .bind(&id)
    .bind(name)
    .bind("1.0.0")
    .bind("body")
    .bind(&frontmatter)
    .bind(source)
    .bind(vec![0u8; 32])
    .bind(Option::<&[u8]>::None)
    .bind(Option::<&[u8]>::None)
    .bind(created_at)
    .bind(tenant)
    .execute(store.pool())
    .await
    .expect("insert skill");
}

fn search_req(qs: &str, tenant: &str) -> String {
    format!(
        "GET /api/v1/skills/search?{qs} HTTP/1.1\r\n\
         X-Argo-Tenant-Id: {tenant}\r\nHost: x\r\n\r\n"
    )
}

#[tokio::test]
async fn ranks_name_hit_ahead_of_description_only_hit() {
    let store = session_store().await;
    let tenant = Uuid::now_v7().to_string();
    // "approval.route" — query term is in the NAME (weight 8).
    // source "user" maps to TrustGrade::Tenant (see `from_source`).
    insert_skill(
        &store,
        &tenant,
        "approval.route",
        "decides where a request goes",
        "user",
        2_000,
    )
    .await;
    // "deal.qualify" — query term only in the DESCRIPTION (weight 3).
    insert_skill(
        &store,
        &tenant,
        "deal.qualify",
        "needs approval before closing",
        "user",
        1_000,
    )
    .await;

    let req = search_req("q=approval", &tenant);
    let (status, body) = dispatch_get(&req, Some(&store)).await.expect("dispatch");
    assert_eq!(status, "200 OK", "{body}");
    let parsed: Value = serde_json::from_str(&body).expect("json");
    let results = parsed["results"].as_array().expect("results array");
    assert_eq!(results.len(), 2, "both rows match the term");
    assert_eq!(
        results[0]["name"], "approval.route",
        "name hit must outrank description-only hit"
    );
    // The card carries the real category (cohort prefix) + trust grade.
    assert_eq!(results[0]["category"], "approval");
    assert_eq!(results[0]["trust_grade"], "tenant");
}

#[tokio::test]
async fn only_returns_callers_tenant_rows() {
    let store = session_store().await;
    let mine = Uuid::now_v7().to_string();
    let other = Uuid::now_v7().to_string();
    insert_skill(&store, &mine, "approval.route", "mine", "user", 1_000).await;
    insert_skill(&store, &other, "approval.escalate", "theirs", "user", 1_000).await;

    let req = search_req("q=approval", &mine);
    let (status, body) = dispatch_get(&req, Some(&store)).await.expect("dispatch");
    assert_eq!(status, "200 OK", "{body}");
    let parsed: Value = serde_json::from_str(&body).expect("json");
    let results = parsed["results"].as_array().expect("results");
    assert_eq!(results.len(), 1, "only my tenant's row is searched");
    assert_eq!(results[0]["name"], "approval.route");
    assert!(
        !body.contains("approval.escalate"),
        "other tenant's skill must not leak: {body}"
    );
}

#[tokio::test]
async fn category_and_source_facet_filters_apply() {
    let store = session_store().await;
    let tenant = Uuid::now_v7().to_string();
    insert_skill(&store, &tenant, "approval.route", "a", "user", 3_000).await;
    insert_skill(&store, &tenant, "invoice.extract", "b", "published:verified:team", 2_000).await;
    insert_skill(&store, &tenant, "invoice.match", "c", "learned", 1_000).await;

    // category=invoice keeps only the two invoice.* rows.
    let req = search_req("category=invoice", &tenant);
    let (_s, body) = dispatch_get(&req, Some(&store)).await.expect("dispatch");
    let parsed: Value = serde_json::from_str(&body).expect("json");
    let names: Vec<&str> = parsed["results"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|r| r["name"].as_str())
        .collect();
    assert_eq!(names.len(), 2, "category filter applied: {body}");
    assert!(names.iter().all(|n| n.starts_with("invoice")));

    // source substring filter narrows to the published row.
    let req = search_req("source=published", &tenant);
    let (_s, body) = dispatch_get(&req, Some(&store)).await.expect("dispatch");
    let parsed: Value = serde_json::from_str(&body).expect("json");
    let results = parsed["results"].as_array().unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0]["name"], "invoice.extract");
}

#[tokio::test]
async fn trust_min_filters_less_trusted_rows() {
    let store = session_store().await;
    let tenant = Uuid::now_v7().to_string();
    // verified (more trusted) + learned (least trusted).
    insert_skill(&store, &tenant, "verified.skill", "x", "published:verified:team", 2_000).await;
    insert_skill(&store, &tenant, "learned.skill", "x", "learned", 1_000).await;

    let req = search_req("trust_min=published-verified", &tenant);
    let (status, body) = dispatch_get(&req, Some(&store)).await.expect("dispatch");
    assert_eq!(status, "200 OK", "{body}");
    let parsed: Value = serde_json::from_str(&body).expect("json");
    let results = parsed["results"].as_array().unwrap();
    assert_eq!(results.len(), 1, "only verified passes trust_min");
    assert_eq!(results[0]["name"], "verified.skill");
}

#[tokio::test]
async fn facets_are_computed_from_real_catalog() {
    let store = session_store().await;
    let tenant = Uuid::now_v7().to_string();
    insert_skill(&store, &tenant, "approval.route", "a", "user", 3_000).await;
    insert_skill(&store, &tenant, "approval.escalate", "b", "user", 2_000).await;
    insert_skill(&store, &tenant, "invoice.extract", "c", "published:verified:team", 1_000).await;

    let req = search_req("", &tenant);
    let (status, body) = dispatch_get(&req, Some(&store)).await.expect("dispatch");
    assert_eq!(status, "200 OK", "{body}");
    let parsed: Value = serde_json::from_str(&body).expect("json");

    // Empty query ranks-returns every tenant row.
    assert_eq!(parsed["results"].as_array().unwrap().len(), 3);

    let facets = &parsed["facets"];
    // Categories: approval (count 2) + invoice (count 1), sorted.
    let cats = facets["category"].as_array().expect("category facets");
    let approval = cats
        .iter()
        .find(|f| f["value"] == "approval")
        .expect("approval category present");
    assert_eq!(approval["count"], 2, "honest occurrence count");
    assert!(cats.iter().any(|f| f["value"] == "invoice"));
    // Trust facets carry the canonical snake_case discriminants.
    let trust = facets["trust"].as_array().expect("trust facets");
    assert!(trust.iter().any(|f| f["value"] == "tenant"));
    assert!(trust.iter().any(|f| f["value"] == "published_verified"));
    // Source facets are the raw provenance strings (e.g. "user").
    let sources = facets["source"].as_array().expect("source facets");
    assert!(sources.iter().any(|f| f["value"] == "user"));
}

#[tokio::test]
async fn unknown_trust_min_label_is_400() {
    let store = session_store().await;
    let tenant = Uuid::now_v7().to_string();
    let req = search_req("trust_min=made-up", &tenant);
    let (status, body) = dispatch_get(&req, Some(&store)).await.expect("dispatch");
    assert_eq!(status, "400 Bad Request", "{body}");
}

#[tokio::test]
async fn search_path_is_not_swallowed_as_skill_id() {
    // Regression: the literal `search` segment must dispatch to this
    // route, NOT the `/api/v1/skills/<id>` detail catch-all.
    let store = session_store().await;
    let tenant = Uuid::now_v7().to_string();
    let req = search_req("q=anything", &tenant);
    let (status, body) = dispatch_get(&req, Some(&store)).await.expect("dispatch");
    assert_eq!(status, "200 OK", "{body}");
    let parsed: Value = serde_json::from_str(&body).expect("json");
    assert!(parsed.get("facets").is_some(), "search envelope, not a 404 detail");
}
