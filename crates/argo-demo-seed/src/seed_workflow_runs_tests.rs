//! Tests for `seed_workflow_runs` — proves the canonical demo seed
//! lands real `workflow_run` rows under the compound `{vertical}/{name}`
//! key the live Scene-5 run-history route builds, so the detail page's
//! aggregates + recent-runs table populate, and proves idempotency.

use super::{seed_workflow_runs, DEMO_RUNS, WORKFLOW_RUN_ID};
use argo_store::SessionStore;
use std::sync::Arc;

async fn fresh_store() -> Arc<SessionStore> {
    let store = Arc::new(SessionStore::open(":memory:").await.expect("open"));
    store.migrate().await.expect("migrate");
    store
}

#[tokio::test]
async fn workflow_run_id_matches_the_seeded_workflow_compound_key() {
    // Load-bearing: the run-history endpoint matches `workflow_id IN
    // (compound_key, bare_key)` where compound = "{vertical}/{name}".
    // The seeded workflow rides under vertical `marketplace`
    // (workflow::DEMO_VERTICAL_ID) with name `customer_research_followup`
    // (workflow::DEMO_WORKFLOW_NAME). If either constant moves, this
    // pin fails and the runs would no longer match the live route.
    assert_eq!(WORKFLOW_RUN_ID, "marketplace/customer_research_followup");
}

#[tokio::test]
async fn seeds_all_demo_runs_under_the_compound_key() {
    let store = fresh_store().await;
    let inserted = seed_workflow_runs(&store).await.expect("seed");
    assert_eq!(inserted, DEMO_RUNS.len());

    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM workflow_run WHERE workflow_id = ?")
        .bind(WORKFLOW_RUN_ID)
        .fetch_one(store.pool())
        .await
        .expect("count");
    assert_eq!(count, DEMO_RUNS.len() as i64);
}

#[tokio::test]
async fn run_mix_drives_a_believable_aggregate() {
    // One running row (no finish, ok NULL), one failure (ok=0), the
    // rest succeed (ok=1) — so the panel shows each status chip and a
    // non-100% success rate.
    let store = fresh_store().await;
    seed_workflow_runs(&store).await.expect("seed");

    let running: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM workflow_run WHERE workflow_id = ? AND finished_at IS NULL",
    )
    .bind(WORKFLOW_RUN_ID)
    .fetch_one(store.pool())
    .await
    .expect("running");
    assert_eq!(running, 1, "exactly one still-running row");

    let failed: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM workflow_run WHERE workflow_id = ? AND ok = 0",
    )
    .bind(WORKFLOW_RUN_ID)
    .fetch_one(store.pool())
    .await
    .expect("failed");
    assert_eq!(failed, 1, "exactly one failed row");

    let succeeded: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM workflow_run WHERE workflow_id = ? AND ok = 1",
    )
    .bind(WORKFLOW_RUN_ID)
    .fetch_one(store.pool())
    .await
    .expect("succeeded");
    assert!(succeeded >= 2, "most runs succeed");

    // Every finished row has finished_at > started_at (non-negative
    // duration the panel's avg/p95 math depends on).
    let bad_durations: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM workflow_run \
         WHERE workflow_id = ? AND finished_at IS NOT NULL AND finished_at <= started_at",
    )
    .bind(WORKFLOW_RUN_ID)
    .fetch_one(store.pool())
    .await
    .expect("durations");
    assert_eq!(bad_durations, 0, "no finished run has a non-positive duration");
}

#[tokio::test]
async fn seed_is_idempotent() {
    let store = fresh_store().await;
    let first = seed_workflow_runs(&store).await.expect("first");
    let second = seed_workflow_runs(&store).await.expect("second");
    assert!(first > 0);
    assert_eq!(second, 0, "re-seed must not duplicate run history");

    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM workflow_run WHERE workflow_id = ?")
        .bind(WORKFLOW_RUN_ID)
        .fetch_one(store.pool())
        .await
        .expect("count");
    assert_eq!(count, first as i64, "row count stable after re-seed");
}
