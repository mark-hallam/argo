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
use super::worker_actions_emit;
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
    auto_refresh_ledger(&memory, ws).await;
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
    auto_refresh_ledger(&memory, ws).await;
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
    auto_refresh_ledger(&memory, ws).await;
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
    auto_refresh_ledger(&memory, ws).await;
    Ok(())
}

pub(crate) async fn fail(action_id: &str, reason: &str, json: bool) -> ArgoResult<()> {
    let (memory, ws) = worker_actions_transitions::open_workspace_memory().await?;
    let rec = worker_actions_transitions::fail(&memory, ws, action_id, reason).await?;
    print_record(&rec, json);
    auto_refresh_ledger(&memory, ws).await;
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

/// `argo worker actions retry <failed-id>` — emit a fresh
/// pending action with the same payload + platform + item_id
/// as a previously-failed one (#737, retry policy added in
/// #755). The failed action_id is added to the new action's
/// `evidence` so the ledger captures the retry chain without
/// mutating the original row.
///
/// Retry policy enforcement (#755):
///   The bound vertical's manifest can declare
///   `[actions.<name>.retry]` with `max_attempts`. When set,
///   the OS counts the chain length from the source action's
///   `evidence` (every `argo://action/<id>` reference is one
///   prior attempt). If the chain has reached `max_attempts`,
///   retry refuses unless `--force` is passed. Operators who
///   bypass with `--force` get a stderr WARN breadcrumb so
///   audit reviewers can see when the limit was overridden.
///
/// Errors when:
/// - the source action_id doesn't exist
/// - the source isn't currently `failed`
/// - max_attempts reached without `--force`
/// - the source's `action_type` isn't declared in the bound
///   vertical's manifest (handled by `build_emission` —
///   undeclared types land `blocked` with an OS-level
///   capability-grant reason)
pub(crate) async fn retry(action_id: &str, force: bool, json: bool) -> ArgoResult<()> {
    let (memory, ws) = worker_actions_transitions::open_workspace_memory().await?;
    let located = worker_actions_transitions::find_action(&memory, ws, action_id)
        .await?
        .ok_or_else(|| ArgoError::Provider(format!("no action with action_id '{action_id}'")))?;
    if located.record.state != "failed" {
        return Err(ArgoError::Provider(format!(
            "action {action_id} is in state '{}', not 'failed'; \
             retry only applies to terminal failures",
            located.record.state
        )));
    }
    // Reload the manifest so build_emission can classify the
    // retried action against the current declarations.
    let dir = super::vertical::resolve_verticals_dir();
    let (registry, _errors) = argo_vertical::load_dir(&dir)?;
    let loaded = registry.get(&located.vertical_id).ok_or_else(|| {
        ArgoError::Provider(format!(
            "vertical '{}' not loaded; cannot retry",
            located.vertical_id
        ))
    })?;

    // ── retry-policy enforcement (#755) ────────────────────
    // Count the existing chain length: every `argo://action/`
    // entry in the source's evidence is one prior attempt.
    // The source itself counts as attempt #1, so the chain
    // length AT THIS RETRY's persist time will be
    // `prior_attempts + 1` (we're emitting the next attempt
    // right now). Compare against max_attempts to decide.
    let prior_attempts = count_retry_chain(&located.record);
    let next_attempt_number = prior_attempts + 1;
    let policy = loaded
        .manifest
        .actions
        .as_ref()
        .and_then(|a| a.actions.get(&located.record.action_type))
        .and_then(|d| d.retry.clone())
        .unwrap_or_default();
    if next_attempt_number > policy.max_attempts && !force {
        return Err(ArgoError::Provider(format!(
            "retry refused: action_type '{}' has max_attempts = {} and \
             this would be attempt #{}; pass --force to override (the \
             override is logged for audit)",
            located.record.action_type, policy.max_attempts, next_attempt_number
        )));
    }
    if force && next_attempt_number > policy.max_attempts {
        eprintln!(
            "argo worker: WARN — --force bypassed retry max_attempts={} \
             for '{}' (attempt #{} of action_id {})",
            policy.max_attempts,
            located.record.action_type,
            next_attempt_number,
            located.record.action_id,
        );
    }

    let mut new_evidence = located.record.evidence.clone();
    new_evidence.push(format!("argo://action/{}", located.record.action_id));

    let inputs = worker_actions_emit::ActionEmissionInputs {
        vertical: &loaded.manifest,
        job_id: &located.record.job_id,
        action_type: &located.record.action_type,
        payload: located.record.payload.clone(),
        item_id: located.record.item_id.as_deref(),
        platform: located.record.platform.as_deref(),
        evidence: new_evidence,
    };
    let new_rec = worker_actions_emit::build_emission(&inputs);
    worker_actions_emit::persist_action_record(&memory, ws, &located.vertical_id, &new_rec).await?;
    if json {
        let payload = serde_json::json!({
            "retried_from": located.record.action_id,
            "new_action": new_rec,
            "retry_attempt": next_attempt_number,
            "max_attempts": policy.max_attempts,
            "forced": force && next_attempt_number > policy.max_attempts,
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&payload).unwrap_or_else(|_| "{}".into())
        );
    } else {
        println!(
            "retried {} → new action {} (state {}, attempt {}/{})",
            located.record.action_id,
            new_rec.action_id,
            new_rec.state,
            next_attempt_number,
            policy.max_attempts,
        );
    }
    auto_refresh_ledger(&memory, ws).await;
    Ok(())
}

/// Count prior attempts in the retry chain. Every
/// `argo://action/<id>` reference in the action's `evidence`
/// is one prior attempt; the action itself counts as the
/// initial attempt, so the total chain length is `1 +
/// retry_marker_count`. Pure function on the ActionRecord —
/// no I/O, no side effects.
pub(crate) fn count_retry_chain(rec: &super::worker_actions::ActionRecord) -> u32 {
    let prior_retries = rec
        .evidence
        .iter()
        .filter(|e| e.starts_with("argo://action/"))
        .count() as u32;
    1 + prior_retries
}

/// Auto-refresh hook fired after every successful transition so
/// the operator's review surface (`.skycot/{approval_queue,
/// completed_actions, blocked_actions}.md`) stays current
/// without manual `argo worker actions render` calls (#735).
///
/// Failure is logged + swallowed — the transition itself already
/// landed; the ledger files are a derived projection that can
/// always be rebuilt from `argo worker actions render`.
async fn auto_refresh_ledger(memory: &MemoryStore, workspace: WorkspaceId) {
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
