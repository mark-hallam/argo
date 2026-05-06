use argo_store::SessionStore;
use sqlx::Row;
use std::path::{Path, PathBuf};
use uuid::Uuid;

#[tokio::test]
async fn migration_chain_applies_and_h_stack_tables_accept_inserts() {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    apply_sql_file_migrations(&store, false).await.unwrap();
    store.migrate().await.unwrap();
    apply_sql_file_migrations(&store, true).await.unwrap();

    for table in expected_tables() {
        assert!(
            table_exists(&store, table).await.unwrap(),
            "missing expected table {table}"
        );
    }

    insert_worker_budget_if_present(&store).await.unwrap();
    insert_orchestrator_session_if_present(&store).await.unwrap();
    insert_handoff_if_present(&store).await.unwrap();
    insert_shared_team_memory_if_present(&store).await.unwrap();
}

async fn apply_sql_file_migrations(store: &SessionStore, idempotent: bool) -> Result<(), String> {
    for path in migration_files()? {
        let body = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
        for stmt in body.split_terminator(';').map(str::trim).filter(|s| !s.is_empty()) {
            match sqlx::query(stmt).execute(store.pool()).await {
                Ok(_) => {}
                Err(e) if is_optional_vec(stmt, &e) => {
                    println!("missing: sqlite-vec vec0; skipping native vector table");
                }
                Err(e) if idempotent && is_clean_idempotent_error(&e) => {}
                Err(e) => return Err(format!("{}: {e}", path.display())),
            }
        }
    }
    Ok(())
}

fn migration_files() -> Result<Vec<PathBuf>, String> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("migrations");
    let mut files = std::fs::read_dir(&dir)
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|s| s.to_str()) == Some("sql"))
        .collect::<Vec<_>>();
    files.sort_by_key(|path| {
        path.file_name()
            .and_then(|s| s.to_str())
            .and_then(|s| s.split_once("__"))
            .and_then(|(n, _)| n.parse::<u32>().ok())
            .unwrap_or(u32::MAX)
    });
    Ok(files)
}

fn expected_tables() -> &'static [&'static str] {
    &[
        "workspace",
        "session",
        "message",
        "tool_call",
        "memory_entry",
        "skill",
        "agent",
        "role",
        "role_binding",
        "routing_decision",
        "judge_record",
        "skill_lock",
        "memory_lock",
        "forge_run",
        "marketplace_artifact",
        "audit_record",
        "spend_ledger",
        "session_skill",
        "forge_queue",
        "session_grant",
        "persona",
        "worker",
        "duty",
        "worker_budget",
        "memory_entry_fts",
        "memory_embedding",
        "handoff",
    ]
}

async fn table_exists(store: &SessionStore, table: &str) -> Result<bool, sqlx::Error> {
    let row: Option<(String,)> =
        sqlx::query_as("SELECT name FROM sqlite_master WHERE type IN ('table','view') AND name = ?")
            .bind(table)
            .fetch_optional(store.pool())
            .await?;
    Ok(row.is_some())
}

async fn insert_worker_budget_if_present(store: &SessionStore) -> Result<(), sqlx::Error> {
    if !table_exists(store, "worker_budget").await? {
        println!("missing: worker_budget; skipping");
        return Ok(());
    }
    sqlx::query(
        "INSERT INTO worker_budget (tenant_id, worker_id, action_id, usd_micros, charged_at)
         VALUES (?, 'worker-a', 'action-a', 1, 1)",
    )
    .bind(uuid_blob())
    .execute(store.pool())
    .await?;
    Ok(())
}

async fn insert_orchestrator_session_if_present(store: &SessionStore) -> Result<(), sqlx::Error> {
    if !has_columns(store, "session", &["status", "participants", "created_at", "deadline"]).await?
    {
        println!("missing: orchestrator session columns; skipping");
        return Ok(());
    }
    let tenant = uuid_blob();
    let workspace = uuid_blob();
    let session = uuid_blob();
    sqlx::query(
        "INSERT INTO workspace (id, tenant_id, name, root_path, created_at)
         VALUES (?, ?, 'migration-chain', '.', 1)",
    )
    .bind(&workspace)
    .bind(&tenant)
    .execute(store.pool())
    .await?;
    sqlx::query(
        "INSERT INTO session (id, tenant_id, workspace_id, title, started_at, model, provider,
         status, participants, created_at, deadline)
         VALUES (?, ?, ?, 'handoff', 1, 'm', 'p', 'active', '[]', 1, NULL)",
    )
    .bind(&session)
    .bind(&tenant)
    .bind(&workspace)
    .execute(store.pool())
    .await?;
    Ok(())
}

async fn insert_handoff_if_present(store: &SessionStore) -> Result<(), sqlx::Error> {
    if !table_exists(store, "handoff").await? {
        println!("missing: handoff; skipping");
        return Ok(());
    }
    let session_id: Vec<u8> =
        sqlx::query("SELECT id FROM session WHERE title = 'handoff' LIMIT 1")
            .fetch_one(store.pool())
            .await?
            .try_get("id")?;
    sqlx::query(
        "INSERT INTO handoff (id, session_id, from_worker_id, \"to\", expected_outcome,
         payload, status, created_at, deadline)
         VALUES ('handoff-a', ?, 'worker-a', '{\"kind\":\"human\",\"addr\":\"ops@example.com\"}',
         'resolve', '{}', 'pending', 1, NULL)",
    )
    .bind(session_id)
    .execute(store.pool())
    .await?;
    Ok(())
}

async fn insert_shared_team_memory_if_present(store: &SessionStore) -> Result<(), sqlx::Error> {
    if !table_exists(store, "shared_team_memory").await? {
        println!("missing: shared_team_memory; skipping");
        return Ok(());
    }
    sqlx::query("INSERT INTO shared_team_memory DEFAULT VALUES")
        .execute(store.pool())
        .await?;
    Ok(())
}

async fn has_columns(
    store: &SessionStore,
    table: &str,
    columns: &[&str],
) -> Result<bool, sqlx::Error> {
    let rows = sqlx::query(&format!("PRAGMA table_info({table})"))
        .fetch_all(store.pool())
        .await?;
    Ok(columns.iter().all(|column| {
        rows.iter()
            .any(|row| row.try_get::<String, _>("name").ok().as_deref() == Some(*column))
    }))
}

fn uuid_blob() -> Vec<u8> {
    Uuid::now_v7().as_bytes().to_vec()
}

fn is_optional_vec(stmt: &str, err: &sqlx::Error) -> bool {
    stmt.contains("memory_vec_index") && err.to_string().contains("no such module: vec0")
}

fn is_clean_idempotent_error(err: &sqlx::Error) -> bool {
    let msg = err.to_string();
    msg.contains("duplicate column name") || msg.contains("already exists")
}
