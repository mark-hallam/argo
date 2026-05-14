//! `argo subtask list` handler — query the persistent subtask
//! ledger with an optional filter set.
use argo_store::subtask::{list, SubtaskFilters, SubtaskState};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};

use crate::home::{db_path, ensure_dir, resolve_home};

const HARD_LIMIT_CAP: u32 = 500;

pub async fn run(
    session: Option<&str>,
    worker: Option<&str>,
    parent_goal: Option<&str>,
    state: Option<&str>,
    limit: u32,
    json: bool,
) -> ArgoResult<()> {
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Io(format!("cwd: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(&db_path(&home)).await?;
    store.migrate().await?;

    let parsed_state = match state {
        Some(label) => Some(parse_state(label)?),
        None => None,
    };
    let capped_limit = limit.clamp(1, HARD_LIMIT_CAP);
    let filters = SubtaskFilters {
        parent_session_id: session,
        worker_id: worker,
        parent_goal,
        state: parsed_state,
    };
    let rows = list(store.pool(), &filters, capped_limit).await?;

    if json {
        let envelope = serde_json::json!({
            "ok": true,
            "count": rows.len(),
            "filters": {
                "session": session,
                "worker": worker,
                "parent_goal": parent_goal,
                "state": state,
                "limit": capped_limit,
            },
            "rows": rows,
        });
        println!("{}", serde_json::to_string(&envelope).unwrap_or_default());
    } else if rows.is_empty() {
        println!("(no subtask rows match the filters)");
    } else {
        println!(
            "{:<14} {:<10} {:<24} {:<20} desc",
            "id-prefix", "state", "parent_goal", "plan_node_path"
        );
        for r in &rows {
            let id_prefix: String = r.id.chars().take(12).collect();
            let goal_prefix: String = r.parent_goal.chars().take(22).collect();
            let path_prefix: String = r.plan_node_path.chars().take(18).collect();
            println!(
                "{:<14} {:<10} {:<24} {:<20} {}",
                id_prefix,
                r.state.as_str(),
                goal_prefix,
                path_prefix,
                r.description
            );
        }
    }
    Ok(())
}

fn parse_state(label: &str) -> ArgoResult<SubtaskState> {
    match label {
        "pending" => Ok(SubtaskState::Pending),
        "running" => Ok(SubtaskState::Running),
        "done" => Ok(SubtaskState::Done),
        "failed" => Ok(SubtaskState::Failed),
        "skipped" => Ok(SubtaskState::Skipped),
        other => Err(ArgoError::Provider(format!(
            "unknown --state '{other}' (expected: pending, running, done, failed, skipped)"
        ))),
    }
}
