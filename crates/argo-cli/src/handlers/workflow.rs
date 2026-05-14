//! `argo workflow validate/run/status/list-runs` handler
//! (Stream `claude-workflow-dag`).
//!
//! v1 wiring uses the `MockRunner` so workflows are end-to-end
//! testable without the production worker dispatch path; a
//! follow-up brief swaps in a `WorkerSessionRunner`. The
//! workflow_run table holds run history regardless of runner.
use crate::cli::WorkflowCmd;
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use argo_workflow::{
    parse_yaml, ExecutionRequest, ExecutionReport, Executor, MockRunner, TaskVerdict,
};
use chrono::Utc;
use std::path::PathBuf;
use std::sync::Arc;

const SCHEMA_SQL: &str = include_str!("../../../argo-store/migrations/18__workflow_runs.sql");

/// `(workflow_id, workflow_yaml, started_at, finished_at, ok, task_verdicts_json)` —
/// row shape returned by `argo workflow status`.
type StatusRow = (String, String, i64, Option<i64>, Option<i64>, Option<String>);

/// `(id, workflow_id, started_at, finished_at, ok)` — row shape
/// returned by `argo workflow list-runs`.
type RunRow = (String, String, i64, Option<i64>, Option<i64>);

pub async fn run(cmd: WorkflowCmd) -> ArgoResult<()> {
    match cmd {
        WorkflowCmd::Validate { path, json } => validate(&PathBuf::from(path), json).await,
        WorkflowCmd::Run { path, dry_run, json } => {
            run_workflow(&PathBuf::from(path), dry_run, json).await
        }
        WorkflowCmd::Status { run_id, json } => status(&run_id, json).await,
        WorkflowCmd::ListRuns { limit, json } => list_runs(limit, json).await,
        WorkflowCmd::Resume { run_id, json } => super::workflow_resume::run(&run_id, json).await,
        WorkflowCmd::Checkpoints { run_id, json } => {
            super::workflow_checkpoints::run(&run_id, json).await
        }
        WorkflowCmd::Rollback {
            run_id,
            to_checkpoint,
            json,
        } => super::workflow_rollback::run(&run_id, to_checkpoint, json).await,
    }
}

async fn validate(path: &std::path::Path, json: bool) -> ArgoResult<()> {
    let workflow = parse_yaml(path).map_err(|e| ArgoError::Provider(format!("{e}")))?;
    let order = workflow
        .topo_order()
        .map_err(|e| ArgoError::Schema(format!("validate: {e}")))?;
    let task_order: Vec<String> = order.iter().map(|t| t.id.clone()).collect();
    if json {
        println!(
            "{}",
            serde_json::to_string(&serde_json::json!({
                "ok": true,
                "id": workflow.id,
                "task_count": workflow.tasks.len(),
                "task_order": task_order,
            }))
            .unwrap_or_default()
        );
    } else {
        println!(
            "workflow '{}': OK ({} tasks)",
            workflow.id,
            workflow.tasks.len()
        );
        for (i, id) in task_order.iter().enumerate() {
            println!("  {:>2}. {id}", i + 1);
        }
    }
    Ok(())
}

async fn run_workflow(path: &std::path::Path, dry_run: bool, json: bool) -> ArgoResult<()> {
    let workflow = parse_yaml(path).map_err(|e| ArgoError::Provider(format!("{e}")))?;
    let exec = Executor::new(MockRunner::echo());
    if dry_run {
        let plan = exec.dry_run(&workflow).map_err(|e| {
            ArgoError::Schema(format!("dry-run: {e}"))
        })?;
        if json {
            println!("{}", serde_json::to_string(&plan).unwrap_or_default());
        } else {
            println!(
                "workflow '{}': dry-run plan ({} tasks)",
                workflow.id,
                plan.task_order.len()
            );
            for (i, id) in plan.task_order.iter().enumerate() {
                println!("  {:>2}. {id}", i + 1);
            }
        }
        return Ok(());
    }
    let yaml_body = std::fs::read_to_string(path)
        .map_err(|e| ArgoError::Io(format!("read {}: {e}", path.display())))?;
    let store = open_store().await?;
    ensure_workflow_schema(&store).await?;
    super::workflow_checkpoint::ensure_schema(&store)
        .await
        .map_err(|e| ArgoError::Provider(format!("checkpoint schema: {e}")))?;
    let ckpt_store = super::workflow_checkpoint::SqliteCheckpointStore::new(Arc::clone(&store));
    let request = ExecutionRequest::new(workflow.clone());
    let started_at = Utc::now();
    let report = exec
        .execute_with_checkpoints(request, &ckpt_store)
        .await
        .map_err(|e| ArgoError::Provider(format!("execute: {e}")))?;
    let finished_at = Utc::now();
    insert_run(
        &store,
        &report,
        &workflow.id,
        &yaml_body,
        started_at,
        finished_at,
    )
    .await?;
    if json {
        println!("{}", serde_json::to_string(&report).unwrap_or_default());
    } else {
        let marker = if report.ok { "OK" } else { "FAILED" };
        println!(
            "workflow '{}' run {} {marker} ({} tasks)",
            workflow.id,
            report.workflow_id.as_str(),
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
            "workflow '{}' had failed task(s)",
            workflow.id
        )));
    }
    Ok(())
}

async fn status(run_id: &str, json: bool) -> ArgoResult<()> {
    let store = open_store().await?;
    ensure_workflow_schema(&store).await?;
    let row: Option<StatusRow> = sqlx::query_as(
            "SELECT workflow_id, workflow_yaml, started_at, finished_at, ok, task_verdicts_json \
             FROM workflow_run WHERE id = ?",
        )
        .bind(run_id)
        .fetch_optional(store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("workflow_run lookup: {e}")))?;
    let row = row.ok_or_else(|| ArgoError::Provider(format!("no run with id '{run_id}'")))?;
    let payload = serde_json::json!({
        "id": run_id,
        "workflow_id": row.0,
        "started_at_micros": row.2,
        "finished_at_micros": row.3,
        "ok": row.4.map(|n| n != 0),
        "task_verdicts": row.5.as_deref().and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok()),
    });
    if json {
        println!("{}", serde_json::to_string(&payload).unwrap_or_default());
    } else {
        println!("run {run_id}");
        println!("  workflow:    {}", row.0);
        println!("  started_at:  {}", row.2);
        println!("  finished_at: {}", row.3.map_or("(running)".to_string(), |n| n.to_string()));
        println!("  ok:          {}", row.4.map_or("?".to_string(), |n| (n != 0).to_string()));
    }
    Ok(())
}

async fn list_runs(limit: u32, json: bool) -> ArgoResult<()> {
    let store = open_store().await?;
    ensure_workflow_schema(&store).await?;
    let lim = limit.min(1000) as i64;
    let rows: Vec<RunRow> = sqlx::query_as(
        "SELECT id, workflow_id, started_at, finished_at, ok \
         FROM workflow_run ORDER BY started_at DESC LIMIT ?",
    )
    .bind(lim)
    .fetch_all(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("workflow_run list: {e}")))?;
    if json {
        let arr: Vec<serde_json::Value> = rows
            .iter()
            .map(|(id, wf, s, f, ok)| {
                serde_json::json!({
                    "id": id,
                    "workflow_id": wf,
                    "started_at_micros": s,
                    "finished_at_micros": f,
                    "ok": ok.map(|n| n != 0),
                })
            })
            .collect();
        println!("{}", serde_json::to_string(&arr).unwrap_or_default());
    } else if rows.is_empty() {
        println!("(no workflow runs)");
    } else {
        println!("{:<40} {:<24} {:<12} started_at", "id", "workflow_id", "ok");
        for (id, wf, s, _f, ok) in &rows {
            let ok_label = ok.map_or("?".to_string(), |n| (n != 0).to_string());
            println!("{id:<40} {wf:<24} {ok_label:<12} {s}");
        }
    }
    Ok(())
}

pub(super) async fn open_store() -> ArgoResult<Arc<SessionStore>> {
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Io(format!("cwd: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(&db_path(&home)).await?;
    store.migrate().await?;
    Ok(Arc::new(store))
}

pub(super) async fn ensure_workflow_schema(store: &SessionStore) -> ArgoResult<()> {
    for stmt in SCHEMA_SQL.split_terminator(';') {
        let sql = stmt.trim();
        if sql.is_empty() {
            continue;
        }
        sqlx::query(sql)
            .execute(store.pool())
            .await
            .map_err(|e| ArgoError::Provider(format!("workflow schema: {e}")))?;
    }
    Ok(())
}

async fn insert_run(
    store: &SessionStore,
    report: &ExecutionReport,
    workflow_id: &str,
    yaml_body: &str,
    started_at: chrono::DateTime<Utc>,
    finished_at: chrono::DateTime<Utc>,
) -> ArgoResult<()> {
    let verdicts_json = serde_json::to_string(&report.task_verdicts)
        .map_err(|e| ArgoError::Provider(format!("verdicts json: {e}")))?;
    sqlx::query(
        "INSERT INTO workflow_run \
         (id, workflow_id, workflow_yaml, started_at, finished_at, ok, task_verdicts_json) \
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(report.workflow_id.as_str())
    .bind(workflow_id)
    .bind(yaml_body)
    .bind(started_at.timestamp_micros())
    .bind(finished_at.timestamp_micros())
    .bind(if report.ok { 1_i64 } else { 0 })
    .bind(verdicts_json)
    .execute(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("workflow_run insert: {e}")))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn validate_emits_topological_order_for_a_diamond() {
        let yaml = r#"
id: t-diamond
tasks:
  - id: a
    worker: w
  - id: b
    worker: w
    depends_on: [a]
  - id: c
    worker: w
    depends_on: [a]
  - id: d
    worker: w
    depends_on: [b, c]
"#;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("flow.yaml");
        std::fs::write(&path, yaml).unwrap();
        // Just check the parse + topo step — the print-side is
        // exercised by the integration test in the cli crate.
        let workflow = argo_workflow::parse_yaml(&path).unwrap();
        let order = workflow.topo_order().unwrap();
        let ids: Vec<&str> = order.iter().map(|t| t.id.as_str()).collect();
        assert_eq!(ids[0], "a");
        assert_eq!(ids[3], "d");
    }
}
