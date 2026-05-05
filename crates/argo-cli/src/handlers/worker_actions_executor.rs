//! Worker v3 execution-mode executors (#731).
//!
//! Layer between `argo worker actions execute <id>` (which
//! advances state) and the eventual carry-out of an action. The
//! design doc §7 calls for a trait so #732+ can layer on
//! browser, API, or marketplace-SDK executors without touching
//! the OS state machine.
//!
//! v1 ships a single executor: `HumanExecutor`. Its job is NOT
//! to carry out the action — that's done by a human, offline.
//! Instead it materialises a per-action artifact bundle under
//! `.skycot/outputs/<job_id>/actions/<action_id>/` so the
//! operator has everything they need (the action's payload,
//! cited evidence, a structured JSON copy) on disk.
//!
//! The trait stays small + future-friendly: an executor is a
//! function from (`ActionRecord`, `cwd`) → `Result<()>`. Future
//! executors (e.g. `BrowserExecutor` driving a real marketplace
//! UI) will share the same surface, called by the same dispatch
//! shim, but with side-effects in the world rather than just on
//! disk.
//!
//! Concretely an "executor" decision is taken by inspecting the
//! manifest's `[actions.<name>].executor` declaration. v1 only
//! recognises `executor = "human"`. Anything else returns an
//! error so the operator sees the gap rather than the OS
//! silently no-op'ing.

use super::worker_actions::ActionRecord;
use argo_types::{ArgoError, ArgoResult};
use std::path::{Path, PathBuf};

/// Executor abstraction. v1 has a single implementor
/// (`HumanExecutor`) but the trait stays around so #732+ can
/// add browser / API / marketplace-SDK executors without
/// reshaping the dispatch shim.
///
/// Implementors run AFTER the state transition has been
/// persisted (the row is already `executing`), so the
/// implementor's job is purely to do (or prep) the work, not
/// to validate state.
pub(crate) trait Executor {
    /// Carry out (or prep) the action. Return the path to the
    /// per-action artifact bundle so the dispatch shim can
    /// surface it in CLI output.
    fn execute(&self, action: &ActionRecord, cwd: &Path) -> ArgoResult<PathBuf>;
}

/// Resolve the executor declared by the manifest for this
/// action_type. Returns the boxed trait object so dispatch
/// stays generic. Errors when the manifest doesn't declare an
/// executor (or declares one v1 doesn't implement).
pub(crate) fn resolve_executor(
    vertical: &argo_vertical::VerticalManifest,
    action_type: &str,
) -> ArgoResult<Box<dyn Executor>> {
    let actions = vertical.actions.as_ref().ok_or_else(|| {
        ArgoError::Provider(format!(
            "vertical '{}' has no [actions] section; cannot execute action_type '{action_type}'",
            vertical.vertical.id
        ))
    })?;
    let decl = actions.actions.get(action_type).ok_or_else(|| {
        ArgoError::Provider(format!(
            "vertical '{}' does not declare action_type '{action_type}'",
            vertical.vertical.id
        ))
    })?;
    match decl.executor.as_str() {
        "human" => Ok(Box::new(HumanExecutor)),
        other => Err(ArgoError::Provider(format!(
            "executor '{other}' for action_type '{action_type}' is not yet implemented; \
             v1 supports `executor = \"human\"` only (browser/API land in #732+)"
        ))),
    }
}

/// Execution-mode-v1 default. Renders the per-action bundle to
/// disk; the human operator carries the action out offline and
/// then runs `argo worker actions complete <id>` to record the
/// result.
pub(crate) struct HumanExecutor;

impl Executor for HumanExecutor {
    fn execute(&self, action: &ActionRecord, cwd: &Path) -> ArgoResult<PathBuf> {
        let dir = cwd
            .join(".skycot")
            .join("outputs")
            .join(&action.job_id)
            .join("actions")
            .join(&action.action_id);
        std::fs::create_dir_all(&dir).map_err(|e| {
            ArgoError::Provider(format!(
                "create per-action bundle dir {}: {e}",
                dir.display()
            ))
        })?;
        let instructions = render_instructions(action);
        let evidence = render_evidence(action);
        write(&dir.join("instructions.md"), &instructions)?;
        write(&dir.join("evidence.md"), &evidence)?;
        write(
            &dir.join("action.json"),
            &serde_json::to_string_pretty(action).unwrap_or_else(|_| "{}".into()),
        )?;
        Ok(dir)
    }
}

/// Render `instructions.md` — the operator-facing markdown brief.
/// v1 is intentionally simple: action_type, platform, item, the
/// payload as JSON, and a "next step" footer pointing back at
/// `argo worker actions complete`. #732 layers on per-action-
/// type templates so e.g. `post_listing` produces a marketplace-
/// specific checklist instead of the generic dump.
pub(crate) fn render_instructions(action: &ActionRecord) -> String {
    let platform = action
        .platform
        .as_deref()
        .map(|p| format!(" on `{p}`"))
        .unwrap_or_default();
    let item = action
        .item_id
        .as_deref()
        .map(|i| format!(" (item `{i}`)"))
        .unwrap_or_default();
    let mut out = format!(
        "# Action: `{}`{platform}{item}\n\n\
         _action_id: `{}`_  \n\
         _job: `{}`_  \n\
         _risk: {}_  \n\n",
        action.action_type, action.action_id, action.job_id, action.risk_level,
    );
    if action.requires_approval {
        match (action.approved_by.as_deref(), action.approved_at.as_deref()) {
            (Some(by), Some(at)) => {
                out.push_str(&format!("**Approved** by `{by}` at `{at}`. Proceed.\n\n"));
            }
            _ => {
                out.push_str(
                    "**⚠️ Approval-gated action**. The OS expected approval before \
                              `executing` — check `argo worker actions status` for the trail.\n\n",
                );
            }
        }
    } else {
        out.push_str("Autonomous-safe action. No approval was required.\n\n");
    }
    out.push_str("## Payload\n\n```json\n");
    out.push_str(
        &serde_json::to_string_pretty(&action.payload).unwrap_or_else(|_| "{}".to_string()),
    );
    out.push_str("\n```\n\n");
    out.push_str(
        "## Next step\n\n\
         Carry the action out offline. When complete, run:\n\n\
         ```\n\
         argo worker actions complete <action_id> --note \"<short summary>\" \
         [--result-json '{\"key\": \"value\"}']\n\
         ```\n\n\
         If something blocks the action, run `argo worker actions fail <action_id> \
         --reason \"<why>\"` instead.\n",
    );
    out
}

/// Render `evidence.md` — the cited research backing this
/// action. Empty-state placeholder when the action carried no
/// evidence (legitimate for autonomous-safe actions where
/// research wasn't needed).
pub(crate) fn render_evidence(action: &ActionRecord) -> String {
    let mut out = format!(
        "# Evidence for action `{}`\n\n\
         _action_id: `{}`_  \n_job: `{}`_  \n\n",
        action.action_type, action.action_id, action.job_id,
    );
    if action.evidence.is_empty() {
        out.push_str("_(no evidence attached to this action)_\n");
        return out;
    }
    out.push_str(&format!(
        "{} source(s) cited at emission time:\n\n",
        action.evidence.len()
    ));
    for url in &action.evidence {
        out.push_str(&format!("- {url}\n"));
    }
    out
}

fn write(path: &Path, body: &str) -> ArgoResult<()> {
    std::fs::write(path, body)
        .map_err(|e| ArgoError::Provider(format!("write {}: {e}", path.display())))
}

#[cfg(test)]
#[path = "worker_actions_executor_tests.rs"]
mod tests;
