//! `argo workflow resume <run-id>` (Stream
//! `claude-checkpoint-resume`). Loads the prior workflow_run +
//! its checkpoints, then re-runs only the still-pending tasks
//! via [`Executor::resume`].

use argo_types::{ArgoError, ArgoResult};
use argo_workflow::{
    parse_yaml_str, ExecutionRequest, Executor, MockRunner, TaskVerdict, WorkflowId,
};
use chrono::Utc;
use std::sync::Arc;

use super::workflow::{ensure_workflow_schema, open_store};
use super::workflow_checkpoint::{ensure_schema, SqliteCheckpointStore};

pub async fn run(run_id: &str, json: bool) -> ArgoResult<()> {
    let store = open_store().await?;
    ensure_workflow_schema(&store).await?;
    ensure_schema(&store)
        .await
        .map_err(|e| ArgoError::Provider(format!("checkpoint schema: {e}")))?;

    let row: Option<(String, String)> = sqlx::query_as(
        "SELECT workflow_id, workflow_yaml FROM workflow_run WHERE id = ?",
    )
    .bind(run_id)
    .fetch_optional(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("workflow_run lookup: {e}")))?;
    let (workflow_id_label, yaml_body) =
        row.ok_or_else(|| ArgoError::Provider(format!("no run with id '{run_id}'")))?;

    let workflow = parse_yaml_str(&yaml_body)
        .map_err(|e| ArgoError::Provider(format!("re-parse run yaml: {e}")))?;
    let ckpt_store = SqliteCheckpointStore::new(Arc::clone(&store));
    let existing = argo_workflow::CheckpointStore::list_for_run(
        &ckpt_store,
        &WorkflowId(run_id.to_string()),
    )
    .await
    .map_err(|e| ArgoError::Provider(format!("list checkpoints: {e}")))?;

    let exec = Executor::new(MockRunner::echo());
    let req = ExecutionRequest::with_id(workflow.clone(), WorkflowId(run_id.to_string()));
    let report = exec
        .resume(req, &ckpt_store, &existing)
        .await
        .map_err(|e| ArgoError::Provider(format!("resume: {e}")))?;

    let finished_at = Utc::now().timestamp_micros();
    sqlx::query("UPDATE workflow_run SET finished_at = ?, ok = ? WHERE id = ?")
        .bind(finished_at)
        .bind(if report.ok { 1_i64 } else { 0 })
        .bind(run_id)
        .execute(store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("workflow_run update: {e}")))?;

    if json {
        println!("{}", serde_json::to_string(&report).unwrap_or_default());
    } else {
        let marker = if report.ok { "OK" } else { "FAILED" };
        println!(
            "workflow '{workflow_id_label}' run {run_id} {marker} ({} tasks)",
            report.task_verdicts.len()
        );
        for (id, verdict) in &report.task_verdicts {
            let label = match verdict {
                TaskVerdict::Success { .. } => "PASS",
                TaskVerdict::Failed { .. } => "FAIL",
                TaskVerdict::Skipped { .. } => "SKIP",
            };
            println!("  [{label}] {id}");
        }
    }
    if !report.ok {
        return Err(ArgoError::Schema(format!(
            "workflow run '{run_id}' had failed task(s)"
        )));
    }
    Ok(())
}
