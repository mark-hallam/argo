//! Pagination + status-filter + date-range coverage for
//! `routes_workflow_runs::list_runs_response`. Extracted from
//! `routes_workflow_runs_tests.rs` to keep both files under the
//! 500-LOC ceiling.

use argo_store::SessionStore;
use chrono::Utc;

use super::{fetch_runs, fresh_store, insert_run};

async fn seed_n_successes(store: &SessionStore, n: i64) {
    let now = Utc::now().timestamp_micros();
    let minute = 60_000_000_i64;
    for i in 0..n {
        insert_run(
            store,
            &format!("ok{i}"),
            "v/w",
            now - minute * (n - i),
            Some(now - minute * (n - i) + 100_000),
            Some(true),
        )
        .await;
    }
}

#[tokio::test]
async fn pagination_returns_correct_page() {
    let store = fresh_store().await;
    seed_n_successes(&store, 60).await;
    let (_, body) = fetch_runs(
        &store,
        "/api/v1/verticals/v/workflows/w/runs?page=1&page_size=25",
    )
    .await;
    assert_eq!(body["page"], 1);
    assert_eq!(body["page_size"], 25);
    assert_eq!(body["total_filtered"], 60);
    assert_eq!(body["runs"].as_array().unwrap().len(), 25);

    // Page 3 has only 10 rows (60 = 25 + 25 + 10).
    let (_, body) = fetch_runs(
        &store,
        "/api/v1/verticals/v/workflows/w/runs?page=3&page_size=25",
    )
    .await;
    assert_eq!(body["page"], 3);
    assert_eq!(body["runs"].as_array().unwrap().len(), 10);

    // Page beyond the data returns an empty slice but valid metadata.
    let (_, body) = fetch_runs(
        &store,
        "/api/v1/verticals/v/workflows/w/runs?page=99&page_size=25",
    )
    .await;
    assert_eq!(body["page"], 99);
    assert_eq!(body["total_filtered"], 60);
    assert!(body["runs"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn page_size_clamped_to_max_100() {
    let store = fresh_store().await;
    seed_n_successes(&store, 150).await;
    let (_, body) = fetch_runs(
        &store,
        "/api/v1/verticals/v/workflows/w/runs?page_size=500",
    )
    .await;
    // 500 → clamped to 100.
    assert_eq!(body["page_size"], 100);
    assert_eq!(body["runs"].as_array().unwrap().len(), 100);
}

#[tokio::test]
async fn status_filter_success_only() {
    let store = fresh_store().await;
    let now = Utc::now().timestamp_micros();
    insert_run(&store, "ok1", "v/w", now - 100, Some(now), Some(true)).await;
    insert_run(&store, "ok2", "v/w", now - 200, Some(now), Some(true)).await;
    insert_run(&store, "fail1", "v/w", now - 300, Some(now), Some(false)).await;
    insert_run(&store, "run1", "v/w", now - 400, None, None).await;

    let (_, body) = fetch_runs(
        &store,
        "/api/v1/verticals/v/workflows/w/runs?status[]=success",
    )
    .await;
    let runs = body["runs"].as_array().unwrap();
    assert_eq!(runs.len(), 2);
    assert!(runs.iter().all(|r| r["status"] == "success"));
    assert_eq!(body["total_filtered"], 2);
    // Aggregate ignores filter: still counts all 4 rows.
    assert_eq!(body["run_count"], 4);
}

#[tokio::test]
async fn status_filter_failed_and_running_multi_select() {
    let store = fresh_store().await;
    let now = Utc::now().timestamp_micros();
    insert_run(&store, "ok1", "v/w", now - 100, Some(now), Some(true)).await;
    insert_run(&store, "fail1", "v/w", now - 200, Some(now), Some(false)).await;
    insert_run(&store, "run1", "v/w", now - 300, None, None).await;

    let (_, body) = fetch_runs(
        &store,
        "/api/v1/verticals/v/workflows/w/runs?status[]=failed&status[]=running",
    )
    .await;
    let runs = body["runs"].as_array().unwrap();
    assert_eq!(runs.len(), 2);
    let statuses: Vec<&str> = runs.iter().map(|r| r["status"].as_str().unwrap()).collect();
    assert!(statuses.contains(&"failed"));
    assert!(statuses.contains(&"running"));
}

#[tokio::test]
async fn status_filter_queued_returns_empty_until_queue_persistence() {
    // The workflow_run schema doesn't represent queued runs (no row
    // exists until execution starts), so the chip filters to zero
    // by design. The UI chip is preserved for symmetry.
    let store = fresh_store().await;
    seed_n_successes(&store, 5).await;
    let (_, body) = fetch_runs(
        &store,
        "/api/v1/verticals/v/workflows/w/runs?status[]=queued",
    )
    .await;
    assert!(body["runs"].as_array().unwrap().is_empty());
    assert_eq!(body["total_filtered"], 0);
    // The aggregate still reports the full 5 rows over the window.
    assert_eq!(body["run_count"], 5);
}

#[tokio::test]
async fn status_filter_accepts_bare_status_param_too() {
    // Both `?status=success` and `?status[]=success` must work — the
    // frontend's URLSearchParams emits the bare form by default.
    let store = fresh_store().await;
    let now = Utc::now().timestamp_micros();
    insert_run(&store, "ok1", "v/w", now - 100, Some(now), Some(true)).await;
    insert_run(&store, "fail1", "v/w", now - 200, Some(now), Some(false)).await;

    let (_, body) = fetch_runs(
        &store,
        "/api/v1/verticals/v/workflows/w/runs?status=failed",
    )
    .await;
    let runs = body["runs"].as_array().unwrap();
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0]["status"], "failed");
}

#[tokio::test]
async fn date_range_since_until_overrides_days_window() {
    let store = fresh_store().await;
    let now = Utc::now().timestamp_micros();
    let day = 86_400_000_000_i64;
    insert_run(&store, "old", "v/w", now - day * 10, Some(now - day * 10 + 100), Some(true)).await;
    insert_run(&store, "mid", "v/w", now - day * 5, Some(now - day * 5 + 100), Some(true)).await;
    insert_run(&store, "new", "v/w", now - day, Some(now - day + 100), Some(true)).await;

    let since = now - day * 7;
    let until = now - day * 3;
    let path = format!(
        "/api/v1/verticals/v/workflows/w/runs?since_ts={since}&until_ts={until}"
    );
    let (_, body) = fetch_runs(&store, &path).await;
    let runs = body["runs"].as_array().unwrap();
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0]["id"], "mid");
    assert_eq!(body["since_ts"].as_i64(), Some(since));
    assert_eq!(body["until_ts"].as_i64(), Some(until));
    assert_eq!(body["run_count"], 1);
}

#[tokio::test]
async fn filter_combinations_status_plus_pagination() {
    // Brief acceptance: "At least one integration test covering the
    // filter combinations." Combines status + pagination.
    let store = fresh_store().await;
    let now = Utc::now().timestamp_micros();
    let minute = 60_000_000_i64;
    // 30 successes interleaved with 10 failures.
    for i in 0..30 {
        insert_run(
            &store,
            &format!("ok{i}"),
            "v/w",
            now - minute * (40 - i),
            Some(now - minute * (40 - i) + 100),
            Some(true),
        )
        .await;
    }
    for i in 0..10 {
        insert_run(
            &store,
            &format!("fail{i}"),
            "v/w",
            now - minute * (10 - i),
            Some(now - minute * (10 - i) + 100),
            Some(false),
        )
        .await;
    }

    // Filter to failures, page 1, page_size 5 → exactly 5 failures.
    let (_, body) = fetch_runs(
        &store,
        "/api/v1/verticals/v/workflows/w/runs?status[]=failed&page=1&page_size=5",
    )
    .await;
    let runs = body["runs"].as_array().unwrap();
    assert_eq!(runs.len(), 5);
    assert!(runs.iter().all(|r| r["status"] == "failed"));
    assert_eq!(body["total_filtered"], 10);
    assert_eq!(body["page"], 1);
    assert_eq!(body["page_size"], 5);

    // Page 2 → remaining 5 failures.
    let (_, body) = fetch_runs(
        &store,
        "/api/v1/verticals/v/workflows/w/runs?status[]=failed&page=2&page_size=5",
    )
    .await;
    assert_eq!(body["runs"].as_array().unwrap().len(), 5);

    // Page 3 with only-failed filter → empty.
    let (_, body) = fetch_runs(
        &store,
        "/api/v1/verticals/v/workflows/w/runs?status[]=failed&page=3&page_size=5",
    )
    .await;
    assert!(body["runs"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn row_status_field_present_for_running_rows() {
    let store = fresh_store().await;
    let now = Utc::now().timestamp_micros();
    insert_run(&store, "running1", "v/w", now - 100, None, None).await;
    let (_, body) = fetch_runs(&store, "/api/v1/verticals/v/workflows/w/runs").await;
    let runs = body["runs"].as_array().unwrap();
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0]["status"], "running");
    assert_eq!(runs[0]["ok"], false, "ok defaults to false for in-flight rows");
}
