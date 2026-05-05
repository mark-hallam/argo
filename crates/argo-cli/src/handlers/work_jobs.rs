//! `argo work jobs list` — read-side view of the worker queue
//! (#742). Walks the `pending_jobs` + `completed_jobs` memory
//! rows the queue runner already uses, so what the operator
//! sees here is exactly what the worker would process / has
//! processed.
//!
//! Quality-of-life on top of `argo memory list --body-contains
//! pending_jobs` — that flow works but requires the operator
//! to know the schema_keys + drill into row bodies. This
//! command formats the same data with the columns operators
//! actually want (state / vertical / job_id / created_at /
//! stop_reason / output_dir).

use crate::handlers::vertical::resolve_verticals_dir;
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_memory::{MemoryFile, MemoryStore};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, WorkspaceId};
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Clone, Serialize)]
pub(crate) struct JobRow {
    pub job_id: String,
    pub vertical_id: String,
    pub state: String,
    pub prompt_preview: String,
    pub created_at: String,
    /// Set for completed jobs only.
    pub stop_reason: Option<String>,
    /// Path to the artifact bundle dir (set for completed jobs).
    pub output_dir: Option<String>,
    /// Only meaningful for completed jobs.
    pub steps_completed: Option<u32>,
    pub actions_emitted: Option<u32>,
}

pub(crate) async fn list(
    state_filter: &str,
    vertical: Option<&str>,
    limit: usize,
    json_out: bool,
) -> ArgoResult<()> {
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Provider(format!("cwd: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(&db_path(&home)).await?;
    store.migrate().await?;
    let name = cwd
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("workspace");
    let root = cwd.to_string_lossy();
    let (_t, ws) = store.ensure_workspace(name, &root).await?;
    let memory = MemoryStore::new(Arc::new(store));

    let mut rows = collect_jobs(&memory, ws, vertical).await?;
    rows.retain(|r| match state_filter {
        "all" => true,
        other => r.state == other,
    });
    rows.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    rows.truncate(limit);

    if json_out {
        for r in &rows {
            println!(
                "{}",
                serde_json::to_string(&r).unwrap_or_else(|_| "{}".into())
            );
        }
    } else if rows.is_empty() {
        println!("(no jobs)");
    } else {
        println!(
            "job_id                                state      vertical        steps stop_reason          created_at"
        );
        for r in &rows {
            println!(
                "{:<37} {:<10} {:<14}  {:<4}  {:<19}  {}",
                r.job_id,
                r.state,
                truncate(&r.vertical_id, 14),
                r.steps_completed
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "-".into()),
                r.stop_reason.as_deref().unwrap_or("-"),
                r.created_at,
            );
        }
    }
    Ok(())
}

async fn collect_jobs(
    memory: &MemoryStore,
    workspace: WorkspaceId,
    vertical_filter: Option<&str>,
) -> ArgoResult<Vec<JobRow>> {
    let dir = resolve_verticals_dir();
    let (registry, _errors) = argo_vertical::load_dir(&dir)?;
    let vertical_ids: Vec<String> = match vertical_filter {
        Some(v) => vec![v.to_string()],
        None => registry.list().map(|(id, _)| id.to_string()).collect(),
    };

    // Outcome record extracted from a `completed_jobs` row;
    // merged into the matching pending row in a second pass.
    struct Outcome {
        completed_at: String,
        stop_reason: Option<String>,
        output_dir: Option<String>,
        steps_completed: Option<u32>,
        actions_emitted: Option<u32>,
    }

    // Two passes per vertical: first collect every pending_jobs
    // row, then mark any whose UUID appears as a
    // completed_jobs.pending_job_id as "completed" + merge
    // outcome fields. Same dedup logic as the queue runner.
    let mut pending: HashMap<String, JobRow> = HashMap::new();
    let mut completed: HashMap<String, Outcome> = HashMap::new();

    for vid in &vertical_ids {
        let entries = memory
            .list_for_vertical(workspace, MemoryFile::Memory, vid)
            .await?;
        for e in entries {
            let Ok(body) = serde_json::from_str::<Value>(&e.body) else {
                continue;
            };
            let schema = body
                .get("schema_key")
                .and_then(|s| s.as_str())
                .unwrap_or("");
            match schema {
                "pending_jobs" => {
                    let row = JobRow {
                        job_id: e.id.to_string(),
                        vertical_id: vid.clone(),
                        state: "pending".to_string(),
                        prompt_preview: body
                            .get("prompt")
                            .and_then(|p| p.as_str())
                            .map(|s| s.chars().take(80).collect::<String>())
                            .unwrap_or_default(),
                        created_at: body
                            .get("created_at")
                            .and_then(|c| c.as_str())
                            .unwrap_or("")
                            .to_string(),
                        stop_reason: None,
                        output_dir: None,
                        steps_completed: None,
                        actions_emitted: None,
                    };
                    pending.insert(e.id.to_string(), row);
                }
                "completed_jobs" => {
                    let pending_id = body
                        .get("pending_job_id")
                        .and_then(|s| s.as_str())
                        .map(String::from);
                    let stop_reason = body
                        .get("stop_reason")
                        .and_then(|s| s.as_str())
                        .map(String::from);
                    let output_dir = body
                        .get("output_dir")
                        .and_then(|s| s.as_str())
                        .map(String::from);
                    let completed_at = body
                        .get("completed_at")
                        .and_then(|s| s.as_str())
                        .unwrap_or("")
                        .to_string();
                    let steps = body
                        .get("steps_completed")
                        .and_then(|n| n.as_u64())
                        .map(|n| n as u32);
                    let actions = body
                        .get("actions_emitted")
                        .and_then(|n| n.as_u64())
                        .map(|n| n as u32);
                    if let Some(pid) = pending_id {
                        completed.insert(
                            pid,
                            Outcome {
                                completed_at,
                                stop_reason,
                                output_dir,
                                steps_completed: steps,
                                actions_emitted: actions,
                            },
                        );
                    }
                }
                _ => {}
            }
        }
    }

    let mut out = Vec::new();
    for (id, mut row) in pending.into_iter() {
        if let Some(Outcome {
            completed_at,
            stop_reason,
            output_dir,
            steps_completed: steps,
            actions_emitted: actions,
        }) = completed.remove(&id)
        {
            row.state = "completed".to_string();
            row.stop_reason = stop_reason;
            row.output_dir = output_dir;
            row.steps_completed = steps;
            row.actions_emitted = actions;
            // Use completed_at as the row's "when" so completed
            // jobs sort to the top of the recency view.
            if !completed_at.is_empty() {
                row.created_at = completed_at;
            }
        }
        out.push(row);
    }
    Ok(out)
}

fn truncate(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        s.to_string()
    } else {
        format!(
            "{}…",
            s.chars().take(n.saturating_sub(1)).collect::<String>()
        )
    }
}

#[cfg(test)]
#[path = "work_jobs_tests.rs"]
mod tests;
