//! CLI dispatch shim for the Worker v3 action transition
//! commands (#730). Owns formatting (text vs json), default
//! values (e.g. `--approved-by` falling back to `$USER`), and
//! the per-action transition handlers (approve, approve_batch,
//! execute, complete, fail, cancel, status).
//!
//! Lives in its own file so `worker_actions.rs` (which holds
//! the schema + read-side) stays under the per-file 500-LOC
//! budget.
//!
//! Sibling files split out under
//! claude-worker-actions-dispatch-split:
//! - `worker_actions_dispatch_ledger.rs` — workspace ledger
//!   rendering + auto-refresh hook (re-exported here as
//!   `render`).
//! - `worker_actions_dispatch_retry.rs` — retry handler +
//!   `count_retry_chain` (re-exported here so existing call
//!   sites — `worker_portrait`, `worker_actions_query`,
//!   transitions tests — keep working unchanged).

use super::worker_actions::ActionRecord;
use super::worker_actions_anomaly::check_action_anomaly;
use super::worker_actions_auto_approval;
use super::worker_actions_dispatch_ledger::auto_refresh_ledger;
use super::worker_actions_executor;
use super::worker_actions_transitions;
use argo_types::{ArgoError, ArgoResult};

pub(crate) use super::worker_actions_dispatch_ledger::render;
pub(crate) use super::worker_actions_dispatch_retry::{count_retry_chain, retry};

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
    deny: bool,
    confirm: bool,
    dry_run: bool,
    json: bool,
) -> ArgoResult<()> {
    use super::worker_actions_batch::{self, BatchDecision};
    if action_ids.is_empty() {
        return Err(ArgoError::Provider(
            "approve-batch requires at least one action_id".into(),
        ));
    }
    let decision = BatchDecision::from_flag(deny);
    if dry_run {
        return worker_actions_batch::print_dry_run(action_ids, decision, json);
    }
    if !confirm && !worker_actions_batch::prompt_confirms(action_ids.len(), decision)? {
        eprintln!("aborted: no decision applied");
        return Ok(());
    }
    let (memory, ws) = worker_actions_transitions::open_workspace_memory().await?;
    let by = default_approver(approved_by);
    let (ok, failed) =
        worker_actions_batch::apply_decisions(&memory, ws, action_ids, &by, note, decision).await;
    worker_actions_batch::print_summary(&ok, &failed, decision, json);
    auto_refresh_ledger(&memory, ws).await;
    if !failed.is_empty() {
        return Err(ArgoError::Provider(format!(
            "{}/{} {} decisions failed",
            failed.len(),
            action_ids.len(),
            decision.label(),
        )));
    }
    Ok(())
}

pub(crate) async fn execute(action_id: &str, dry_run: bool, json: bool) -> ArgoResult<()> {
    let (memory, ws) = worker_actions_transitions::open_workspace_memory().await?;
    worker_actions_auto_approval::ensure_circuit_allows_dispatch(&memory, ws, action_id).await?;
    if dry_run {
        return super::worker_actions_dry_run::preview(&memory, ws, action_id, json).await;
    }
    // Stream H-2: auto-approval gate runs first so an
    // auto-approved action gets a real `approve()` audit row.
    worker_actions_auto_approval::maybe_auto_approve(&memory, ws, action_id).await?;
    let rec = worker_actions_transitions::execute(&memory, ws, action_id).await?;
    let (final_rec, bundle_path) = match invoke_executor_for(&rec).await? {
        worker_actions_executor::ActionOutcome::HumanHandoff { bundle_path } => (rec, bundle_path),
        // Auto-complete path (#A-executor-trait): the executor
        // did the work end-to-end, so write a fresh
        // `executing → completed` row.
        worker_actions_executor::ActionOutcome::Completed {
            bundle_path,
            result_json,
        } => {
            let result_str = serde_json::to_string(&result_json).unwrap_or_else(|_| "{}".into());
            let auto_note = format!("auto-completed by executor for `{}`", rec.action_type);
            let completed = worker_actions_transitions::complete(
                &memory,
                ws,
                action_id,
                Some(&auto_note),
                Some(&result_str),
            )
            .await?;
            (completed, bundle_path)
        }
    };
    if json {
        let mut payload = serde_json::to_value(&final_rec).unwrap_or(serde_json::json!({}));
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
        print_record(&final_rec, false);
        println!("  bundle       {}", bundle_path.display());
    }
    auto_refresh_ledger(&memory, ws).await;
    Ok(())
}

/// Find the manifest that owns this action and dispatch to its
/// declared executor.
async fn invoke_executor_for(
    rec: &ActionRecord,
) -> ArgoResult<worker_actions_executor::ActionOutcome> {
    let dir = super::vertical::resolve_verticals_dir();
    let (registry, _errors) = argo_vertical::load_dir(&dir)?;
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
    executor.dispatch(rec, &cwd)
}

pub(crate) async fn complete(
    action_id: &str,
    note: Option<&str>,
    result_json: Option<&str>,
    json: bool,
) -> ArgoResult<()> {
    let (memory, ws, store) =
        worker_actions_transitions::open_workspace_memory_with_store().await?;
    let rec =
        worker_actions_transitions::complete(&memory, ws, action_id, note, result_json).await?;
    if let Err(e) = check_action_anomaly(&store, &rec).await {
        eprintln!("argo worker actions complete: anomaly check failed: {e}");
    }
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

pub(crate) async fn cancel(
    action_id: &str,
    cancelled_by: Option<&str>,
    note: Option<&str>,
    json: bool,
) -> ArgoResult<()> {
    let (memory, ws) = worker_actions_transitions::open_workspace_memory().await?;
    let by = default_approver(cancelled_by);
    let rec = worker_actions_transitions::cancel(&memory, ws, action_id, &by, note).await?;
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
