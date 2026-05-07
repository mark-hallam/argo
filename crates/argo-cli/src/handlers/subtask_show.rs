//! `argo subtask show <id>` — fetch one row by id.
use argo_store::subtask::get;
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};

use crate::home::{db_path, ensure_dir, resolve_home};

pub async fn run(id: &str, json: bool) -> ArgoResult<()> {
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Provider(format!("cwd: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(&db_path(&home)).await?;
    store.migrate().await?;

    let row = get(store.pool(), id)
        .await?
        .ok_or_else(|| ArgoError::Provider(format!("no subtask row with id '{id}'")))?;

    if json {
        let envelope = serde_json::json!({
            "ok": true,
            "row": row,
        });
        println!("{}", serde_json::to_string(&envelope).unwrap_or_default());
    } else {
        println!("subtask {id}");
        println!("  state          : {}", row.state.as_str());
        println!("  worker         : {}", row.worker_id);
        println!(
            "  parent_session : {}",
            row.parent_session_id.as_deref().unwrap_or("(none)")
        );
        println!("  parent_goal    : {}", row.parent_goal);
        println!("  plan_node_path : {}", row.plan_node_path);
        println!("  description    : {}", row.description);
        if let Some(o) = &row.output {
            println!("  output         : {o}");
        }
        if let Some(s) = row.started_at {
            println!("  started_at     : {s}");
        }
        if let Some(c) = row.completed_at {
            println!("  completed_at   : {c}");
        }
        println!("  created_at     : {}", row.created_at);
    }
    Ok(())
}
