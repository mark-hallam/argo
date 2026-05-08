//! Dry-run preview for `argo worker actions execute`.
//!
//! The preview path deliberately stops before the transition
//! writer and before executor dispatch. It validates that the
//! current state could advance to `executing`, resolves the
//! manifest executor when available, then renders a stable JSON
//! or text summary for operator review.

use super::worker_actions::{ActionRecord, ActionState};
use super::worker_actions_executor;
use super::worker_actions_transitions;
use argo_memory::MemoryStore;
use argo_types::{ArgoError, ArgoResult, WorkspaceId};
use serde::Serialize;

#[derive(Debug, Serialize)]
struct DryRunPreview {
    dry_run: bool,
    action_id: String,
    would_execute: WouldExecute,
}

#[derive(Debug, Serialize)]
struct WouldExecute {
    action_type: String,
    current_state: String,
    next_state: String,
    requires_approval: bool,
    executor_kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    vertical_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    executor_error: Option<String>,
}

pub(crate) async fn preview(
    memory: &MemoryStore,
    workspace: WorkspaceId,
    action_id: &str,
    json: bool,
) -> ArgoResult<()> {
    let located = worker_actions_transitions::find_action(memory, workspace, action_id)
        .await?
        .ok_or_else(|| ArgoError::Provider(format!("no action with action_id '{action_id}'")))?;
    validate_would_execute(&located.record)?;
    let executor = resolve_executor_preview(&located.record.action_type);
    let preview = DryRunPreview {
        dry_run: true,
        action_id: located.record.action_id.clone(),
        would_execute: WouldExecute {
            action_type: located.record.action_type.clone(),
            current_state: located.record.state.clone(),
            next_state: "executing".to_string(),
            requires_approval: located.record.requires_approval,
            executor_kind: executor.kind,
            vertical_id: executor.vertical_id,
            executor_error: executor.error,
        },
    };
    print_preview(&preview, json);
    Ok(())
}

fn validate_would_execute(rec: &ActionRecord) -> ArgoResult<()> {
    let current = ActionState::parse(&rec.state).ok_or_else(|| {
        ArgoError::Provider(format!("action {}: unknown current state", rec.action_id))
    })?;
    if current == ActionState::Pending && rec.requires_approval {
        return Err(ArgoError::Provider(format!(
            "action {}: approval required (state pending, requires_approval=true); \
             run `argo worker actions approve {}` first",
            rec.action_id, rec.action_id
        )));
    }
    if !current.can_transition_to(ActionState::Executing) {
        return Err(ArgoError::Provider(format!(
            "action {}: illegal transition {} -> executing",
            rec.action_id,
            current.as_str()
        )));
    }
    Ok(())
}

struct ExecutorPreview {
    kind: String,
    vertical_id: Option<String>,
    error: Option<String>,
}

fn resolve_executor_preview(action_type: &str) -> ExecutorPreview {
    let dir = super::vertical::resolve_verticals_dir();
    let (registry, _errors) = match argo_vertical::load_dir(&dir) {
        Ok(loaded) => loaded,
        Err(e) => {
            return ExecutorPreview {
                kind: "unknown".to_string(),
                vertical_id: None,
                error: Some(format!("load vertical manifests: {e}")),
            };
        }
    };
    let found = registry.list().find_map(|(vid, loaded)| {
        let decl = loaded
            .manifest
            .actions
            .as_ref()
            .and_then(|a| a.actions.get(action_type))?;
        Some((
            vid.to_string(),
            loaded.manifest.clone(),
            decl.executor.kind_str().to_string(),
        ))
    });
    let Some((vertical_id, manifest, kind)) = found else {
        return ExecutorPreview {
            kind: "unknown".to_string(),
            vertical_id: None,
            error: Some(format!(
                "no loaded vertical declares action_type '{action_type}'"
            )),
        };
    };
    let error = worker_actions_executor::resolve_executor(&manifest, action_type)
        .err()
        .map(|e| e.to_string());
    ExecutorPreview {
        kind,
        vertical_id: Some(vertical_id),
        error,
    }
}

fn print_preview(preview: &DryRunPreview, json: bool) {
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(preview).unwrap_or_else(|_| "{}".into())
        );
        return;
    }
    println!("{} -> executing (dry-run)", preview.action_id);
    println!("  action_type  {}", preview.would_execute.action_type);
    println!("  executor     {}", preview.would_execute.executor_kind);
    if let Some(vertical_id) = preview.would_execute.vertical_id.as_deref() {
        println!("  vertical     {vertical_id}");
    }
    if let Some(error) = preview.would_execute.executor_error.as_deref() {
        println!("  executor_error {error}");
    }
}

#[cfg(test)]
#[path = "worker_actions_dry_run_tests.rs"]
mod tests;
