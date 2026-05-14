//! `argo tui` — open the interactive TUI bound to the workspace store.
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_store::SessionStore;
use argo_tui::run_session_viewer;
use argo_types::{ArgoError, ArgoResult};
use std::sync::Arc;

pub async fn run() -> ArgoResult<()> {
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Io(format!("current_dir: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let db = db_path(&home);
    let store = SessionStore::open(&db).await?;
    store.migrate().await?;
    run_session_viewer(Arc::new(store), cwd).await
}
