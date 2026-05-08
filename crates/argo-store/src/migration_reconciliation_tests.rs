use crate::SessionStore;
use sqlx::Row;

#[tokio::test]
async fn fresh_migrate_reaches_v15_and_creates_reconciled_schema() {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();

    assert_eq!(store.schema_version().await.unwrap(), 15);
    for table in [
        "persona",
        "worker",
        "duty",
        "worker_budget",
        "memory_entry_fts",
        "memory_embedding",
        "shared_team_memory",
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
    assert_schema_version_singleton(&store).await;
}

#[tokio::test]
async fn raw_v12_to_v15_legacy_schema_migrates_without_duplicate_errors() {
    let store = baseline_v11().await;
    for raw in [
        include_str!("../migrations/12__worker_identity.sql"),
        include_str!("../migrations/13__memory_layers.sql"),
        include_str!("../migrations/14__worker_budget.sql"),
        include_str!("../migrations/15__shared_team_memory.sql"),
    ] {
        apply_raw(&store, raw).await;
    }
    assert_eq!(store.schema_version().await.unwrap(), 11);

    store.migrate().await.unwrap();

    assert_eq!(store.schema_version().await.unwrap(), 15);
    assert!(table_exists(&store, "shared_team_memory").await);
    assert_columns(&store, "memory_entry", &["layer", "tenant_id", "worker_id"]).await;
    assert_schema_version_singleton(&store).await;
}

#[tokio::test]
async fn partial_legacy_worker_identity_migrates_to_full_v15() {
    let store = baseline_v11().await;
    apply_raw(
        &store,
        include_str!("../migrations/12__worker_identity.sql"),
    )
    .await;

    store.migrate().await.unwrap();

    assert_eq!(store.schema_version().await.unwrap(), 15);
    assert!(table_exists(&store, "worker").await);
    assert!(table_exists(&store, "worker_budget").await);
    assert!(table_exists(&store, "shared_team_memory").await);
    assert_columns(
        &store,
        "memory_entry",
        &["layer", "tenant_id", "worker_id", "memory_scope"],
    )
    .await;
}

#[tokio::test]
async fn rollback_to_v11_returns_clear_unsupported_error() {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();

    let err = store.migrate_down_to(11).await.expect_err("unsupported");
    assert!(err.to_string().contains("rollback unsupported"));
    assert_eq!(store.schema_version().await.unwrap(), 15);
}

async fn baseline_v11() -> SessionStore {
    let store = SessionStore::open(":memory:").await.unwrap();
    crate::migrations::run_up_to(store.pool(), 11)
        .await
        .unwrap();
    store
}

async fn apply_raw(store: &SessionStore, body: &str) {
    for stmt in body
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
