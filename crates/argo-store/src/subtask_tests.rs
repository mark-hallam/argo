use super::*;
use crate::SessionStore;

async fn fresh_pool() -> SqlitePool {
    // The subtask_ledger table now lives in `migrations/v39.rs`; a
    // migrated SessionStore gives us the full ladder including the
    // subtask schema with no raw SQL needed.
    let store = SessionStore::open(":memory:").await.expect("open store");
    store.migrate().await.expect("migrate");
    store.pool().clone()
}

#[tokio::test]
async fn migrate_yields_subtask_ledger_table() {
    let pool = fresh_pool().await;
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'subtask_ledger'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(count, 1);
}

#[tokio::test]
async fn insert_and_get_round_trip() {
    let pool = fresh_pool().await;
    let id = insert(
        &pool,
        &NewSubtask {
            parent_goal: "promote launch",
            parent_session_id: Some("sess-1"),
            worker_id: "wkr-1",
            plan_node_path: "/marketing/draft",
            description: "draft a launch announcement",
        },
    )
    .await
    .expect("insert");
    let row = get(&pool, &id).await.expect("get").expect("row exists");
    assert_eq!(row.parent_goal, "promote launch");
    assert_eq!(row.parent_session_id.as_deref(), Some("sess-1"));
    assert_eq!(row.worker_id, "wkr-1");
    assert_eq!(row.plan_node_path, "/marketing/draft");
    assert_eq!(row.description, "draft a launch announcement");
    assert_eq!(row.state, SubtaskState::Pending);
    assert!(row.output.is_none());
    assert!(row.started_at.is_none());
    assert!(row.completed_at.is_none());
}

#[tokio::test]
async fn update_state_transitions_persist_timestamps() {
    let pool = fresh_pool().await;
    let id = insert(
        &pool,
        &NewSubtask {
            parent_goal: "g",
            parent_session_id: Some("s"),
            worker_id: "w",
            plan_node_path: "/n",
            description: "d",
        },
    )
    .await
    .expect("insert");

    update_state(&pool, &id, SubtaskState::Running, None)
        .await
        .expect("running");
    let after_running = get(&pool, &id).await.unwrap().unwrap();
    assert_eq!(after_running.state, SubtaskState::Running);
    assert!(after_running.started_at.is_some());
    assert!(after_running.completed_at.is_none());

    update_state(&pool, &id, SubtaskState::Done, Some(r#"{"k":"v"}"#))
        .await
        .expect("done");
    let after_done = get(&pool, &id).await.unwrap().unwrap();
    assert_eq!(after_done.state, SubtaskState::Done);
    assert_eq!(after_done.output.as_deref(), Some(r#"{"k":"v"}"#));
    assert!(after_done.started_at.is_some());
    assert!(after_done.completed_at.is_some());
    assert_eq!(after_done.started_at, after_running.started_at);
}

#[tokio::test]
async fn update_state_unknown_id_errors() {
    let pool = fresh_pool().await;
    let err = update_state(&pool, "no-such-id", SubtaskState::Running, None)
        .await
        .expect_err("expected error");
    assert!(format!("{err}").contains("no row"));
}

#[tokio::test]
async fn list_filters_compose() {
    let pool = fresh_pool().await;
    for path in ["/a/1", "/a/2", "/b/1"] {
        insert(
            &pool,
            &NewSubtask {
                parent_goal: "g",
                parent_session_id: Some("s"),
                worker_id: if path.starts_with("/a") { "alpha" } else { "beta" },
                plan_node_path: path,
                description: "d",
            },
        )
        .await
        .unwrap();
    }
    // The runtime claim 4× rule: don't trust insert order alone for
    // sub-millisecond inserts on the same UUIDv7 timestamp; we
    // just check the filter SQL is correct.
    let alpha = list(
        &pool,
        &SubtaskFilters {
            worker_id: Some("alpha"),
            ..Default::default()
        },
        10,
    )
    .await
    .unwrap();
    assert_eq!(alpha.len(), 2);
    assert!(alpha.iter().all(|r| r.worker_id == "alpha"));

    let beta = list(
        &pool,
        &SubtaskFilters {
            worker_id: Some("beta"),
            parent_goal: Some("g"),
            ..Default::default()
        },
        10,
    )
    .await
    .unwrap();
    assert_eq!(beta.len(), 1);
    assert_eq!(beta[0].plan_node_path, "/b/1");
}

#[tokio::test]
async fn list_by_session_ordered_ascending() {
    let pool = fresh_pool().await;
    // Insert with a small sleep to guarantee distinct
    // timestamp_micros values across rows.
    let mut ids = Vec::new();
    for i in 0..3 {
        let path = format!("/x/{i}");
        let id = insert(
            &pool,
            &NewSubtask {
                parent_goal: "g",
                parent_session_id: Some("sess-x"),
                worker_id: "w",
                plan_node_path: &path,
                description: "d",
            },
        )
        .await
        .unwrap();
        ids.push(id);
        tokio::time::sleep(std::time::Duration::from_millis(2)).await;
    }
    let rows = list_by_session(&pool, "sess-x").await.unwrap();
    let returned: Vec<&str> = rows.iter().map(|r| r.id.as_str()).collect();
    let expected: Vec<&str> = ids.iter().map(String::as_str).collect();
    assert_eq!(returned, expected);
}

#[tokio::test]
async fn unique_path_per_session_worker() {
    let pool = fresh_pool().await;
    insert(
        &pool,
        &NewSubtask {
            parent_goal: "g",
            parent_session_id: Some("s"),
            worker_id: "w",
            plan_node_path: "/dup",
            description: "d",
        },
    )
    .await
    .unwrap();
    let err = insert(
        &pool,
        &NewSubtask {
            parent_goal: "g",
            parent_session_id: Some("s"),
            worker_id: "w",
            plan_node_path: "/dup",
            description: "d",
        },
    )
    .await
    .expect_err("expected unique-index violation");
    assert!(format!("{err}").to_lowercase().contains("unique"));
}
