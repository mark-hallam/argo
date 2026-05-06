//! `argo feedback list` handler — print stored feedback rows.

use argo_store::feedback::{list_feedback, micros_to_rfc3339, Vote};
use argo_types::{ArgoError, ArgoResult};

use super::feedback_common::{ensure_feedback_schema, open_store};

pub async fn run(
    worker_id: Option<&str>,
    vote: Option<&str>,
    limit: u32,
    json: bool,
) -> ArgoResult<()> {
    let store = open_store().await?;
    ensure_feedback_schema(&store).await?;
    let parsed_vote = match vote {
        None => None,
        Some(v) => match v {
            "up" => Some(Vote::Up),
            "down" => Some(Vote::Down),
            other => {
                return Err(ArgoError::Provider(format!(
                    "--vote must be 'up' or 'down' (got '{other}')"
                )))
            }
        },
    };
    let rows = list_feedback(store.pool(), worker_id, parsed_vote, limit)
        .await
        .map_err(|e| ArgoError::Provider(format!("list feedback: {e}")))?;
    if json {
        let arr: Vec<serde_json::Value> = rows
            .iter()
            .map(|r| {
                serde_json::json!({
                    "id": r.id,
                    "action_id": r.action_id,
                    "vote": r.vote.as_str(),
                    "comment": r.comment,
                    "given_at_micros": r.given_at_micros,
                    "given_at": micros_to_rfc3339(r.given_at_micros),
                    "worker_id": r.worker_id,
                })
            })
            .collect();
        println!("{}", serde_json::to_string(&arr).unwrap_or_default());
    } else if rows.is_empty() {
        println!("(no feedback rows)");
    } else {
        println!(
            "{:<6} {:<6} {:<24} {:<28} {:<16} comment",
            "id", "vote", "action_id", "given_at", "worker"
        );
        for r in &rows {
            let worker = r.worker_id.as_deref().unwrap_or("-");
            let comment = if r.comment.is_empty() { "-" } else { r.comment.as_str() };
            println!(
                "{:<6} {:<6} {:<24} {:<28} {:<16} {}",
                r.id,
                r.vote.as_str(),
                r.action_id,
                micros_to_rfc3339(r.given_at_micros),
                worker,
                comment,
            );
        }
    }
    Ok(())
}
