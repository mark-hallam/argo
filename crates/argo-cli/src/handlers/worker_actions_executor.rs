//! Worker v3 execution-mode executors (#731 + Stream A
//! `#A-executor-trait`).
//!
//! Layer between `argo worker actions execute <id>` (which
//! advances state) and the eventual carry-out of an action.
//!
//! Today's fully-wired impl is `HumanExecutor` — it materialises
//! a per-action artifact bundle under
//! `.skycot/outputs/<job_id>/actions/<action_id>/` so the
//! operator has everything they need (payload, cited evidence,
//! a structured JSON copy) on disk; the operator carries the
//! action out offline and runs `argo worker actions complete
//! <id>` when finished.
//!
//! Stream A adds three placeholder impls (`McpExecutor`,
//! `ComputerUseExecutor`, `ApiExecutor` in
//! `worker_actions_executor_stubs`) that return clean
//! not-yet-implemented errors pointing at the brief that swaps
//! each stub for a real impl. As Streams B / C / H land, those
//! stubs become real without reshaping the dispatch shim.
//!
//! The manifest's `[actions.<name>].executor` declaration picks
//! the impl. Both the legacy string form (`executor = "human"`)
//! and the tagged-table form (`executor = { kind = "...",
//! profile = "...", budget_usd = 0.50 }`) parse — see
//! `argo_vertical::ExecutorDeclaration`.

use super::worker_actions::ActionRecord;
use super::worker_actions_executor_code::CodeExecutor;
use super::worker_actions_executor_document::DocumentExecutor;
use super::worker_actions_executor_stubs::{
    ApiExecutor, ComputerUseExecutor, FilesystemExecutor, McpExecutor,
};
use argo_types::{ArgoError, ArgoResult};
use argo_vertical::ActionDeclaration;
use std::path::{Path, PathBuf};

/// Stable identifier for which executor family handles an
/// action. Each variant maps to one `Executor` impl.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExecutorKind {
    /// Renders an artifact bundle on disk; the operator carries
    /// the action out offline and runs `argo worker actions
    /// complete <id>` when finished. Today's only fully-wired
    /// executor.
    Human,
    /// Calls a registered MCP server's tool (gmail, slack, etc.).
    /// Wired by Stream `#C-mcp-wire`.
    Mcp,
    /// Runs the Anthropic computer-use loop in an `argo-sandbox`
    /// (E2B Firecracker microVM). Wired by Stream
    /// `#B-computer-use-executor`.
    ComputerUse,
    /// Calls a registered REST API binding directly. Wired by
    /// Stream H connectors that ship as `ApiExecutor` flavours.
    Api,
    /// Sandboxed filesystem operations rooted at `worker.fs_root`.
    /// Wired by `claude-filesystem-executor` (V1.2 capability).
    Filesystem,
    /// Sandboxed Python / Node code execution. Wired by
    /// `claude-code-execution-executor` (V1.2 capability).
    CodeExecution,
    /// Document synthesis (markdown / CSV / PDF / slides). Wired by
    /// `claude-document-creation-executor` (V1.2 capability).
    DocumentCreation,
}

impl ExecutorKind {
    /// Parse from the manifest-level kind string. Mirrors the
    /// legacy `executor = "..."` form and the tagged
    /// `kind = "..."` form.
    fn parse(s: &str) -> ArgoResult<Self> {
        match s {
            "human" => Ok(Self::Human),
            "mcp" => Ok(Self::Mcp),
            "computer_use" => Ok(Self::ComputerUse),
            "api" => Ok(Self::Api),
            "filesystem" => Ok(Self::Filesystem),
            "code_execution" => Ok(Self::CodeExecution),
            "document_creation" => Ok(Self::DocumentCreation),
            other => Err(ArgoError::Provider(format!(
                "unknown executor kind '{other}' — \
                 valid: human | mcp | computer_use | api | filesystem | code_execution | document_creation"
            ))),
        }
    }
}

/// Outcome returned from an `Executor::dispatch` call. The
/// dispatch shim uses the variant to decide whether to leave the
/// action at `executing` (HumanHandoff) or auto-complete with the
/// embedded result (Completed).
#[derive(Debug)]
pub(crate) enum ActionOutcome {
    /// The executor rendered a per-action bundle to disk; the
    /// human operator will do the work offline and run
    /// `argo worker actions complete <id> --result-json '...'`
    /// when finished. The ledger row stays at `executing`.
    HumanHandoff { bundle_path: PathBuf },
    /// The executor did the work end-to-end. The dispatch shim
    /// auto-completes the action: writes a fresh
    /// `executing → completed` row stamped with the embedded
    /// `result_json`. The chain follow-up emitter (#777/#778)
    /// fires off this auto-completion the same way it does for
    /// operator-driven completions.
    #[allow(dead_code)] // wired in Stream B / Stream C; unused by HumanExecutor today
    Completed {
        bundle_path: PathBuf,
        result_json: serde_json::Value,
    },
}

/// Per-action pre-flight info. Today purely descriptive; Stream
/// H will read the `kind` to decide whether the action can clear
/// auto-approval. Implementors that override `preflight` may
/// fill the struct with the executor's own kind; the default
/// impl in `Executor::preflight` builds one from `kind()`.
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)] // consumed by Stream H auto-approval; default-built today
pub(crate) struct Preflight {
    pub kind: ExecutorKind,
}

/// Executor abstraction. Today's fully-wired impl is
/// `HumanExecutor`; the three placeholder impls (`McpExecutor`,
/// `ComputerUseExecutor`, `ApiExecutor`) live in
/// `worker_actions_executor_stubs.rs` and return a clean
/// not-yet-implemented error from `dispatch()` until Streams B /
/// C / H wire each.
///
/// Implementors run AFTER the state transition has been
/// persisted (the row is already `executing`), so the
/// implementor's job is purely to do (or prep) the work, not to
/// validate state.
pub(crate) trait Executor {
    /// Stable kind tag for this executor. Used by Stream H's
    /// auto-approval policy (per-kind budget rules) and by tests
    /// that round-trip the trait.
    fn kind(&self) -> ExecutorKind;

    /// Optional pre-flight validation. Default: pass-through.
    /// Implementors override when they need resource checks
    /// (token quota, sandbox quota, declared MCP server
    /// availability) BEFORE the executing-state row is written.
    /// A returned error halts dispatch with no side effect on
    /// the ledger.
    #[allow(dead_code)] // wired by Stream B / Stream C; default impl is fine for Human
    fn preflight(&self, _action: &ActionRecord) -> ArgoResult<Preflight> {
        Ok(Preflight { kind: self.kind() })
    }

    /// Carry out (or prep) the action. Returns an
    /// `ActionOutcome` the dispatch shim uses to decide whether
    /// to leave the row at `executing` or auto-complete it.
    fn dispatch(&self, action: &ActionRecord, cwd: &Path) -> ArgoResult<ActionOutcome>;
}

/// Resolve the executor declared by the manifest for this
/// action_type. Returns the boxed trait object so dispatch stays
/// generic. The placeholder impls (`Mcp` / `ComputerUse` / `Api`)
/// resolve cleanly here — they fail at `dispatch()` time with a
/// useful error so the operator sees the gap.
pub(crate) fn resolve_executor(
    vertical: &argo_vertical::VerticalManifest,
    action_type: &str,
) -> ArgoResult<Box<dyn Executor>> {
    let decl = lookup_action(vertical, action_type)?;
    let kind = ExecutorKind::parse(decl.executor.kind_str())?;
    let exec: Box<dyn Executor> = match kind {
        ExecutorKind::Human => Box::new(HumanExecutor::new(decl.clone())),
        ExecutorKind::Mcp => {
            let servers = vertical
                .mcp_servers
                .as_ref()
                .map(|s| s.servers.clone().into_iter().collect())
                .unwrap_or_default();
            Box::new(McpExecutor::with_manifest(decl.clone(), servers))
        }
        ExecutorKind::ComputerUse => {
            Box::new(ComputerUseExecutor::with_declaration(decl.clone()))
        }
        ExecutorKind::Api => Box::new(ApiExecutor::new()),
        ExecutorKind::Filesystem => Box::new(FilesystemExecutor::new()),
        ExecutorKind::CodeExecution => Box::new(CodeExecutor::new()),
        ExecutorKind::DocumentCreation => Box::new(DocumentExecutor::new()),
    };
    Ok(exec)
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
    /// existing call site already proves the action is declared,
    /// so this path stays conservative — falls back to the
    /// generic dump).
    #[allow(dead_code)]
    pub(crate) fn without_manifest() -> Self {
        Self { declaration: None }
    }
}

impl Executor for HumanExecutor {
    fn kind(&self) -> ExecutorKind {
        ExecutorKind::Human
    }

    fn dispatch(&self, action: &ActionRecord, cwd: &Path) -> ArgoResult<ActionOutcome> {
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
        Ok(ActionOutcome::HumanHandoff { bundle_path: dir })
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
pub(super) mod worker_actions_executor_tests;

#[cfg(test)]
#[path = "worker_actions_executor_stream_a_tests.rs"]
mod stream_a_tests;
