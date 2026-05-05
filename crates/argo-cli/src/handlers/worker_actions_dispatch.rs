//! CLI dispatch shim for the Worker v3 action transition
//! commands (#730). Owns formatting (text vs json), default
//! values (e.g. `--approved-by` falling back to `$USER`), and
//! the workspace-level `render` handler that walks every
//! loaded vertical, groups every action by state, and writes
//! `.skycot/{approval_queue,completed_actions,blocked_actions}.md`.
//!
//! Lives in its own file so `worker_actions.rs` (which holds
//! the schema + read-side) stays under the per-file 500-LOC
//! budget.

use super::worker_actions::ActionRecord;
use super::worker_actions_executor;
use super::worker_actions_render;
use super::worker_actions_transitions;
use argo_memory::{MemoryFile, MemoryStore};
use argo_types::{ArgoError, ArgoResult, WorkspaceId};

fn print_record(rec: &ActionRecord, json: bool) {
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(rec).unwrap_or_else(|_| "{}".into())
        );
    } else {
        println!("{} → {}", rec.action_id, rec.state);
        if let Some(at) = rec.approved_at.as_deref() {
            println!("  approved_at  {at}");
        }
        if let Some(by) = rec.approved_by.as_deref() {
            println!("  approved_by  {by}");
        }
        if let Some(at) = rec.executed_at.as_deref() {
            println!("  executed_at  {at}");
        }
        if let Some(reason) = rec.blocked_reason.as_deref() {
            println!("  blocked      {reason}");
        }
    }
}

fn default_approver(supplied: Option<&str>) -> String {
    if let Some(s) = supplied {
        return s.to_string();
    }
    std::env::var("USER").unwrap_or_else(|_| "operator".to_string())
}

pub(crate) async fn approve(
    action_id: &str,
    approved_by: Option<&str>,
    note: Option<&str>,
    json: bool,
) -> ArgoResult<()> {
    let (memory, ws) = worker_actions_transitions::open_workspace_memory().await?;
    let by = default_approver(approved_by);
    let rec = worker_actions_transitions::approve(&memory, ws, action_id, &by, note).await?;
    print_record(&rec, json);
    Ok(())
}

pub(crate) async fn approve_batch(
    action_ids: &[String],
    approved_by: Option<&str>,
    note: Option<&str>,
    json: bool,
) -> ArgoResult<()> {
    if action_ids.is_empty() {
        return Err(ArgoError::Provider(
            "approve-batch requires at least one action_id".into(),
        ));
    }
    let (memory, ws) = worker_actions_transitions::open_workspace_memory().await?;
    let by = default_approver(approved_by);
    let mut ok = Vec::new();
    let mut failed: Vec<(String, String)> = Vec::new();
    for id in action_ids {
        match worker_actions_transitions::approve(&memory, ws, id, &by, note).await {
            Ok(rec) => ok.push(rec),
            Err(e) => failed.push((id.clone(), e.to_string())),
        }
    }
    if json {
        let payload = serde_json::json!({
            "approved": ok,
            "failed": failed.iter().map(|(id, e)| serde_json::json!({"action_id": id, "error": e})).collect::<Vec<_>>(),
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&payload).unwrap_or_else(|_| "{}".into())
        );
    } else {
        for r in &ok {
            println!("approved {} (job {})", r.action_id, r.job_id);
        }
        for (id, err) in &failed {
            eprintln!("FAILED {id}: {err}");
        }
    }
    if !failed.is_empty() {
        return Err(ArgoError::Provider(format!(
            "{}/{} approvals failed",
            failed.len(),
            action_ids.len()
        )));
    }
    Ok(())
}

pub(crate) async fn execute(action_id: &str, json: bool) -> ArgoResult<()> {
    let (memory, ws) = worker_actions_transitions::open_workspace_memory().await?;
    // 1. Advance state pending/approved → executing.
    let rec = worker_actions_transitions::execute(&memory, ws, action_id).await?;
    // 2. Resolve the executor declared by the manifest. v1 only
    //    supports `executor = "human"` — anything else is a hard
    //    error so the operator sees the gap. Future executors
    //    (browser, marketplace SDK) plug in here.
    let bundle_path = invoke_executor_for(&rec).await?;
    if json {
        let mut payload = serde_json::to_value(&rec).unwrap_or(serde_json::json!({}));
        if let Some(map) = payload.as_object_mut() {
            map.insert(
                "execution_bundle_path".to_string(),
                serde_json::Value::String(bundle_path.to_string_lossy().into_owned()),
            );
        }
        println!(
            "{}",
            serde_json::to_string_pretty(&payload).unwrap_or_else(|_| "{}".into())
        );
    } else {
        print_record(&rec, false);
        println!("  bundle       {}", bundle_path.display());
    }
    Ok(())
}

/// Hand an executing action to whichever executor its manifest
/// declares. Lifted out of `execute` so the file stays small +
/// testable. Returns the path to the per-action artifact bundle.
async fn invoke_executor_for(rec: &ActionRecord) -> ArgoResult<std::path::PathBuf> {
    let dir = super::vertical::resolve_verticals_dir();
    let (registry, _errors) = argo_vertical::load_dir(&dir)?;
    // Find the manifest that owns this action. We don't have
    // vertical_id on the ActionRecord directly; walk the
    // registry's action declarations looking for one that
    // declares this action_type.
    let manifest = registry
        .list()
        .find_map(|(_, loaded)| {
            loaded
                .manifest
                .actions
                .as_ref()
                .and_then(|a| a.actions.get(&rec.action_type))
                .map(|_| &loaded.manifest)
        })
        .ok_or_else(|| {
            ArgoError::Provider(format!(
                "no loaded vertical declares action_type '{}' — cannot resolve executor",
                rec.action_type
            ))
        })?;
    let executor = worker_actions_executor::resolve_executor(manifest, &rec.action_type)?;
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Provider(format!("cwd: {e}")))?;
    executor.execute(rec, &cwd)
}

pub(crate) async fn complete(
    action_id: &str,
    note: Option<&str>,
    result_json: Option<&str>,
    json: bool,
) -> ArgoResult<()> {
    let (memory, ws) = worker_actions_transitions::open_workspace_memory().await?;
    let rec =
        worker_actions_transitions::complete(&memory, ws, action_id, note, result_json).await?;
    print_record(&rec, json);
    Ok(())
}

pub(crate) async fn fail(action_id: &str, reason: &str, json: bool) -> ArgoResult<()> {
    let (memory, ws) = worker_actions_transitions::open_workspace_memory().await?;
    let rec = worker_actions_transitions::fail(&memory, ws, action_id, reason).await?;
    print_record(&rec, json);
    Ok(())
}

pub(crate) async fn status(action_id: &str, json: bool) -> ArgoResult<()> {
    let (memory, ws) = worker_actions_transitions::open_workspace_memory().await?;
    let history = worker_actions_transitions::full_history(&memory, ws, action_id).await?;
    if history.is_empty() {
        return Err(ArgoError::Provider(format!(
            "no action with action_id '{action_id}'"
        )));
    }
    if json {
        for h in &history {
            println!(
                "{}",
                serde_json::to_string(h).unwrap_or_else(|_| "{}".into())
            );
        }
    } else {
        // SAFETY: we returned early above when history was
        // empty, so `last()` is total here.
        let Some(latest) = history.last() else {
            return Ok(());
        };
        println!("action_id   {}", latest.action_id);
        println!("type        {}", latest.action_type);
        println!("job         {}", latest.job_id);
        println!("transitions ({} state-row(s)):", history.len());
        for h in &history {
            println!("  {} — {}", h.created_at, h.state);
            if let Some(by) = h.approved_by.as_deref() {
                println!("      approved_by: {by}");
            }
            if let Some(reason) = h.blocked_reason.as_deref() {
                println!("      blocked: {reason}");
            }
        }
    }
    Ok(())
}

/// `argo worker actions render [--vertical X]`. Walks every
/// loaded vertical, collects the latest snapshot per action_id,
/// then writes 3 markdown files at the workspace root under
/// `.skycot/`.
pub(crate) async fn render(vertical: Option<&str>) -> ArgoResult<()> {
    let (memory, ws) = worker_actions_transitions::open_workspace_memory().await?;
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Provider(format!("cwd: {e}")))?;
    let records = collect_latest_actions(&memory, ws, vertical).await?;
    let dir = cwd.join(".skycot");
    std::fs::create_dir_all(&dir)
        .map_err(|e| ArgoError::Provider(format!("create .skycot dir: {e}")))?;
    let queue = worker_actions_render::render_approval_queue(&records);
    let completed = worker_actions_render::render_completed_actions(&records);
    let blocked = worker_actions_render::render_blocked_actions(&records);
    write(&dir.join("approval_queue.md"), &queue)?;
    write(&dir.join("completed_actions.md"), &completed)?;
    write(&dir.join("blocked_actions.md"), &blocked)?;
    println!(
        "wrote {} (queue), {} (completed), {} (blocked) under {}",
        records
            .iter()
            .filter(|r| r.state == "pending" && r.requires_approval)
            .count(),
        records.iter().filter(|r| r.state == "completed").count(),
        records.iter().filter(|r| r.state == "blocked").count(),
        dir.display(),
    );
    Ok(())
}

fn write(path: &std::path::Path, body: &str) -> ArgoResult<()> {
    std::fs::write(path, body)
        .map_err(|e| ArgoError::Provider(format!("write {}: {e}", path.display())))
}

/// Latest-wins reduction across a vertical (or all loaded
/// verticals when None). Lifted from the read-side `list`
/// helper but inlined here because we want to share it with the
/// `render` handler without making the original helper public.
async fn collect_latest_actions(
    memory: &MemoryStore,
    workspace: WorkspaceId,
    vertical: Option<&str>,
) -> ArgoResult<Vec<ActionRecord>> {
    let dir = super::vertical::resolve_verticals_dir();
    let (registry, _errors) = argo_vertical::load_dir(&dir)?;
    let vertical_ids: Vec<String> = match vertical {
        Some(v) => vec![v.to_string()],
        None => registry.list().map(|(id, _)| id.to_string()).collect(),
    };
    let mut latest: std::collections::HashMap<String, ActionRecord> =
        std::collections::HashMap::new();
    for vid in &vertical_ids {
        let entries = memory
            .list_for_vertical(workspace, MemoryFile::Memory, vid)
            .await?;
        for e in entries {
            let Ok(rec) = serde_json::from_str::<ActionRecord>(&e.body) else {
                continue;
            };
            if rec.schema_key != ActionRecord::SCHEMA_KEY {
                continue;
            }
            match latest.get(&rec.action_id) {
                Some(prev) if prev.created_at >= rec.created_at => {}
                _ => {
                    latest.insert(rec.action_id.clone(), rec);
                }
            }
        }
    }
    Ok(latest.into_values().collect())
}
