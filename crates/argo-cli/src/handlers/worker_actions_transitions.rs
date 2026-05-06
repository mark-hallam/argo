//! Worker v3 action state-transition CLI (#730).
//!
//! Sibling of `worker_actions` (which owns the read-side and
//! the persisted `ActionRecord` schema). This module is the
//! write-side: `approve`, `approve-batch`, `execute`,
//! `complete`, `fail`, and the `status` history view.
//!
//! ## State machine
//!
//! ```text
//!   pending ──→ approved ──→ executing ──→ completed
//!     │           │              │           ▲
//!     │           └──────────────┴───────────┘   (synchronous shortcut)
//!     │                                ↓
//!     │                              failed
//!     ↓
//!   blocked  (terminal)
//! ```
//!
//! Each transition appends a NEW `worker_action` memory row
//! carrying the full snapshot at the new state. The "current
//! state" of an action is the latest row matching its
//! `action_id` (latest-wins per `created_at`). Append-only —
//! prior rows are never mutated.
//!
//! ## Approval boundary
//!
//! `approve` is the ONLY way to advance an approval-gated
//! action past `pending`. The validator refuses any
//! `pending → executing` transition for a row whose
//! `requires_approval = true`. This is enforced in code, not
//! prompts — the safety boundary is a state-machine invariant
//! the OS holds, not a model behaviour the worker honours.
//!
//! See `docs/design/worker_v3_execution.md` §2 + §5 for the
//! full spec.

use super::worker_actions::{ActionRecord, ActionState};
use crate::handlers::vertical::resolve_verticals_dir;
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_memory::{MemoryFile, MemoryStore};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, WorkspaceId};
use serde_json::Value;
use std::sync::Arc;

/// Lookup result: the latest snapshot of an action plus the
/// vertical_id its memory rows live under. Knowing the
/// vertical is required to write the next state-row back into
/// the correct namespace.
#[derive(Clone)]
pub(crate) struct LocatedAction {
    pub record: ActionRecord,
    pub vertical_id: String,
}

/// Walk every loaded vertical's memory namespace looking for
/// rows whose body's `action_id` matches. Returns the latest-
/// wins snapshot + the vertical_id that owns it.
pub(crate) async fn find_action(
    memory: &MemoryStore,
    workspace: WorkspaceId,
    action_id: &str,
) -> ArgoResult<Option<LocatedAction>> {
    let dir = resolve_verticals_dir();
    let (registry, _errors) = argo_vertical::load_dir(&dir)?;
    let mut latest: Option<LocatedAction> = None;
    for (vid, _loaded) in registry.list() {
        let rows = memory
            .list_for_vertical(workspace, MemoryFile::Memory, vid)
            .await?;
        for e in rows {
            let Ok(rec) = serde_json::from_str::<ActionRecord>(&e.body) else {
                continue;
            };
            if rec.schema_key != ActionRecord::SCHEMA_KEY {
                continue;
            }
            if rec.action_id != action_id {
                continue;
            }
            let take = match &latest {
                None => true,
                Some(prev) => rec.created_at > prev.record.created_at,
            };
            if take {
                latest = Some(LocatedAction {
                    record: rec,
                    vertical_id: vid.to_string(),
                });
            }
        }
    }
    Ok(latest)
}

/// Walk every loaded vertical and return every persisted state-row
/// for `action_id` in time order (oldest first). Used by `status`
/// to render the full transition history. Empty when the
/// action_id is unknown.
pub(crate) async fn full_history(
    memory: &MemoryStore,
    workspace: WorkspaceId,
    action_id: &str,
) -> ArgoResult<Vec<ActionRecord>> {
    let dir = resolve_verticals_dir();
    let (registry, _errors) = argo_vertical::load_dir(&dir)?;
    let mut out = Vec::new();
    for (vid, _loaded) in registry.list() {
        let rows = memory
            .list_for_vertical(workspace, MemoryFile::Memory, vid)
            .await?;
        for e in rows {
            let Ok(rec) = serde_json::from_str::<ActionRecord>(&e.body) else {
                continue;
            };
            if rec.schema_key != ActionRecord::SCHEMA_KEY {
                continue;
            }
            if rec.action_id != action_id {
                continue;
            }
            out.push(rec);
        }
    }
    out.sort_by(|a, b| a.created_at.cmp(&b.created_at));
    Ok(out)
}

/// Workspace bootstrap shared by every transition handler. Opens
/// the session store, resolves the canonical workspace, returns a
/// `(MemoryStore, WorkspaceId)` pair ready to use.
pub(crate) async fn open_workspace_memory() -> ArgoResult<(MemoryStore, WorkspaceId)> {
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
    let (_tenant, workspace) = store.ensure_workspace(name, &root).await?;
    let memory = MemoryStore::new(Arc::new(store));
    Ok((memory, workspace))
}

/// Persist a state-transition row. Validates `current → next` via
/// `ActionState::can_transition_to`. Returns the persisted record.
async fn write_transition(
    memory: &MemoryStore,
    workspace: WorkspaceId,
    located: &LocatedAction,
    next_state: ActionState,
    mutate: impl FnOnce(&mut ActionRecord),
) -> ArgoResult<ActionRecord> {
    let current = ActionState::parse(&located.record.state).ok_or_else(|| {
        ArgoError::Provider(format!(
            "action {}: unknown current state '{}'",
            located.record.action_id, located.record.state
        ))
    })?;
    if !current.can_transition_to(next_state) {
        return Err(ArgoError::Provider(format!(
            "action {}: illegal transition {} → {}",
            located.record.action_id,
            current.as_str(),
            next_state.as_str()
        )));
    }
    let mut next = located.record.clone();
    next.state = next_state.as_str().to_string();
    next.created_at = chrono::Utc::now().to_rfc3339();
    mutate(&mut next);
    let body = serde_json::to_string(&next)
        .map_err(|e| ArgoError::Provider(format!("serialize ActionRecord: {e}")))?;
    memory
        .append_for_vertical(
            workspace,
            MemoryFile::Memory,
            body,
            None,
            &located.vertical_id,
        )
        .await?;
    Ok(next)
}

/// `pending → approved`. Refuses if the action isn't currently
/// `pending` or its manifest classification doesn't require
/// approval (autonomous-safe actions don't need to be approved
/// — they go pending → executing directly).
pub(crate) async fn approve(
    memory: &MemoryStore,
    workspace: WorkspaceId,
    action_id: &str,
    approved_by: &str,
    note: Option<&str>,
) -> ArgoResult<ActionRecord> {
    let located = find_action(memory, workspace, action_id)
        .await?
        .ok_or_else(|| ArgoError::Provider(format!("no action with action_id '{action_id}'")))?;
    if !located.record.requires_approval {
        return Err(ArgoError::Provider(format!(
            "action {action_id} is autonomous-safe (no approval required); \
             use `execute` to advance it directly"
        )));
    }
    let now = chrono::Utc::now().to_rfc3339();
    let by = approved_by.to_string();
    let note = note.map(String::from);
    write_transition(memory, workspace, &located, ActionState::Approved, |rec| {
        rec.approved_by = Some(by);
        rec.approved_at = Some(now);
        if let Some(n) = note {
            rec.result = Some(serde_json::json!({ "approval_note": n }));
        }
    })
    .await
}

/// Move an action into `executing`. Valid sources: `approved`
/// (typical) or `pending` for autonomous-safe actions. Records
/// `executed_at` so the operator can audit when execution
/// started.
pub(crate) async fn execute(
    memory: &MemoryStore,
    workspace: WorkspaceId,
    action_id: &str,
) -> ArgoResult<ActionRecord> {
    let located = find_action(memory, workspace, action_id)
        .await?
        .ok_or_else(|| ArgoError::Provider(format!("no action with action_id '{action_id}'")))?;
    let current = ActionState::parse(&located.record.state)
        .ok_or_else(|| ArgoError::Provider(format!("action {action_id}: unknown current state")))?;
    // For pending actions, only autonomous-safe ones may go
    // straight to executing — approval-gated must approve first.
    if current == ActionState::Pending && located.record.requires_approval {
        return Err(ArgoError::Provider(format!(
            "action {action_id}: approval required (state pending, requires_approval=true); \
             run `argo worker actions approve {action_id}` first"
        )));
    }
    let now = chrono::Utc::now().to_rfc3339();
    write_transition(memory, workspace, &located, ActionState::Executing, |rec| {
        rec.executed_at = Some(now);
    })
    .await
}

/// `executing → completed` (or `approved → completed` as a
/// synchronous shortcut for human-executed actions where the
/// operator clicks complete without a separate execute step).
pub(crate) async fn complete(
    memory: &MemoryStore,
    workspace: WorkspaceId,
    action_id: &str,
    note: Option<&str>,
    result_json: Option<&str>,
) -> ArgoResult<ActionRecord> {
    let located = find_action(memory, workspace, action_id)
        .await?
        .ok_or_else(|| ArgoError::Provider(format!("no action with action_id '{action_id}'")))?;
    let parsed_result: Option<Value> = match result_json {
        None => None,
        Some(s) => Some(
            serde_json::from_str(s)
                .map_err(|e| ArgoError::Provider(format!("--result-json invalid: {e}")))?,
        ),
    };
    let now = chrono::Utc::now().to_rfc3339();
    let note = note.map(String::from);
    let completed = write_transition(memory, workspace, &located, ActionState::Completed, |rec| {
        rec.executed_at.get_or_insert(now);
        let mut payload = parsed_result.unwrap_or_else(|| serde_json::json!({}));
        if let Some(n) = note {
            if let Some(map) = payload.as_object_mut() {
                map.insert("summary".into(), Value::String(n));
            } else {
                payload = serde_json::json!({ "summary": n });
            }
        }
        rec.result = Some(payload);
    })
    .await?;
    // #771 + #778 — auto-emit any follow-up actions the
    // manifest declares for this action_type's
    // `on_complete_emit`. Pass the COMPLETED record (with the
    // operator's result populated) so the chain payload
    // propagation in #777 sees the operator's verdict — not
    // the stale pre-completion snapshot from `find_action`.
    // Errors are logged + swallowed so a follow-up failure
    // cannot undo the operator's `complete` transition.
    let completed_source = LocatedAction {
        record: completed.clone(),
        vertical_id: located.vertical_id.clone(),
    };
    super::worker_actions_followup::emit_on_complete_followups(
        memory,
        workspace,
        &completed_source,
    )
    .await;
    Ok(completed)
}

/// `executing → failed`. Records the operator-supplied reason
/// into `result.failure` so retry / triage tooling has a
/// structured handle. `failed` is terminal for THIS action —
/// retries create a fresh action that references the failed
/// one in `evidence`.
pub(crate) async fn fail(
    memory: &MemoryStore,
    workspace: WorkspaceId,
    action_id: &str,
    reason: &str,
) -> ArgoResult<ActionRecord> {
    let located = find_action(memory, workspace, action_id)
        .await?
        .ok_or_else(|| ArgoError::Provider(format!("no action with action_id '{action_id}'")))?;
    let reason_string = reason.to_string();
    write_transition(memory, workspace, &located, ActionState::Failed, |rec| {
        rec.result = Some(serde_json::json!({ "failure": reason_string }));
    })
    .await
}

/// Cancel a `pending` or `approved` action. Records the
/// operator + optional note into `result.cancellation`. Refused
/// for other states (Stream `claude-action-cancel-flow`).
pub(crate) async fn cancel(
    memory: &MemoryStore,
    workspace: WorkspaceId,
    action_id: &str,
    cancelled_by: &str,
    note: Option<&str>,
) -> ArgoResult<ActionRecord> {
    let located = find_action(memory, workspace, action_id)
        .await?
        .ok_or_else(|| ArgoError::Provider(format!("no action with action_id '{action_id}'")))?;
    let cancelled_by = cancelled_by.to_string();
    let note = note.map(str::to_string);
    write_transition(memory, workspace, &located, ActionState::Cancelled, |rec| {
        let mut payload = serde_json::json!({
            "cancelled_by": cancelled_by,
            "cancelled_at": chrono::Utc::now().to_rfc3339(),
        });
        if let Some(n) = note {
            payload["note"] = serde_json::Value::String(n);
        }
        rec.result = Some(serde_json::json!({ "cancellation": payload }));
    })
    .await
}

#[cfg(test)]
#[path = "worker_actions_transitions_tests.rs"]
mod tests;
