//! `argo feedback up <action-id>` handler.

use argo_store::feedback::{insert_feedback, Vote};
use argo_store::skill_rating::SkillVote;
use argo_types::{ArgoError, ArgoResult};

use super::feedback_common::{ensure_feedback_schema, maybe_record_skill_rating, open_store};

pub async fn run(
    action_id: &str,
    comment: Option<&str>,
    worker_id: Option<&str>,
    skill: Option<&str>,
    json: bool,
) -> ArgoResult<()> {
    let store = open_store().await?;
    ensure_feedback_schema(&store).await?;
    let id = insert_feedback(
        store.pool(),
        action_id,
        Vote::Up,
        comment.unwrap_or(""),
        worker_id,
    )
    .await
    .map_err(|e| ArgoError::Provider(format!("insert feedback: {e}")))?;
    let skill_rating_recorded = if let Some(name) = skill {
        maybe_record_skill_rating(&store, name, SkillVote::Up, comment).await?
    } else {
        false
    };
    if json {
        println!(
            "{}",
            serde_json::to_string(&serde_json::json!({
                "ok": true,
                "id": id,
                "action_id": action_id,
                "vote": "up",
                "skill_rating_recorded": skill_rating_recorded,
            }))
            .unwrap_or_default()
        );
    } else {
        println!("recorded up-vote on action '{action_id}' (id={id})");
        if skill_rating_recorded {
            println!("  +propagated to publisher skill_rating");
        }
    }
    Ok(())
}
