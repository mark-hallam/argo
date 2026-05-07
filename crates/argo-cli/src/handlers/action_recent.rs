//! `argo action recent`.

use super::action_history::{collect_action_views, filter_recent, print_json, ActionView};
use argo_types::ArgoResult;
use serde::Serialize;

#[derive(Debug, Serialize)]
struct RecentOut {
    tenant: Option<String>,
    worker: Option<String>,
    limit: usize,
    rows: Vec<ActionView>,
}

pub(crate) async fn run(
    tenant: Option<&str>,
    worker: Option<&str>,
    limit: usize,
    json: bool,
) -> ArgoResult<()> {
    let rows = filter_recent(collect_action_views(true).await?, tenant, worker, limit);
    if json {
        let out = RecentOut {
            tenant: tenant.map(str::to_string),
            worker: worker.map(str::to_string),
            limit,
            rows,
        };
        print_json("action_recent", out)
    } else {
        render_recent_text(&rows);
        Ok(())
    }
}

fn render_recent_text(rows: &[ActionView]) {
    if rows.is_empty() {
        println!("(no actions)");
        return;
    }
    println!("created_at                 tenant       worker        action_id       state       type");
    for row in rows {
        println!(
            "{:<26} {:<12} {:<13} {:<15} {:<11} {}",
            row.created_at,
            row.tenant_id.as_deref().unwrap_or("-"),
            row.worker_id,
            short(&row.action_id),
            row.state,
            row.action_type
        );
    }
}

fn short(id: &str) -> String {
    if id.len() > 14 {
        format!("{}...", &id[..11])
    } else {
        id.to_string()
    }
}
