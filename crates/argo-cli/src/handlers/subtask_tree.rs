//! `argo subtask tree <session>` — render the subtask plan for one
//! session id, oldest insert first. The current view is a flat
//! ordered list (insert order); a future revision can fold rows
//! whose `plan_node_path` shares a common prefix into a real tree
//! once the Planner emits richer paths.
use argo_store::subtask::list_by_session;
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};

use crate::home::{db_path, ensure_dir, resolve_home};

pub async fn run(session: &str, json: bool) -> ArgoResult<()> {
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Io(format!("cwd: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(&db_path(&home)).await?;
    store.migrate().await?;

    let rows = list_by_session(store.pool(), session).await?;

    if json {
        let envelope = serde_json::json!({
            "ok": true,
            "session": session,
            "count": rows.len(),
            "rows": rows,
        });
        println!("{}", serde_json::to_string(&envelope).unwrap_or_default());
    } else if rows.is_empty() {
        println!("(no subtask rows for session '{session}')");
    } else {
        println!("subtask plan for session {session} ({} rows)", rows.len());
        for r in &rows {
            println!(
                "  [{}] {} :: {} — {}",
                r.state.as_str(),
                r.plan_node_path,
                r.description,
                r.worker_id
            );
        }
    }
    Ok(())
}
