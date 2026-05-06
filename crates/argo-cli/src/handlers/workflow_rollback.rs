//! `argo workflow rollback <run-id> --to-checkpoint <seq>`
//! (Stream `claude-checkpoint-resume`). Drops every checkpoint
//! row strictly above the target `seq` for one run.

use argo_store::workflow_checkpoint::rollback_to;
use argo_types::{ArgoError, ArgoResult};

use super::workflow::{ensure_workflow_schema, open_store};
use super::workflow_checkpoint::ensure_schema;

pub async fn run(run_id: &str, to_checkpoint: i64, json: bool) -> ArgoResult<()> {
    let store = open_store().await?;
    ensure_workflow_schema(&store).await?;
    ensure_schema(&store)
        .await
        .map_err(|e| ArgoError::Provider(format!("checkpoint schema: {e}")))?;
    let ok = rollback_to(store.pool(), run_id, to_checkpoint)
        .await
        .map_err(|e| ArgoError::Provider(format!("rollback: {e}")))?;
    if !ok {
        return Err(ArgoError::Provider(format!(
            "no checkpoint with seq {to_checkpoint} on run '{run_id}'"
        )));
    }
    if json {
        println!(
            "{}",
            serde_json::to_string(&serde_json::json!({
                "ok": true,
                "run_id": run_id,
                "rolled_back_to": to_checkpoint,
            }))
            .unwrap_or_default()
        );
    } else {
        println!("rolled run '{run_id}' back to checkpoint seq {to_checkpoint}");
    }
    Ok(())
}
