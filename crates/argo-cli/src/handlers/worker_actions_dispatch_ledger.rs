//! Workspace-level ledger rendering for the worker-actions
//! dispatch surface. Owns the `.skycot/{approval_queue,
//! completed_actions,blocked_actions}.md` write-out and the
//! cross-vertical reduction that feeds it. Split out of
//! `worker_actions_dispatch.rs` (claude-worker-actions-dispatch-split)
//! to keep that file under the 500-LOC HARD RULE 6 ceiling. Pure
//! refactor; no behaviour change.

use super::worker_actions::ActionRecord;
use super::worker_actions_render;
use super::worker_actions_transitions;
use argo_memory::{MemoryFile, MemoryStore};
use argo_types::{ArgoError, ArgoResult, WorkspaceId};

/// `argo worker actions render [--vertical X]`. Walks every
/// loaded vertical, collects the latest snapshot per action_id,
/// then writes 3 markdown files at the workspace root under
/// `.skycot/`.
pub(crate) async fn render(vertical: Option<&str>) -> ArgoResult<()> {
    let (memory, ws) = worker_actions_transitions::open_workspace_memory().await?;
    let counts = refresh_workspace_ledger(&memory, ws, vertical).await?;
    println!(
        "wrote {} (queue), {} (completed), {} (blocked) under {}",
        counts.pending_approval,
        counts.completed,
        counts.blocked,
        counts.dir.display(),
    );
    Ok(())
}

/// Counts surfaced by `refresh_workspace_ledger` so callers can
/// log meaningful one-liners without re-walking the records.
struct LedgerCounts {
    pending_approval: usize,
    completed: usize,
    blocked: usize,
    dir: std::path::PathBuf,
}

/// Re-render the workspace-level ledger files (approval_queue,
/// completed_actions, blocked_actions). Used by both the
/// explicit `render` CLI command and the auto-refresh hook
/// that fires after every transition (#735).
async fn refresh_workspace_ledger(
    memory: &MemoryStore,
    workspace: WorkspaceId,
    vertical: Option<&str>,
) -> ArgoResult<LedgerCounts> {
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Provider(format!("cwd: {e}")))?;
    let records = collect_latest_actions(memory, workspace, vertical).await?;
    let dir = cwd.join(".skycot");
    std::fs::create_dir_all(&dir)
        .map_err(|e| ArgoError::Provider(format!("create .skycot dir: {e}")))?;
    let queue = worker_actions_render::render_approval_queue(&records);
    let completed = worker_actions_render::render_completed_actions(&records);
    let blocked = worker_actions_render::render_blocked_actions(&records);
    write(&dir.join("approval_queue.md"), &queue)?;
    write(&dir.join("completed_actions.md"), &completed)?;
    write(&dir.join("blocked_actions.md"), &blocked)?;
    Ok(LedgerCounts {
        pending_approval: records
            .iter()
            .filter(|r| r.state == "pending" && r.requires_approval)
            .count(),
        completed: records.iter().filter(|r| r.state == "completed").count(),
        blocked: records.iter().filter(|r| r.state == "blocked").count(),
        dir,
    })
}

/// Auto-refresh hook fired after every successful transition so
/// the operator's review surface stays current (#735). Failures
/// are logged + swallowed since the ledger files are a derived
/// projection rebuildable via `argo worker actions render`.
pub(super) async fn auto_refresh_ledger(memory: &MemoryStore, workspace: WorkspaceId) {
    if let Err(e) = refresh_workspace_ledger(memory, workspace, None).await {
        eprintln!("argo worker: warning — workspace ledger auto-refresh failed: {e}");
    }
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
