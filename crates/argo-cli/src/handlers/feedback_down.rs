//! `argo feedback down <action-id>` handler.

use argo_store::feedback::{insert_feedback, Vote};
use argo_types::{ArgoError, ArgoResult};

use super::feedback_common::{ensure_feedback_schema, open_store};

pub async fn run(
    action_id: &str,
    comment: Option<&str>,
    worker_id: Option<&str>,
    json: bool,
) -> ArgoResult<()> {
    let store = open_store().await?;
    ensure_feedback_schema(&store).await?;
    let id = insert_feedback(
        store.pool(),
        action_id,
        Vote::Down,
        comment.unwrap_or(""),
        worker_id,
    )
    .await
    .map_err(|e| ArgoError::Provider(format!("insert feedback: {e}")))?;
    if json {
        println!(
            "{}",
            serde_json::to_string(&serde_json::json!({
                "ok": true,
                "id": id,
                "action_id": action_id,
                "vote": "down",
            }))
            .unwrap_or_default()
        );
    } else {
        println!("recorded down-vote on action '{action_id}' (id={id})");
    }
    Ok(())
}
