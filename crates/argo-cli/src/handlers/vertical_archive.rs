//! Vertical archive support for `argo vertical`.

use crate::home::{db_path, ensure_dir, resolve_home};
use argo_identity::IdentityStore;
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use argo_vertical::LoadedVertical;
use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::json;
use sqlx::Row;
use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Debug, Serialize)]
pub(crate) struct VerticalLifecycleOut {
    id: String,
    archived_at: Option<i64>,
    archived_at_rfc3339: Option<String>,
}

pub async fn run(id: &str, json_out: bool) -> ArgoResult<()> {
    let id = normalize_id(id)?;
    load_installed(id)?;
    let (store, _) = open_store().await?;
    let archived_at = archive_vertical(&store, id, Utc::now().timestamp_micros()).await?;
    let out = VerticalLifecycleOut::archived(id, archived_at);
    if json_out {
        print_json(&json!({"ok": true, "vertical_archive": out}))
    } else {
        println!("archived vertical {id}");
        Ok(())
    }
}

pub(crate) async fn open_store() -> ArgoResult<(SessionStore, PathBuf)> {
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Io(format!("cwd: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(db_path(&home)).await?;
    store.migrate().await?;
    IdentityStore::new(&store).ensure_schema().await?;
    ensure_vertical_lifecycle_schema(&store).await?;
    Ok((store, cwd))
}

pub(crate) async fn archived_map_for_default_store() -> ArgoResult<BTreeMap<String, i64>> {
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Io(format!("cwd: {e}")))?;
    let home = resolve_home(&cwd)?;
    let path = db_path(&home);
    if !path.is_file() {
        return Ok(BTreeMap::new());
    }
    let store = SessionStore::open(path).await?;
    store.migrate().await?;
    ensure_vertical_lifecycle_table(&store).await?;
    archived_map(&store).await
}

pub(crate) async fn ensure_vertical_lifecycle_schema(store: &SessionStore) -> ArgoResult<()> {
    ensure_vertical_lifecycle_table(store).await?;
    for sql in [
        REJECT_ARCHIVED_SESSION_INSERT,
        REJECT_ARCHIVED_SESSION_UPDATE,
        REJECT_ARCHIVED_WORKER_INSERT,
        REJECT_ARCHIVED_WORKER_UPDATE,
    ] {
        sqlx::query(sql)
            .execute(store.pool())
            .await
            .map_err(|e| ArgoError::Provider(format!("vertical lifecycle trigger: {e}")))?;
    }
    Ok(())
}

async fn ensure_vertical_lifecycle_table(store: &SessionStore) -> ArgoResult<()> {
    for sql in [CREATE_VERTICAL_LIFECYCLE, CREATE_ARCHIVED_INDEX] {
        sqlx::query(sql)
            .execute(store.pool())
            .await
            .map_err(|e| ArgoError::Provider(format!("vertical lifecycle schema: {e}")))?;
    }
    Ok(())
}

const CREATE_VERTICAL_LIFECYCLE: &str = "CREATE TABLE IF NOT EXISTS vertical_lifecycle (
    vertical_id TEXT PRIMARY KEY,
    archived_at INTEGER
)";

const CREATE_ARCHIVED_INDEX: &str = "CREATE INDEX IF NOT EXISTS idx_vertical_lifecycle_archived
ON vertical_lifecycle(archived_at)";

const REJECT_ARCHIVED_SESSION_INSERT: &str =
    "CREATE TRIGGER IF NOT EXISTS session_reject_archived_vertical_insert
BEFORE INSERT ON session
FOR EACH ROW
WHEN NEW.vertical_id IS NOT NULL
 AND EXISTS (
    SELECT 1 FROM vertical_lifecycle
    WHERE vertical_id = NEW.vertical_id
      AND archived_at IS NOT NULL
 )
BEGIN
    SELECT RAISE(ABORT, 'vertical is archived');
END";

const REJECT_ARCHIVED_SESSION_UPDATE: &str =
    "CREATE TRIGGER IF NOT EXISTS session_reject_archived_vertical_update
BEFORE UPDATE OF vertical_id ON session
FOR EACH ROW
WHEN NEW.vertical_id IS NOT NULL
 AND EXISTS (
    SELECT 1 FROM vertical_lifecycle
    WHERE vertical_id = NEW.vertical_id
      AND archived_at IS NOT NULL
 )
BEGIN
    SELECT RAISE(ABORT, 'vertical is archived');
END";

const REJECT_ARCHIVED_WORKER_INSERT: &str =
    "CREATE TRIGGER IF NOT EXISTS worker_reject_archived_vertical_insert
BEFORE INSERT ON worker
FOR EACH ROW
WHEN EXISTS (
    SELECT 1 FROM vertical_lifecycle
    WHERE vertical_id = NEW.vertical_id
      AND archived_at IS NOT NULL
)
BEGIN
    SELECT RAISE(ABORT, 'vertical is archived');
END";

const REJECT_ARCHIVED_WORKER_UPDATE: &str =
    "CREATE TRIGGER IF NOT EXISTS worker_reject_archived_vertical_update
BEFORE UPDATE OF vertical_id ON worker
FOR EACH ROW
WHEN EXISTS (
    SELECT 1 FROM vertical_lifecycle
    WHERE vertical_id = NEW.vertical_id
      AND archived_at IS NOT NULL
)
BEGIN
    SELECT RAISE(ABORT, 'vertical is archived');
END";

pub(crate) fn load_installed(id: &str) -> ArgoResult<LoadedVertical> {
    let dir = super::vertical::resolve_verticals_dir();
    let (registry, _) = argo_vertical::load_dir(&dir)?;
    registry.get(id).cloned().ok_or_else(|| {
        ArgoError::Provider(format!("vertical '{id}' not found in {}", dir.display()))
    })
}

pub(crate) async fn archive_vertical(
    store: &SessionStore,
    id: &str,
    archived_at: i64,
) -> ArgoResult<i64> {
    sqlx::query(
        "INSERT INTO vertical_lifecycle (vertical_id, archived_at)
         VALUES (?, ?)
         ON CONFLICT(vertical_id) DO UPDATE
         SET archived_at = COALESCE(vertical_lifecycle.archived_at, excluded.archived_at)",
    )
    .bind(id)
    .bind(archived_at)
    .execute(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("vertical archive: {e}")))?;
    Ok(vertical_archived_at(store, id)
        .await?
        .unwrap_or(archived_at))
}

pub(crate) async fn unarchive_vertical(
    store: &SessionStore,
    id: &str,
) -> ArgoResult<VerticalLifecycleOut> {
    sqlx::query(
        "INSERT INTO vertical_lifecycle (vertical_id, archived_at)
         VALUES (?, NULL)
         ON CONFLICT(vertical_id) DO UPDATE SET archived_at = NULL",
    )
    .bind(id)
    .execute(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("vertical unarchive: {e}")))?;
    Ok(VerticalLifecycleOut::active(id))
}

pub(crate) async fn vertical_archived_at(
    store: &SessionStore,
    id: &str,
) -> ArgoResult<Option<i64>> {
    let row = sqlx::query("SELECT archived_at FROM vertical_lifecycle WHERE vertical_id = ?")
        .bind(id)
        .fetch_optional(store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("vertical archived_at lookup: {e}")))?;
    Ok(row.and_then(|row| row.get("archived_at")))
}

async fn archived_map(store: &SessionStore) -> ArgoResult<BTreeMap<String, i64>> {
    let rows = sqlx::query(
        "SELECT vertical_id, archived_at FROM vertical_lifecycle WHERE archived_at IS NOT NULL",
    )
    .fetch_all(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("vertical archived list: {e}")))?;
    let mut out = BTreeMap::new();
    for row in rows {
        out.insert(row.get("vertical_id"), row.get("archived_at"));
    }
    Ok(out)
}

pub(crate) fn normalize_id(raw: &str) -> ArgoResult<&str> {
    let id = raw.trim();
    if id.is_empty() {
        return Err(ArgoError::Provider("vertical id is empty".into()));
    }
    Ok(id)
}

pub(crate) fn print_json<T: Serialize>(value: &T) -> ArgoResult<()> {
    let body = serde_json::to_string_pretty(value)
        .map_err(|e| ArgoError::Provider(format!("serialize vertical JSON: {e}")))?;
    println!("{body}");
    Ok(())
}

pub(crate) fn format_micros(micros: i64) -> Option<String> {
    DateTime::<Utc>::from_timestamp_micros(micros).map(|ts| ts.to_rfc3339())
}

impl VerticalLifecycleOut {
    fn archived(id: &str, archived_at: i64) -> Self {
        Self {
            id: id.to_string(),
            archived_at: Some(archived_at),
            archived_at_rfc3339: format_micros(archived_at),
        }
    }

    fn active(id: &str) -> Self {
        Self {
            id: id.to_string(),
            archived_at: None,
            archived_at_rfc3339: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use argo_types::{TenantId, WorkspaceId};

    async fn store() -> SessionStore {
        let store = SessionStore::open(":memory:").await.unwrap();
        store.migrate().await.unwrap();
        IdentityStore::new(&store).ensure_schema().await.unwrap();
        ensure_vertical_lifecycle_schema(&store).await.unwrap();
        store
    }

    async fn insert_workspace(store: &SessionStore, tenant: TenantId, workspace: WorkspaceId) {
        sqlx::query(
            "INSERT INTO workspace (id, tenant_id, name, root_path, created_at)
             VALUES (?, ?, 'demo', '.', 1)",
        )
        .bind(workspace.0.as_bytes().as_slice())
        .bind(tenant.0.as_bytes().as_slice())
        .execute(store.pool())
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn schema_adds_lifecycle_table() {
        let store = store().await;
        let exists: Option<String> = sqlx::query_scalar(
            "SELECT name FROM sqlite_master WHERE type = 'table' AND name = 'vertical_lifecycle'",
        )
        .fetch_optional(store.pool())
        .await
        .unwrap();
        assert_eq!(exists.as_deref(), Some("vertical_lifecycle"));
    }

    #[tokio::test]
    async fn archive_marks_vertical() {
        let store = store().await;
        let archived_at = archive_vertical(&store, "demo", 42).await.unwrap();
        assert_eq!(archived_at, 42);
        assert_eq!(
            vertical_archived_at(&store, "demo").await.unwrap(),
            Some(42)
        );
    }

    #[tokio::test]
    async fn archive_is_idempotent() {
        let store = store().await;
        archive_vertical(&store, "demo", 42).await.unwrap();
        let archived_at = archive_vertical(&store, "demo", 99).await.unwrap();
        assert_eq!(archived_at, 42);
    }

    #[tokio::test]
    async fn unarchive_clears_archived_marker() {
        let store = store().await;
        archive_vertical(&store, "demo", 42).await.unwrap();
        unarchive_vertical(&store, "demo").await.unwrap();
        assert_eq!(vertical_archived_at(&store, "demo").await.unwrap(), None);
    }

    #[tokio::test]
    async fn archived_map_only_returns_archived_rows() {
        let store = store().await;
        archive_vertical(&store, "alpha", 42).await.unwrap();
        unarchive_vertical(&store, "beta").await.unwrap();
        let rows = archived_map(&store).await.unwrap();
        assert_eq!(rows.get("alpha"), Some(&42));
        assert!(!rows.contains_key("beta"));
    }

    #[tokio::test]
    async fn archive_blocks_new_bindings_but_keeps_existing_rows() {
        let store = store().await;
        let tenant = TenantId::new();
        let workspace = WorkspaceId::new();
        insert_workspace(&store, tenant, workspace).await;
        store
            .create_session(tenant, workspace, "mock", "mock", Some("old"), Some("demo"))
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO worker
             (id, tenant_id, vertical_id, persona_id, role_id, memory_scope,
              auto_approval_json, channels_json, created_at)
             VALUES ('old-worker', ?, 'demo', 'p', 'r', 'worker_self', '{}', '[]', 1)",
        )
        .bind(tenant.to_string())
        .execute(store.pool())
        .await
        .unwrap();

        archive_vertical(&store, "demo", 42).await.unwrap();

        let existing_sessions: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM session WHERE vertical_id = 'demo'")
                .fetch_one(store.pool())
                .await
                .unwrap();
        assert_eq!(existing_sessions, 1);
        let existing_workers: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM worker WHERE vertical_id = 'demo'")
                .fetch_one(store.pool())
                .await
                .unwrap();
        assert_eq!(existing_workers, 1);

        let session_err = store
            .create_session(tenant, workspace, "mock", "mock", Some("new"), Some("demo"))
            .await
            .unwrap_err();
        assert!(session_err.to_string().contains("vertical is archived"));

        let worker_err = sqlx::query(
            "INSERT INTO worker
             (id, tenant_id, vertical_id, persona_id, role_id, memory_scope,
              auto_approval_json, channels_json, created_at)
             VALUES ('new-worker', ?, 'demo', 'p', 'r', 'worker_self', '{}', '[]', 1)",
        )
        .bind(tenant.to_string())
        .execute(store.pool())
        .await
        .unwrap_err();
        assert!(worker_err.to_string().contains("vertical is archived"));
    }
}
