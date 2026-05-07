//! `argo plan execute` handler — runs a workflow YAML against
//! argo-workflow's mock executor (or dry-runs it). Real action
//! emit lives in a follow-up brief; v1's job is to prove the DAG
//! is shaped + dispatchable.

use argo_types::{ArgoError, ArgoResult};
use std::path::PathBuf;

pub async fn run(yaml_path: &str, dry_run: bool, json: bool) -> ArgoResult<()> {
    let path = PathBuf::from(yaml_path);
    let workflow = argo_workflow::parse_yaml(&path)
        .map_err(|e| ArgoError::Provider(format!("parse {yaml_path}: {e}")))?;

    if dry_run {
        let executor = argo_workflow::Executor::new(argo_workflow::MockRunner::echo());
        let plan = executor
            .dry_run(&workflow)
            .map_err(|e| ArgoError::Provider(format!("dry_run: {e}")))?;
        if json {
            let envelope = serde_json::json!({
                "ok": true,
                "dry_run": true,
                "tasks": plan.task_order,
            });
            println!("{}", serde_json::to_string(&envelope).unwrap_or_default());
        } else {
            println!("dry-run order:");
            for t in &plan.task_order {
                println!("  {t}");
            }
        }
        return Ok(());
    }

    let executor = argo_workflow::Executor::new(argo_workflow::MockRunner::echo());
    let request = argo_workflow::ExecutionRequest::new(workflow);
    let report = executor
        .execute(request)
        .await
        .map_err(|e| ArgoError::Provider(format!("execute: {e}")))?;
    if json {
        let verdicts: serde_json::Map<String, serde_json::Value> = report
            .task_verdicts
            .iter()
            .map(|(k, v)| (k.clone(), serde_json::json!(format!("{v:?}"))))
            .collect();
        let envelope = serde_json::json!({
            "ok": report.ok,
            "workflow_id": report.workflow_id.as_str(),
            "task_verdicts": verdicts,
        });
        println!("{}", serde_json::to_string(&envelope).unwrap_or_default());
    } else {
        println!(
            "workflow {} completed (ok={})",
            report.workflow_id.as_str(),
            report.ok
        );
        for (task_id, verdict) in &report.task_verdicts {
            println!("  {task_id}: {verdict:?}");
        }
    }
    Ok(())
}
