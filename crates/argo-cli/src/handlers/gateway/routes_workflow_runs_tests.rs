//! Unit coverage for `routes_workflow_runs::list_runs_response`.
//!
//! Boots an in-memory `SessionStore`, seeds `workflow_run` rows via raw
//! sqlx (no need to spin up the workflow CLI), and asserts the JSON
//! envelope shape + the aggregate math.

use argo_store::SessionStore;
use chrono::Utc;
use serde_json::Value;
use std::sync::Arc;

use super::dispatch_get;

async fn fresh_store() -> Arc<SessionStore> {
    let store = SessionStore::open(":memory:").await.expect("open store");
    // Create only the workflow_run schema (migration 18). We skip
    // `store.migrate()` to avoid the pre-existing v43 migration bug
    // (raw-SQL splitter trips on a semicolon inside a SQL comment in
    // `47__skill_canary.sql`); none of this module's tests need any
    // other table so the partial schema is sufficient.
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS workflow_run (\
            id TEXT PRIMARY KEY, \
            workflow_id TEXT NOT NULL, \
            workflow_yaml TEXT NOT NULL, \
            started_at INTEGER NOT NULL, \
            finished_at INTEGER, \
            ok INTEGER, \
            task_verdicts_json TEXT\
         )",
    )
    .execute(store.pool())
    .await
    .expect("create workflow_run");
    Arc::new(store)
}

async fn insert_run(
    store: &SessionStore,
    id: &str,
    workflow_id: &str,
    started_at_micros: i64,
    finished_at_micros: Option<i64>,
    ok: Option<bool>,
) {
    sqlx::query(
        "INSERT INTO workflow_run \
         (id, workflow_id, workflow_yaml, started_at, finished_at, ok, task_verdicts_json) \
         VALUES (?, ?, '', ?, ?, ?, NULL)",
    )
    .bind(id)
    .bind(workflow_id)
    .bind(started_at_micros)
    .bind(finished_at_micros)
    .bind(ok.map(|b| if b { 1_i64 } else { 0 }))
    .execute(store.pool())
    .await
    .expect("insert");
}

fn req(path: &str) -> String {
    format!("GET {path} HTTP/1.1\r\nHost: x\r\n\r\n")
}

async fn fetch_runs(store: &Arc<SessionStore>, path: &str) -> (String, Value) {
    let (status, body) = dispatch_get(&req(path), Some(store))
        .await
        .expect("must dispatch /runs path");
    let parsed: Value = serde_json::from_str(&body).expect("json");
    (status.to_string(), parsed)
}

#[tokio::test]
async fn list_runs_returns_aggregates_and_recent_rows() {
    let store = fresh_store().await;
    let now = Utc::now().timestamp_micros();
    let one_hour = 3_600_000_000_i64;
    // Workflow with 4 runs in the last hour: 3 successes + 1 failure,
    // durations 1s, 2s, 3s, 4s (in micros).
    insert_run(&store, "r1", "pr_checker/standard_review", now - one_hour * 4, Some(now - one_hour * 4 + 1_000_000), Some(true)).await;
    insert_run(&store, "r2", "pr_checker/standard_review", now - one_hour * 3, Some(now - one_hour * 3 + 2_000_000), Some(true)).await;
    insert_run(&store, "r3", "pr_checker/standard_review", now - one_hour * 2, Some(now - one_hour * 2 + 3_000_000), Some(true)).await;
    insert_run(&store, "r4", "pr_checker/standard_review", now - one_hour, Some(now - one_hour + 4_000_000), Some(false)).await;

    let (status, body) = fetch_runs(
        &store,
        "/api/v1/verticals/pr_checker/workflows/standard_review/runs",
    )
    .await;
    assert_eq!(status, "200 OK");
    assert_eq!(body["run_count"], 4);
    let rate = body["success_rate"].as_f64().expect("rate present");
    assert!((rate - 0.75).abs() < 1e-9, "want 0.75, got {rate}");
    assert_eq!(body["window_days"], 30);
    assert_eq!(body["vertical_id"], "pr_checker");
    assert_eq!(body["workflow_name"], "standard_review");
    // 4 rows ordered DESC by started_at — newest first is r4 (the failure).
    let runs = body["runs"].as_array().expect("runs array");
    assert_eq!(runs.len(), 4);
    assert_eq!(runs[0]["id"], "r4");
    assert_eq!(runs[0]["ok"], false);
    assert!(body["last_run_at"].is_string());
}

#[tokio::test]
async fn list_runs_empty_workflow_returns_null_success_rate() {
    let store = fresh_store().await;
    let (status, body) = fetch_runs(
        &store,
        "/api/v1/verticals/empty_v/workflows/never_run/runs",
    )
    .await;
    assert_eq!(status, "200 OK");
    assert_eq!(body["run_count"], 0);
    assert!(body["success_rate"].is_null(), "0/0 must be null");
    assert_eq!(body["avg_duration_ms"], 0);
    assert_eq!(body["p95_duration_ms"], 0);
    assert!(body["last_run_at"].is_null());
    assert!(body["runs"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn list_runs_respects_days_filter() {
    let store = fresh_store().await;
    let now = Utc::now().timestamp_micros();
    let day = 86_400_000_000_i64;
    // 1 recent (2 days ago) + 1 old (60 days ago, outside the 30-day default).
    insert_run(&store, "recent", "pr_checker/standard_review", now - day * 2, Some(now - day * 2 + 1_000_000), Some(true)).await;
    insert_run(&store, "stale", "pr_checker/standard_review", now - day * 60, Some(now - day * 60 + 1_000_000), Some(true)).await;

    // Default 30-day window: only the recent row counts.
    let (_, body) = fetch_runs(
        &store,
        "/api/v1/verticals/pr_checker/workflows/standard_review/runs",
    )
    .await;
    assert_eq!(body["run_count"], 1);
    assert_eq!(body["runs"].as_array().unwrap().len(), 1);

    // 90-day window: both rows count.
    let (_, body) = fetch_runs(
        &store,
        "/api/v1/verticals/pr_checker/workflows/standard_review/runs?days=90",
    )
    .await;
    assert_eq!(body["run_count"], 2);
    assert_eq!(body["window_days"], 90);
}

#[tokio::test]
async fn list_runs_respects_limit() {
    let store = fresh_store().await;
    let now = Utc::now().timestamp_micros();
    let minute = 60_000_000_i64;
    for i in 0..10 {
        insert_run(
            &store,
            &format!("r{i}"),
            "pr_checker/standard_review",
            now - minute * (10 - i),
            Some(now - minute * (10 - i) + 100_000),
            Some(true),
        )
        .await;
    }
    let (_, body) = fetch_runs(
        &store,
        "/api/v1/verticals/pr_checker/workflows/standard_review/runs?limit=3",
    )
    .await;
    assert_eq!(body["run_count"], 10, "aggregate counts all rows in window");
    assert_eq!(
        body["runs"].as_array().unwrap().len(),
        3,
        "recent-rows array is capped by limit"
    );
}

#[tokio::test]
async fn success_rate_computed_correctly() {
    let store = fresh_store().await;
    let now = Utc::now().timestamp_micros();
    insert_run(&store, "ok1", "v1/w", now - 1_000_000, Some(now), Some(true)).await;
    insert_run(&store, "ok2", "v1/w", now - 2_000_000, Some(now), Some(true)).await;
    insert_run(&store, "ok3", "v1/w", now - 3_000_000, Some(now), Some(true)).await;
    insert_run(&store, "ok4", "v1/w", now - 4_000_000, Some(now), Some(true)).await;
    insert_run(&store, "fail", "v1/w", now - 5_000_000, Some(now), Some(false)).await;

    let (_, body) = fetch_runs(&store, "/api/v1/verticals/v1/workflows/w/runs").await;
    let rate = body["success_rate"].as_f64().expect("rate present");
    assert!((rate - 0.8).abs() < 1e-9, "4/5 = 0.8; got {rate}");
}

#[tokio::test]
async fn p95_duration_within_top_5pct_bucket() {
    let store = fresh_store().await;
    let now = Utc::now().timestamp_micros();
    // 20 runs with durations 1ms, 2ms, ..., 20ms (in micros).
    for i in 1..=20 {
        let dur_micros = (i as i64) * 1_000;
        insert_run(
            &store,
            &format!("r{i}"),
            "v/p",
            now - 1_000_000 - dur_micros,
            Some(now - 1_000_000),
            Some(true),
        )
        .await;
    }
    let (_, body) = fetch_runs(&store, "/api/v1/verticals/v/workflows/p/runs").await;
    let p95_ms = body["p95_duration_ms"].as_i64().expect("p95 present");
    // floor(20 * 0.95) = 19, sorted asc index 19 → duration 20ms.
    // Implementation may pick 19ms (idx clamped to len-1=19 → element 20ms).
    assert!(
        (19..=20).contains(&p95_ms),
        "p95 should be in the top-5pct bucket; got {p95_ms}ms"
    );
}

#[tokio::test]
async fn list_runs_matches_bare_workflow_id_convention() {
    // Some workspaces store workflow_id as the bare YAML id (e.g.
    // "standard_review") rather than the synthetic
    // "<vertical>/<name>" key. The endpoint matches both.
    let store = fresh_store().await;
    let now = Utc::now().timestamp_micros();
    insert_run(&store, "bare", "standard_review", now - 1_000_000, Some(now), Some(true)).await;
    let (_, body) = fetch_runs(
        &store,
        "/api/v1/verticals/pr_checker/workflows/standard_review/runs",
    )
    .await;
    assert_eq!(body["run_count"], 1, "bare-key row must be matched");
}

#[tokio::test]
async fn store_unavailable_returns_503() {
    let (status, _) = dispatch_get(&req("/api/v1/verticals/v/workflows/n/runs"), None)
        .await
        .expect("must dispatch");
    assert_eq!(status, "503 Service Unavailable");
}

#[test]
fn unrelated_paths_return_none() {
    // These tests are sync because dispatch_get short-circuits on
    // parse failure before any await.
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let store: Option<&Arc<SessionStore>> = None;
        // No `/runs` suffix.
        assert!(dispatch_get(&req("/api/v1/verticals/v/workflows/n"), store)
            .await
            .is_none());
        // Wrong verb is impossible here (dispatch_get is GET-only), but
        // wrong root prefix must also bail.
        assert!(dispatch_get(&req("/api/v1/something-else"), store)
            .await
            .is_none());
    });
}
