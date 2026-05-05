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
use argo_vertical::ActionDeclaration;
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
    let decl = lookup_action(vertical, action_type)?;
    match decl.executor.as_str() {
        "human" => Ok(Box::new(HumanExecutor::new(decl.clone()))),
        other => Err(ArgoError::Provider(format!(
            "executor '{other}' for action_type '{action_type}' is not yet implemented; \
             v1 supports `executor = \"human\"` only (browser/API land in later PRs)"
        ))),
    }
}

/// Shared lookup so resolve_executor + the test helpers don't
/// duplicate the "find the action_type in the manifest" logic.
fn lookup_action<'a>(
    vertical: &'a argo_vertical::VerticalManifest,
    action_type: &str,
) -> ArgoResult<&'a ActionDeclaration> {
    let actions = vertical.actions.as_ref().ok_or_else(|| {
        ArgoError::Provider(format!(
            "vertical '{}' has no [actions] section; cannot execute action_type '{action_type}'",
            vertical.vertical.id
        ))
    })?;
    actions.actions.get(action_type).ok_or_else(|| {
        ArgoError::Provider(format!(
            "vertical '{}' does not declare action_type '{action_type}'",
            vertical.vertical.id
        ))
    })
}

/// Execution-mode-v1 default. Renders the per-action bundle to
/// disk; the human operator carries the action out offline and
/// then runs `argo worker actions complete <id>` to record the
/// result.
///
/// When the manifest's `[actions.<name>]` declaration carries an
/// `instruction_template` (or a per-platform override in
/// `instruction_templates`), the executor renders that template
/// instead of the generic dump. See `pick_template` for the
/// platform-precedence rules.
pub(crate) struct HumanExecutor {
    declaration: Option<ActionDeclaration>,
}

impl HumanExecutor {
    pub(crate) fn new(declaration: ActionDeclaration) -> Self {
        Self {
            declaration: Some(declaration),
        }
    }

    /// Constructor used by `dispatch::invoke_executor_for` when
    /// it needs an executor without a manifest in hand (the
    /// existing call site already proves the action is
    /// declared, so this path stays conservative — falls back
    /// to the generic dump).
    #[allow(dead_code)]
    pub(crate) fn without_manifest() -> Self {
        Self { declaration: None }
    }
}

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
        let template_body = self
            .declaration
            .as_ref()
            .and_then(|d| pick_template(d, action.platform.as_deref()));
        let instructions = match template_body {
            Some(body) => render_with_template(action, body),
            None => render_instructions(action),
        };
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

/// Choose which instruction template to use for an action, in
/// the precedence order:
///   1. `instruction_templates.<platform>` (per-platform override)
///   2. `instruction_template` (default for this action_type)
///   3. `None` → executor falls back to the generic dump
pub(crate) fn pick_template<'a>(
    decl: &'a ActionDeclaration,
    platform: Option<&str>,
) -> Option<&'a str> {
    if let Some(p) = platform {
        if let Some(per_platform) = decl.instruction_templates.get(p) {
            return Some(per_platform.as_str());
        }
    }
    decl.instruction_template.as_deref()
}

/// Render an instruction template against the action's fields.
/// Tiny substitution engine — no Handlebars dep, just literal
/// `{{ key }}` and `{{ payload.X }}` lookups. Unknown keys
/// render as `(unset)` so operators see the gap rather than a
/// confusing literal.
pub(crate) fn render_with_template(action: &ActionRecord, body: &str) -> String {
    let mut out = String::with_capacity(body.len() + 64);
    let bytes = body.as_bytes();
    let mut cursor = 0;
    while cursor < bytes.len() {
        let Some(start) = body[cursor..].find("{{") else {
            out.push_str(&body[cursor..]);
            break;
        };
        out.push_str(&body[cursor..cursor + start]);
        let open = cursor + start;
        let Some(rel_end) = body[open + 2..].find("}}") else {
            // Unmatched `{{` — emit verbatim and stop.
            out.push_str(&body[open..]);
            break;
        };
        let close = open + 2 + rel_end;
        let key = body[open + 2..close].trim();
        let resolved = lookup_template_key(action, key);
        out.push_str(&resolved);
        cursor = close + 2;
    }
    out
}

fn lookup_template_key(action: &ActionRecord, key: &str) -> String {
    match key {
        "action_id" => action.action_id.clone(),
        "action_type" => action.action_type.clone(),
        "job_id" => action.job_id.clone(),
        "risk_level" => action.risk_level.clone(),
        "platform" => action.platform.clone().unwrap_or_else(|| "(unset)".into()),
        "item_id" => action.item_id.clone().unwrap_or_else(|| "(unset)".into()),
        "approved_by" => action
            .approved_by
            .clone()
            .unwrap_or_else(|| "(unset)".into()),
        "approved_at" => action
            .approved_at
            .clone()
            .unwrap_or_else(|| "(unset)".into()),
        other => {
            if let Some(rest) = other.strip_prefix("payload.") {
                return lookup_payload_field(&action.payload, rest);
            }
            "(unset)".to_string()
        }
    }
}

/// Lookup a dotted path inside the action's payload Value. Each
/// segment must resolve to a string-coercible value at the leaf;
/// missing segments return `(unset)`. Numbers + booleans render
/// via their canonical to_string.
fn lookup_payload_field(payload: &serde_json::Value, path: &str) -> String {
    let mut cur = payload;
    for seg in path.split('.') {
        match cur {
            serde_json::Value::Object(map) => match map.get(seg) {
                Some(v) => cur = v,
                None => return "(unset)".to_string(),
            },
            _ => return "(unset)".to_string(),
        }
    }
    match cur {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Null => "(unset)".to_string(),
        other => other.to_string(),
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
