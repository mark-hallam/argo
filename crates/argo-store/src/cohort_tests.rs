//! Tests for `cohort` — read-only fleet-wide cohort aggregation.
//!
//! Backend tests for Stream `mission-skill-cohort-ui`. Six tests cover:
//! cohort identity collapse (vertical+band), member_count + skill_count
//! roll-up, detail member ordering, skill-coverage pct math, missing
//! cohort returns `None`, and version resolution via the `skill` table.

use super::{cohort_detail, list_cohorts, MAX_COHORTS};
use crate::SessionStore;
use argo_types::TenantId;

async fn store_with_tenant() -> (std::sync::Arc<SessionStore>, TenantId) {
    let store = SessionStore::open(":memory:").await.expect("open mem db");
    store.migrate().await.expect("migrate");
    let store = std::sync::Arc::new(store);
    let (tenant, _ws) = store
        .ensure_workspace("cohort-tests", "/tmp/cohort-tests")
        .await
        .expect("ensure workspace");
    (store, tenant)
}

async fn seed_worker(
    store: &SessionStore,
    tenant: TenantId,
    worker_id: &str,
    vertical: &str,
    trust_grade: i64,
) {
    let persona_id = format!("persona-{worker_id}");
    sqlx::query(
        "INSERT INTO persona
            (id, worker_id, tenant_id, name, voice_preamble, avatar, bio,
             builder_attribution, provenance_chain_json)
         VALUES (?, ?, ?, ?, ?, NULL, NULL, ?, ?)",
    )
    .bind(&persona_id)
    .bind(worker_id)
    .bind(tenant.to_string())
    .bind(format!("worker-{worker_id}"))
    .bind("op@example.test")
    .bind("op@example.test")
    .bind("[]")
    .execute(store.pool())
    .await
    .expect("seed persona");

    sqlx::query(
        "INSERT INTO worker
            (id, tenant_id, vertical_id, persona_id, role_id, memory_scope,
             auto_approval_json, channels_json, created_at, trust_grade)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(worker_id)
    .bind(tenant.to_string())
    .bind(vertical)
    .bind(persona_id)
    .bind(format!("role-{worker_id}"))
    .bind("worker_self")
    .bind("{}")
    .bind("[]")
    .bind(0_i64)
    .bind(trust_grade)
    .execute(store.pool())
    .await
    .expect("seed worker");
}

async fn add_worker_skill(store: &SessionStore, worker_id: &str, skill_name: &str) {
    sqlx::query(
        "INSERT INTO worker_skill (worker_id, skill_id, proficiency)
         VALUES (?, ?, ?)
         ON CONFLICT(worker_id, skill_id) DO NOTHING",
    )
    .bind(worker_id)
    .bind(skill_name)
    .bind(3_i64)
    .execute(store.pool())
    .await
    .expect("add worker_skill");
}

async fn add_skill_row(store: &SessionStore, name: &str, version: &str) {
    // uuid::Uuid::new_v4 isn't enabled in this crate's feature flag set;
    // hand-roll a deterministic 16-byte blob via name+version hash.
    let mut id = [0u8; 16];
    let bytes = format!("{name}@{version}").into_bytes();
    for (i, b) in bytes.iter().take(16).enumerate() {
        id[i] = *b;
    }
    sqlx::query(
        "INSERT INTO skill (id, name, version, body, frontmatter, source, hash, created_at)
         VALUES (?, ?, ?, '', '{}', 'test', ?, 0)",
    )
    .bind(id.to_vec())
    .bind(name)
    .bind(version)
    .bind(vec![0u8; 32])
    .execute(store.pool())
    .await
    .expect("insert skill");
}

// Static guard so the gateway cap stays operator-meaningful even if a
// future refactor reaches for a smaller value.
const _: () = assert!(MAX_COHORTS >= 50);

#[tokio::test]
async fn empty_fleet_returns_empty_cohorts() {
    let (store, tenant) = store_with_tenant().await;
    let cohorts = list_cohorts(&store, tenant).await.expect("list");
    assert!(cohorts.is_empty());
}

#[tokio::test]
async fn workers_collapse_by_vertical_and_trust_band() {
    let (store, tenant) = store_with_tenant().await;
    // Two workers, same vertical, both in the "developing" band (1..=3).
    seed_worker(&store, tenant, "w1", "support", 2).await;
    seed_worker(&store, tenant, "w2", "support", 3).await;
    // Third worker, same vertical but "trusted" band — separate cohort.
    seed_worker(&store, tenant, "w3", "support", 8).await;
    // Fourth worker, different vertical — separate cohort.
    seed_worker(&store, tenant, "w4", "sales", 2).await;

    let cohorts = list_cohorts(&store, tenant).await.expect("list");
    assert_eq!(cohorts.len(), 3);
    let support_dev = cohorts
        .iter()
        .find(|c| c.cohort_id == "support::developing")
        .expect("support::developing");
    assert_eq!(support_dev.member_count, 2);
    assert_eq!(support_dev.vertical, "support");
    assert_eq!(support_dev.trust_grade_min, 1);
    assert_eq!(support_dev.trust_grade_max, 3);
}

#[tokio::test]
async fn skill_count_is_distinct_across_members() {
    let (store, tenant) = store_with_tenant().await;
    seed_worker(&store, tenant, "w1", "support", 2).await;
    seed_worker(&store, tenant, "w2", "support", 2).await;
    add_worker_skill(&store, "w1", "follow_up").await;
    add_worker_skill(&store, "w1", "qualify").await;
    add_worker_skill(&store, "w2", "follow_up").await;
    // w2's "follow_up" must NOT double-count; cohort has 2 distinct skills.
    let cohorts = list_cohorts(&store, tenant).await.expect("list");
    let c = cohorts.first().expect("one cohort");
    assert_eq!(c.member_count, 2);
    assert_eq!(c.skill_count, 2);
}

#[tokio::test]
async fn cohort_detail_orders_members_by_trust_then_id() {
    let (store, tenant) = store_with_tenant().await;
    seed_worker(&store, tenant, "wA", "support", 2).await;
    seed_worker(&store, tenant, "wB", "support", 3).await;
    seed_worker(&store, tenant, "wC", "support", 3).await;

    let detail = cohort_detail(&store, tenant, "support::developing")
        .await
        .expect("query")
        .expect("cohort exists");
    assert_eq!(detail.members.len(), 3);
    // trust_grade=3 (wB, wC) before trust_grade=2 (wA); ties alphabetical.
    let order: Vec<&str> = detail
        .members
        .iter()
        .map(|m| m.worker_id.as_str())
        .collect();
    assert_eq!(order, vec!["wB", "wC", "wA"]);
}

#[tokio::test]
async fn skill_coverage_pct_reflects_member_holders() {
    let (store, tenant) = store_with_tenant().await;
    seed_worker(&store, tenant, "w1", "support", 2).await;
    seed_worker(&store, tenant, "w2", "support", 2).await;
    seed_worker(&store, tenant, "w3", "support", 2).await;
    seed_worker(&store, tenant, "w4", "support", 2).await;
    add_worker_skill(&store, "w1", "follow_up").await;
    add_worker_skill(&store, "w2", "follow_up").await;
    add_worker_skill(&store, "w3", "follow_up").await;
    // 3 / 4 = 75%
    add_worker_skill(&store, "w1", "qualify").await;
    // 1 / 4 = 25%

    let detail = cohort_detail(&store, tenant, "support::developing")
        .await
        .expect("query")
        .expect("cohort exists");
    let follow_up = detail
        .skills
        .iter()
        .find(|s| s.skill_name == "follow_up")
        .expect("follow_up row");
    assert_eq!(follow_up.member_hits, 3);
    assert!((follow_up.member_coverage_pct - 75.0).abs() < 0.001);
    // Skills sorted DESC by coverage — follow_up before qualify.
    assert_eq!(detail.skills[0].skill_name, "follow_up");
    assert_eq!(detail.skills[1].skill_name, "qualify");
}

#[tokio::test]
async fn missing_cohort_returns_none() {
    let (store, tenant) = store_with_tenant().await;
    seed_worker(&store, tenant, "w1", "support", 2).await;
    // Wrong cohort id format.
    assert!(cohort_detail(&store, tenant, "not-a-cohort")
        .await
        .expect("query")
        .is_none());
    // Unknown band label.
    assert!(cohort_detail(&store, tenant, "support::imaginary")
        .await
        .expect("query")
        .is_none());
    // Valid id, no matching workers.
    assert!(cohort_detail(&store, tenant, "support::trusted")
        .await
        .expect("query")
        .is_none());
}

#[tokio::test]
async fn skill_version_label_picks_latest_lexicographic() {
    let (store, tenant) = store_with_tenant().await;
    seed_worker(&store, tenant, "w1", "support", 2).await;
    add_worker_skill(&store, "w1", "follow_up").await;
    add_skill_row(&store, "follow_up", "1.0.0").await;
    add_skill_row(&store, "follow_up", "1.2.0").await;
    add_skill_row(&store, "follow_up", "1.1.0").await;

    let detail = cohort_detail(&store, tenant, "support::developing")
        .await
        .expect("query")
        .expect("cohort exists");
    let row = detail
        .skills
        .iter()
        .find(|s| s.skill_name == "follow_up")
        .expect("follow_up row");
    assert_eq!(row.version.as_deref(), Some("1.2.0"));
}
