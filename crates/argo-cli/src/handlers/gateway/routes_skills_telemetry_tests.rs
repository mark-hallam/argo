//! Tests for [`super`] — `GET /api/v1/skills/<id-or-name>/telemetry`,
//! the REAL learning-loop telemetry that retires the
//! `sample-skill-telemetry.ts` fixture.
//!
//! These pin that every field is sourced from a real table and that the
//! fixture's invented fields (`latency_ms`, per-worker `worker_name` on
//! a run) are NOT present — NO THEATER.

use super::*;
// The `/api/v1/skills/<id>/telemetry` route is reached through the
// sibling list-get dispatcher (where the `/telemetry` suffix is wired),
// so the integration tests drive it the same way the gateway does.
use crate::handlers::gateway::routes_skills_list_get::dispatch_get;
use argo_skill::SkillStore;
use argo_store::SessionStore;
use chrono::{Duration, Utc};
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

/// Install one skill under an explicit tenant UUID; return
/// `(tenant, skill_uuid)`.
async fn seed_skill(store: &Arc<SessionStore>, name: &str, source: &str) -> (Uuid, Uuid) {
    let tenant = Uuid::now_v7();
    let skills = SkillStore::new(Arc::clone(store));
    let skill = skills
        .install_for_tenant(&skill_md(name, source), &tenant.to_string(), None)
        .await
        .expect("install skill for tenant");
    (tenant, skill.id.0)
}

/// Insert one reeval-history row for `skill_id`.
async fn insert_reeval(
    store: &Arc<SessionStore>,
    skill_id: Uuid,
    version: &str,
    run_at_micros: i64,
    pass_rate: f64,
    previous: Option<f64>,
    regressed: i64,
) {
    ensure_reeval_schema(store).await.expect("schema");
    sqlx::query(
        "INSERT INTO skill_reeval_history \
         (eval_run_id, skill_id, skill_name, skill_version, run_at, \
          pass_rate, previous_pass_rate, regressed) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(Uuid::now_v7().to_string())
    .bind(skill_id.to_string())
    .bind("scrape.skill")
    .bind(version)
    .bind(run_at_micros)
    .bind(pass_rate)
    .bind(previous)
    .bind(regressed)
    .execute(store.pool())
    .await
    .expect("insert reeval");
}

/// Insert a `worker_skill` adoption row (real table) with an explicit
/// `created_at` so the adoption-trend bucketing is deterministic.
async fn insert_worker_skill(
    store: &Arc<SessionStore>,
    worker_id: &str,
    skill_id: Uuid,
    created_at: &str,
) {
    IdentityStore::new(store)
        .ensure_schema()
        .await
        .expect("worker_skill schema");
    sqlx::query(
        "INSERT INTO worker_skill (worker_id, skill_id, proficiency, created_at) \
         VALUES (?, ?, 3, ?)",
    )
    .bind(worker_id)
    .bind(skill_id.to_string())
    .bind(created_at)
    .execute(store.pool())
    .await
    .expect("insert worker_skill");
}

fn telemetry_req(segment: &str, tenant: Uuid) -> String {
    format!(
        "GET /api/v1/skills/{segment}/telemetry HTTP/1.1\r\n\
         X-Argo-Tenant-Id: {tenant}\r\nHost: x\r\n\r\n"
    )
}

#[tokio::test]
async fn telemetry_returns_real_run_count_and_history_by_id() {
    let store = session_store().await;
    let (tenant, skill_id) =
        seed_skill(&store, "pricing.competitor_scrape", "published:verified:argo-seed").await;

    // Three REAL reeval runs: a baseline, an improvement, then a regression.
    let now = Utc::now();
    insert_reeval(
        &store,
        skill_id,
        "v1",
        (now - Duration::days(12)).timestamp_micros(),
        0.86,
        None,
        0,
    )
    .await;
    insert_reeval(
        &store,
        skill_id,
        "v2",
        (now - Duration::days(6)).timestamp_micros(),
        0.91,
        Some(0.86),
        0,
    )
    .await;
    insert_reeval(
        &store,
        skill_id,
        "v3",
        (now - Duration::days(1)).timestamp_micros(),
        0.72,
        Some(0.91),
        1,
    )
    .await;

    // Two REAL adopters (cross-tenant aggregate).
    insert_worker_skill(&store, "wkr-a", skill_id, "2026-05-10 09:00:00").await;
    insert_worker_skill(&store, "wkr-b", skill_id, "2026-05-20 09:00:00").await;

    let req = telemetry_req(&skill_id.to_string(), tenant);
    let (status, body) = dispatch_get(&req, Some(&store))
        .await
        .expect("telemetry route must dispatch");
    assert_eq!(status, "200 OK", "by-id telemetry must resolve: {body}");
    let parsed: serde_json::Value = serde_json::from_str(&body).expect("json");

    // run_count is the REAL row count, not a fixture constant.
    assert_eq!(parsed["run_count"], 3, "run_count = real reeval rows");
    assert_eq!(parsed["distinct_workers"], 2, "real adoption count");
    assert_eq!(parsed["skill_name"], "pricing.competitor_scrape");
    assert_eq!(parsed["current_grade"], "published_verified");

    // eval_history is oldest→newest with the real decisions.
    let history = parsed["eval_history"].as_array().expect("eval_history array");
    assert_eq!(history.len(), 3);
    assert_eq!(history[0]["decision"], "no_baseline");
    assert_eq!(history[1]["decision"], "stable");
    assert_eq!(history[2]["decision"], "regressed");
    assert_eq!(history[0]["version"], "v1");
    // The latest row's pass_rate is the real 0.72.
    assert!((history[2]["pass_rate"].as_f64().unwrap() - 0.72).abs() < 1e-9);

    // recent_runs is newest-first, capped at 3.
    let recent = parsed["recent_runs"].as_array().expect("recent_runs array");
    assert_eq!(recent.len(), 3);
    assert_eq!(recent[0]["version"], "v3", "recent_runs newest first");
    assert_eq!(recent[0]["decision"], "regressed");

    // adoption_trend is a real 30-point cumulative series ending at 2.
    let trend = parsed["adoption_trend"].as_array().expect("adoption_trend");
    assert_eq!(trend.len(), 30, "30-day adoption series");
    assert_eq!(
        trend.last().unwrap().as_i64().unwrap(),
        2,
        "cumulative adoption ends at the 2 real adopters"
    );
    assert!(
        trend.first().unwrap().as_i64().unwrap() <= trend.last().unwrap().as_i64().unwrap(),
        "cumulative series is monotone non-decreasing"
    );

    // NO THEATER: the fixture's invented per-run fields must be ABSENT.
    let body_lc = body.to_lowercase();
    assert!(
        !body_lc.contains("latency"),
        "no fabricated latency in telemetry: {body}"
    );
    assert!(
        !recent[0].as_object().unwrap().contains_key("worker_name"),
        "recent runs carry no fabricated worker_name"
    );
    assert!(
        !recent[0].as_object().unwrap().contains_key("worker_id"),
        "recent runs carry no fabricated worker_id"
    );
}

#[tokio::test]
async fn telemetry_resolves_by_name() {
    let store = session_store().await;
    let (tenant, skill_id) = seed_skill(&store, "approval.route", "published:verified:team").await;
    insert_reeval(
        &store,
        skill_id,
        "v1",
        Utc::now().timestamp_micros(),
        0.9,
        None,
        0,
    )
    .await;

    // The SPA may pass the NAME instead of the catalog UUID.
    let req = telemetry_req("approval.route", tenant);
    let (status, body) = dispatch_get(&req, Some(&store))
        .await
        .expect("dispatch by name");
    assert_eq!(status, "200 OK", "name-keyed telemetry must resolve: {body}");
    let parsed: serde_json::Value = serde_json::from_str(&body).expect("json");
    assert_eq!(parsed["skill_name"], "approval.route");
    // skill_id is a real UUID even though we queried by name.
    let id = parsed["skill_id"].as_str().expect("skill_id");
    assert!(Uuid::parse_str(id).is_ok());
    assert_eq!(parsed["run_count"], 1);
}

#[tokio::test]
async fn telemetry_honest_empty_state_when_no_history() {
    let store = session_store().await;
    // Skill exists but has NO reeval rows and NO adopters.
    let (tenant, skill_id) = seed_skill(&store, "lead.enrich", "learned").await;

    let req = telemetry_req(&skill_id.to_string(), tenant);
    let (status, body) = dispatch_get(&req, Some(&store))
        .await
        .expect("dispatch empty");
    assert_eq!(status, "200 OK", "empty telemetry still 200: {body}");
    let parsed: serde_json::Value = serde_json::from_str(&body).expect("json");

    // Honest empty: real zero counts + empty arrays, never a fixture.
    assert_eq!(parsed["run_count"], 0);
    assert_eq!(parsed["distinct_workers"], 0);
    assert_eq!(
        parsed["eval_history"].as_array().unwrap().len(),
        0,
        "no fabricated history"
    );
    assert_eq!(
        parsed["recent_runs"].as_array().unwrap().len(),
        0,
        "no fabricated runs"
    );
    // adoption_trend is all-zero (real: nobody learned it yet).
    let trend = parsed["adoption_trend"].as_array().expect("trend");
    assert!(
        trend.iter().all(|v| v.as_i64() == Some(0)),
        "empty adoption is an all-zero series, not fabricated"
    );
}

#[tokio::test]
async fn telemetry_404s_on_unknown_skill() {
    let store = session_store().await;
    let (tenant, _id) = seed_skill(&store, "support.triage", "learned").await;
    let req = telemetry_req("no.such.skill", tenant);
    let (status, body) = dispatch_get(&req, Some(&store)).await.expect("dispatch");
    assert_eq!(status, "404 Not Found", "unknown skill must 404: {body}");
}

#[tokio::test]
async fn telemetry_is_tenant_scoped() {
    // Tenant B cannot read tenant A's skill telemetry by id.
    let store = session_store().await;
    let (_tenant_a, skill_id) = seed_skill(&store, "deal.qualify", "learned").await;
    let tenant_b = Uuid::now_v7();
    let req = telemetry_req(&skill_id.to_string(), tenant_b);
    let (status, body) = dispatch_get(&req, Some(&store)).await.expect("dispatch");
    assert_eq!(
        status, "404 Not Found",
        "cross-tenant skill id must 404 for tenant B: {body}"
    );
}
