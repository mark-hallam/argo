use crate::SessionStore;
use sqlx::Row;

#[tokio::test]
async fn fresh_migrate_reaches_v21_and_creates_reconciled_schema() {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();

    assert_eq!(store.schema_version().await.unwrap(), 21);
    for table in [
        "persona",
        "worker",
        "duty",
        "worker_budget",
        "memory_entry_fts",
        "memory_embedding",
        "shared_team_memory",
        "handoff",
        "worker_circuit_history",
        "workflow_run",
        "worker_behavior_baseline",
        "worker_anomaly_event",
        "skill_ab_test_run",
        "skill_ab_test_outcome",
    ] {
        assert!(table_exists(&store, table).await, "missing {table}");
    }
    assert_columns(
        &store,
        "role",
        &["worker_id", "charter", "kpis_json", "escalation"],
    )
    .await;
    assert_columns(
        &store,
        "memory_entry",
        &["layer", "tenant_id", "worker_id", "memory_scope"],
    )
    .await;
    assert_columns(
        &store,
        "session",
        &["status", "participants", "created_at", "deadline"],
    )
    .await;
    assert_columns(&store, "worker", &["fs_root"]).await;
    assert_schema_version_singleton(&store).await;
}

#[tokio::test]
async fn raw_v12_to_v21_legacy_schema_migrates_without_duplicate_errors() {
    let store = baseline_v11().await;
    for raw in [
        include_str!("../migrations/12__worker_identity.sql"),
        include_str!("../migrations/13__memory_layers.sql"),
        include_str!("../migrations/14__worker_budget.sql"),
        include_str!("../migrations/15__shared_team_memory.sql"),
        include_str!("../migrations/16__orchestrator.sql"),
        include_str!("../migrations/17__worker_circuit_history.sql"),
        include_str!("../migrations/18__workflow_runs.sql"),
        include_str!("../migrations/19__anomaly.sql"),
        include_str!("../migrations/19__worker_fs_root.sql"),
        include_str!("../migrations/20__skill_ab_tests.sql"),
    ] {
        apply_raw(&store, raw).await;
    }
    assert_eq!(store.schema_version().await.unwrap(), 11);

    store.migrate().await.unwrap();

    assert_eq!(store.schema_version().await.unwrap(), 21);
    for table in [
        "shared_team_memory",
        "handoff",
        "worker_circuit_history",
        "workflow_run",
        "worker_behavior_baseline",
        "worker_anomaly_event",
        "skill_ab_test_run",
        "skill_ab_test_outcome",
    ] {
        assert!(table_exists(&store, table).await, "missing {table}");
    }
    assert_columns(
        &store,
        "session",
        &["status", "participants", "created_at", "deadline"],
    )
    .await;
    assert_columns(&store, "worker", &["fs_root"]).await;
    assert_schema_version_singleton(&store).await;
}

#[tokio::test]
async fn partial_legacy_19_collision_pair_migrates_to_full_v21() {
    let store = baseline_v11().await;
    apply_raw(
        &store,
        include_str!("../migrations/12__worker_identity.sql"),
    )
    .await;
    apply_raw(&store, include_str!("../migrations/19__anomaly.sql")).await;
    apply_raw(
        &store,
        include_str!("../migrations/19__worker_fs_root.sql"),
    )
    .await;

    store.migrate().await.unwrap();

    assert_eq!(store.schema_version().await.unwrap(), 21);
    assert!(table_exists(&store, "worker_anomaly_event").await);
    assert!(table_exists(&store, "worker_behavior_baseline").await);
    assert!(table_exists(&store, "skill_ab_test_run").await);
    assert_columns(&store, "worker", &["fs_root"]).await;
}

#[tokio::test]
async fn rollback_to_v16_drops_v17_through_v21_tables() {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    assert_eq!(store.schema_version().await.unwrap(), 21);

    store.migrate_down_to(16).await.unwrap();

    assert_eq!(store.schema_version().await.unwrap(), 16);
    assert!(table_exists(&store, "handoff").await);
    assert_columns(
        &store,
        "session",
        &["status", "participants", "created_at", "deadline"],
    )
    .await;
    for dropped in [
        "worker_circuit_history",
        "workflow_run",
        "worker_behavior_baseline",
        "worker_anomaly_event",
        "skill_ab_test_run",
        "skill_ab_test_outcome",
    ] {
        assert!(
            !table_exists(&store, dropped).await,
            "expected {dropped} dropped after roll_back_to(v16)"
        );
    }
    let worker_columns = sqlx::query("PRAGMA table_info(worker)")
        .fetch_all(store.pool())
        .await
        .unwrap();
    assert!(
        !worker_columns
            .iter()
            .any(|row| row.get::<String, _>("name") == "fs_root"),
        "expected worker.fs_root dropped after roll_back_to(v16)"
    );
}

#[tokio::test]
async fn rollback_to_v11_returns_clear_unsupported_error() {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();

    let err = store.migrate_down_to(11).await.expect_err("unsupported");
    assert!(err.to_string().contains("rollback unsupported"));
    // After v17..v21 unwound, v16 refuses and the schema_version
    // settles at 16 — the highest layer whose down() is supported.
    assert_eq!(store.schema_version().await.unwrap(), 16);
}

async fn baseline_v11() -> SessionStore {
    let store = SessionStore::open(":memory:").await.unwrap();
    crate::migrations::run_up_to(store.pool(), 11)
        .await
        .unwrap();
    store
}

async fn apply_raw(store: &SessionStore, body: &str) {
    // Strip `--` line comments before splitting on `;` so semicolons
    // inside SQL prose comments (e.g. `19__worker_fs_root.sql`) do
    // not split mid-statement.
    let stripped: String = body
        .lines()
        .map(|line| match line.find("--") {
            Some(idx) => &line[..idx],
            None => line,
        })
        .collect::<Vec<_>>()
        .join("\n");
    for stmt in stripped
        .split_terminator(';')
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        match sqlx::query(stmt).execute(store.pool()).await {
            Ok(_) => {}
            Err(e)
                if stmt.contains("memory_vec_index")
                    && e.to_string().contains("no such module: vec0") => {}
            Err(e) => panic!("raw migration failed: {e}; stmt={stmt}"),
        }
    }
}

async fn table_exists(store: &SessionStore, table: &str) -> bool {
    let row: Option<(String,)> = sqlx::query_as(
        "SELECT name FROM sqlite_master WHERE type IN ('table','view') AND name = ?",
    )
    .bind(table)
    .fetch_optional(store.pool())
    .await
    .unwrap();
    row.is_some()
}

async fn assert_columns(store: &SessionStore, table: &str, columns: &[&str]) {
    let rows = sqlx::query(&format!("PRAGMA table_info({table})"))
        .fetch_all(store.pool())
        .await
        .unwrap();
    for column in columns {
        assert!(
            rows.iter()
                .any(|row| row.get::<String, _>("name") == *column),
            "missing {table}.{column}"
        );
    }
}

async fn assert_schema_version_singleton(store: &SessionStore) {
    let rows: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM schema_version")
        .fetch_one(store.pool())
        .await
        .unwrap();
    assert_eq!(rows, 1);
}
