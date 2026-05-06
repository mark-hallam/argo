//! Shared helpers for the `argo feedback ...` handlers.

use crate::home::{db_path, ensure_dir, resolve_home};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use std::sync::Arc;

const SCHEMA_SQL: &str =
    include_str!("../../../argo-store/migrations/24__operator_feedback.sql");

pub(super) async fn open_store() -> ArgoResult<Arc<SessionStore>> {
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Provider(format!("cwd: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(&db_path(&home)).await?;
    store.migrate().await?;
    Ok(Arc::new(store))
}

pub(super) async fn ensure_feedback_schema(store: &SessionStore) -> ArgoResult<()> {
    for stmt in SCHEMA_SQL.split_terminator(';') {
        let sql = stmt.trim();
        if sql.is_empty() {
            continue;
        }
        sqlx::query(sql)
            .execute(store.pool())
            .await
            .map_err(|e| ArgoError::Provider(format!("feedback schema: {e}")))?;
    }
    Ok(())
}
