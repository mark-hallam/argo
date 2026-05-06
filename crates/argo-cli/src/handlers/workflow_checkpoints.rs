//! `argo workflow checkpoints <run-id>` (Stream
//! `claude-checkpoint-resume`). Lists every checkpoint row for
//! one workflow run in seq order.

use argo_store::workflow_checkpoint::{list_for_run, micros_to_rfc3339};
use argo_types::{ArgoError, ArgoResult};

use super::workflow::{ensure_workflow_schema, open_store};
use super::workflow_checkpoint::ensure_schema;

pub async fn run(run_id: &str, json: bool) -> ArgoResult<()> {
    let store = open_store().await?;
    ensure_workflow_schema(&store).await?;
    ensure_schema(&store)
        .await
        .map_err(|e| ArgoError::Provider(format!("checkpoint schema: {e}")))?;
    let rows = list_for_run(store.pool(), run_id)
        .await
        .map_err(|e| ArgoError::Provider(format!("list checkpoints: {e}")))?;
    if json {
        let arr: Vec<serde_json::Value> = rows
            .iter()
            .map(|r| {
                serde_json::json!({
                    "seq": r.seq,
                    "task_id": r.task_id,
                    "completed_at_micros": r.completed_at_micros,
                    "completed_at": micros_to_rfc3339(r.completed_at_micros),
                    "cache_key": r.cache_key,
                    "outputs_json": r.outputs_json,
                })
            })
            .collect();
        println!("{}", serde_json::to_string(&arr).unwrap_or_default());
    } else if rows.is_empty() {
        println!("(no checkpoints for run '{run_id}')");
    } else {
        println!("{:<6} {:<24} {:<28} cache_key", "seq", "task_id", "completed_at");
        for r in &rows {
            let key = r.cache_key.as_deref().unwrap_or("-");
            println!(
                "{:<6} {:<24} {:<28} {}",
                r.seq,
                r.task_id,
                micros_to_rfc3339(r.completed_at_micros),
                key,
            );
        }
    }
    Ok(())
}
