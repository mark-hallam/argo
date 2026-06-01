//! Tests for [`super`] — the `/api/v1/skills/<id-or-name>/versions`
//! version-history + eval-delta route.
//!
//! Pins: (1) the version timeline is newest-first with the real
//! `superseded` + `lifecycle_state` flags, (2) the eval-delta block is
//! the latest `skill_reeval_history` run for the version with a real
//! percentage-point delta, (3) a version with no re-eval rows reports
//! `eval: null` (honest empty — no fabricated promotion arc), (4) the
//! route is tenant-scoped (a name the caller's tenant doesn't own 404s),
//! (5) name-or-id resolution both work.

// Drive the route through the real parent dispatcher so the
// `/versions` suffix routing in `routes_skills_list_get::dispatch_get`
// is exercised end-to-end (same pattern as the `/usage` tests).
use super::super::routes_skills_list_get::dispatch_get;
use argo_store::SessionStore;
use serde_json::Value;
use std::sync::Arc;
use uuid::Uuid;

async fn session_store() -> Arc<SessionStore> {
    let store = SessionStore::open(":memory:").await.expect("store");
    store.migrate().await.expect("migrate");
    Arc::new(store)
}

/// Insert one skill row; returns its 16-byte id blob.
async fn insert_skill(
    store: &Arc<SessionStore>,
    tenant: &str,
    name: &str,
    version: &str,
    source: &str,
    created_at: i64,
    superseded_by: Option<&[u8]>,
) -> Vec<u8> {
    let id = Uuid::now_v7().as_bytes().to_vec();
    sqlx::query(
        "INSERT INTO skill (id, name, version, body, frontmatter, source, hash, \
         learned_in, superseded_by, created_at, lifecycle_state, tenant_id) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 'active', ?)",
    )
    .bind(&id)
    .bind(name)
    .bind(version)
    .bind("body")
    .bind("{}")
    .bind(source)
    .bind(vec![0u8; 32])
    .bind(Option::<&[u8]>::None)
    .bind(superseded_by)
    .bind(created_at)
    .bind(tenant)
    .execute(store.pool())
    .await
    .expect("insert skill");
    id
}

/// Seed one `skill_reeval_history` row (lazy table — created on demand).
async fn insert_reeval(
    store: &Arc<SessionStore>,
    skill_name: &str,
    version: &str,
    run_at: i64,
    pass_rate: f64,
    previous: Option<f64>,
    regressed: i64,
) {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS skill_reeval_history ( \
         eval_run_id TEXT PRIMARY KEY, skill_id TEXT NOT NULL, skill_name TEXT NOT NULL, \
         skill_version TEXT NOT NULL, run_at INTEGER NOT NULL, pass_rate REAL, \
         previous_pass_rate REAL, regressed INTEGER NOT NULL DEFAULT 0 )",
    )
    .execute(store.pool())
    .await
    .expect("ensure reeval table");
    sqlx::query(
        "INSERT INTO skill_reeval_history \
         (eval_run_id, skill_id, skill_name, skill_version, run_at, pass_rate, previous_pass_rate, regressed) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(Uuid::now_v7().to_string())
    .bind("skill-id")
    .bind(skill_name)
    .bind(version)
    .bind(run_at)
    .bind(pass_rate)
    .bind(previous)
    .bind(regressed)
    .execute(store.pool())
    .await
    .expect("insert reeval");
}

fn versions_req(segment: &str, tenant: &str) -> String {
    format!(
        "GET /api/v1/skills/{segment}/versions HTTP/1.1\r\n\
         X-Argo-Tenant-Id: {tenant}\r\nHost: x\r\n\r\n"
    )
}

#[tokio::test]
async fn timeline_is_newest_first_with_superseded_flag() {
    let store = session_store().await;
    let tenant = Uuid::now_v7().to_string();
    // v1 older + superseded by v2; v2 newest + active.
    let v1 = insert_skill(&store, &tenant, "deal.qualify", "1.0.0", "tenant", 1_000, None).await;
    insert_skill(&store, &tenant, "deal.qualify", "2.0.0", "tenant", 2_000, None).await;
    // Link v1 → v2 (mark v1 superseded).
    sqlx::query("UPDATE skill SET superseded_by = (SELECT id FROM skill WHERE version='2.0.0') WHERE id = ?")
        .bind(&v1)
        .execute(store.pool())
        .await
        .expect("link superseded");

    let req = versions_req("deal.qualify", &tenant);
    let (status, body) = dispatch_get(&req, Some(&store)).await.expect("dispatch");
    assert_eq!(status, "200 OK", "{body}");
    let parsed: Value = serde_json::from_str(&body).expect("json");
    assert_eq!(parsed["skill_name"], "deal.qualify");
    assert_eq!(parsed["version_count"], 2);
    let versions = parsed["versions"].as_array().expect("versions array");
    assert_eq!(versions[0]["version"], "2.0.0", "newest first");
    assert_eq!(versions[1]["version"], "1.0.0");
    assert_eq!(versions[1]["superseded"], true, "v1 is superseded");
    assert_eq!(versions[0]["superseded"], false, "v2 is active");
    assert_eq!(versions[0]["lifecycle_state"], "active");
}

#[tokio::test]
async fn eval_delta_surfaces_real_pass_rate_movement() {
    let store = session_store().await;
    let tenant = Uuid::now_v7().to_string();
    insert_skill(&store, &tenant, "support.triage", "2.0.0", "tenant", 2_000, None).await;
    // Two re-eval runs for v2: baseline 0.80, then 0.92 (+12.0pp, stable).
    insert_reeval(&store, "support.triage", "2.0.0", 1_000, 0.80, None, 0).await;
    insert_reeval(&store, "support.triage", "2.0.0", 2_000, 0.92, Some(0.80), 0).await;

    let req = versions_req("support.triage", &tenant);
    let (status, body) = dispatch_get(&req, Some(&store)).await.expect("dispatch");
    assert_eq!(status, "200 OK", "{body}");
    let parsed: Value = serde_json::from_str(&body).expect("json");
    let v = &parsed["versions"][0];
    let eval = &v["eval"];
    assert!(!eval.is_null(), "eval block present when runs exist");
    // Latest run's pass rate + delta in percentage points.
    assert_eq!(eval["pass_rate"], 0.92);
    assert_eq!(eval["previous_pass_rate"], 0.80);
    assert_eq!(eval["delta_pp"], 12.0, "(0.92-0.80)*100 = 12.0pp");
    assert_eq!(eval["run_count"], 2);
    assert_eq!(eval["decision"], "stable");
}

#[tokio::test]
async fn version_without_reeval_reports_null_eval() {
    let store = session_store().await;
    let tenant = Uuid::now_v7().to_string();
    insert_skill(&store, &tenant, "lead.score", "1.0.0", "learned", 1_000, None).await;
    // No skill_reeval_history rows seeded.

    let req = versions_req("lead.score", &tenant);
    let (status, body) = dispatch_get(&req, Some(&store)).await.expect("dispatch");
    assert_eq!(status, "200 OK", "{body}");
    let parsed: Value = serde_json::from_str(&body).expect("json");
    assert_eq!(parsed["version_count"], 1);
    assert!(
        parsed["versions"][0]["eval"].is_null(),
        "honest empty — no fabricated eval: {body}"
    );
}

#[tokio::test]
async fn resolves_by_uuid_too() {
    let store = session_store().await;
    let tenant = Uuid::now_v7().to_string();
    let id = insert_skill(&store, &tenant, "invoice.extract", "1.0.0", "tenant", 1_000, None).await;
    let uuid = Uuid::from_slice(&id).expect("uuid").to_string();

    let req = versions_req(&uuid, &tenant);
    let (status, body) = dispatch_get(&req, Some(&store)).await.expect("dispatch");
    assert_eq!(status, "200 OK", "uuid resolves: {body}");
    let parsed: Value = serde_json::from_str(&body).expect("json");
    assert_eq!(parsed["skill_name"], "invoice.extract");
}

#[tokio::test]
async fn other_tenants_skill_404s() {
    let store = session_store().await;
    let mine = Uuid::now_v7().to_string();
    let other = Uuid::now_v7().to_string();
    insert_skill(&store, &other, "secret.skill", "1.0.0", "tenant", 1_000, None).await;

    let req = versions_req("secret.skill", &mine);
    let (status, body) = dispatch_get(&req, Some(&store)).await.expect("dispatch");
    assert_eq!(status, "404 Not Found", "cross-tenant name must 404: {body}");
}

#[tokio::test]
async fn timeline_excludes_other_tenants_rows_for_the_same_name() {
    // Regression for the demo "6× v0.1.0" symptom: the same skill name
    // seeded under several tenants must NOT pool into one timeline. My
    // tenant's version history shows ONLY my row, even though three other
    // tenants carry the identical name+version.
    let store = session_store().await;
    let mine = Uuid::now_v7().to_string();
    insert_skill(&store, &mine, "pricing.competitor_scrape", "v3", "published:verified:argo-seed", 9_000, None).await;
    for _ in 0..5 {
        let other = Uuid::now_v7().to_string();
        insert_skill(&store, &other, "pricing.competitor_scrape", "0.1.0", "published:verified:argo-seed", 1_000, None).await;
    }

    let req = versions_req("pricing.competitor_scrape", &mine);
    let (status, body) = dispatch_get(&req, Some(&store)).await.expect("dispatch");
    assert_eq!(status, "200 OK", "{body}");
    let parsed: Value = serde_json::from_str(&body).expect("json");
    assert_eq!(
        parsed["version_count"], 1,
        "only my tenant's single row shows, not the 5 other tenants' rows: {body}"
    );
    assert_eq!(parsed["versions"][0]["version"], "v3");
}
