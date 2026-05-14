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
use super::worker_actions_executor_cu_linux::ComputerUseLinuxExecutor;
use super::worker_actions_executor_cu_macos::ComputerUseMacOSExecutor;
use super::worker_actions_executor_cu_windows::ComputerUseWindowsExecutor;
use super::worker_actions_executor_document::DocumentExecutor;
use super::worker_actions_executor_document_csv::DocumentCsvExecutor;
use super::worker_actions_executor_document_gsuite::DocumentGsuiteExecutor;
use super::worker_actions_executor_stubs::{
    ApiExecutor, BrowserExecutor, ComputerUseExecutor, FilesystemExecutor, McpExecutor,
};
use super::worker_actions_executor_timing::TimedExecutor;
use argo_types::{ArgoError, ArgoResult};
use argo_vertical::ActionDeclaration;
use std::path::{Path, PathBuf};

// Bundle-rendering helpers live in the sibling module
// `worker_actions_executor_render`. Re-exported here so the
// `HumanExecutor::dispatch` body and the in-tree test module
// (`worker_actions_executor_tests.rs`, which does
// `use super::*;`) can keep their pre-split call sites unchanged.
pub(crate) use super::worker_actions_executor_render::{
    pick_template, render_evidence, render_instructions, render_with_template,
};

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
    /// Native macOS computer-use via `screencapture` + `osascript`.
    /// Wired by `mission-cu-native-macos-executor-dispatch`
    /// (follow-up to #941 claude-computer-use-native-macos-poc).
    /// Platform-gated: dispatch on non-macOS returns a clean
    /// Provider error so manifests that pin `computer_use_macos`
    /// fail loudly on Linux/Windows hosts.
    ComputerUseMacOS,
    /// Native Linux computer-use via `xdotool` + `scrot` against
    /// an X11 desktop. Wired by
    /// `mission-cu-native-linux-executor-dispatch` (follow-up to
    /// #992 claude-cu-native-linux-poc). Platform-gated: dispatch
    /// on non-Linux returns a clean Provider error so manifests
    /// that pin `computer_use_linux` fail loudly on macOS/Windows
    /// hosts. Full Anthropic run-loop port is a follow-up brief;
    /// v1 lands as HumanHandoff with a bundle.
    ComputerUseLinux,
    /// Native Windows computer-use via `SendInput` + `BitBlt` +
    /// `IUIAutomation`. Wired by
    /// `G1-cu-native-windows-executor-dispatch` (follow-up to the
    /// Windows POC crate `mission-cu-native-windows-poc`).
    /// Platform-gated: dispatch on non-Windows returns a clean
    /// Provider error so manifests that pin `computer_use_windows`
    /// fail loudly on macOS/Linux hosts. Full Anthropic run-loop
    /// port is a follow-up brief; v1 lands as HumanHandoff with a
    /// bundle.
    ComputerUseWindows,
    /// Calls a registered REST API binding directly. Wired by
    /// Stream H connectors that ship as `ApiExecutor` flavours.
    Api,
    /// Drives a stateful headless browser via the existing 16-tool
    /// browser MCP server. Wired by
    /// `mission-wire-browser-action-executor` for autonomous
    /// posting / form-fill flows on Marketplace, eBay, and similar
    /// SaaS surfaces. Sticky sessions persist login + cookies per
    /// `(tenant, worker)`.
    Browser,
    /// Sandboxed filesystem operations rooted at `worker.fs_root`.
    /// Wired by `claude-filesystem-executor` (V1.2 capability).
    Filesystem,
    /// Sandboxed Python / Node code execution. Wired by
    /// `claude-code-execution-executor` (V1.2 capability).
    CodeExecution,
    /// Document synthesis (markdown / CSV / PDF / slides). Wired by
    /// `claude-document-creation-executor` (V1.2 capability).
    DocumentCreation,
    /// Typed RFC 4180 CSV writer with column-schema + record-level
    /// rejection. Wired by
    /// `mission-document-creation-executor-csv`. Distinct from
    /// `DocumentCreation` — that path's CSV backend is a thin string
    /// primitive; this one carries the typed schema + sha256-stamped
    /// report.
    DocumentCsv,
    /// Google Workspace draft-document writer (Docs / Sheets / Slides
    /// via Drive + per-API `batchUpdate`). Wired by
    /// `mission-doc-executor-gsuite`. Service-account OAuth2; env-
    /// indirect creds; drafts-only by design.
    DocumentGsuite,
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
            "computer_use_macos" | "computer-use-macos" => Ok(Self::ComputerUseMacOS),
            "computer_use_linux" | "computer-use-linux" => Ok(Self::ComputerUseLinux),
            "computer_use_windows" | "computer-use-windows" => Ok(Self::ComputerUseWindows),
            "api" => Ok(Self::Api),
            "browser" => Ok(Self::Browser),
            "filesystem" => Ok(Self::Filesystem),
            "code_execution" => Ok(Self::CodeExecution),
            "document_creation" => Ok(Self::DocumentCreation),
            "document_csv" | "document-csv" => Ok(Self::DocumentCsv),
            "document_gsuite" | "document-gsuite" => Ok(Self::DocumentGsuite),
            other => Err(ArgoError::Provider(format!(
                "unknown executor kind '{other}' — \
                 valid: human | mcp | computer_use | computer_use_macos | \
                 computer_use_linux | computer_use_windows | api | browser | \
                 filesystem | code_execution | document_creation | document_csv | \
                 document_gsuite"
            ))),
        }
    }

    /// Stable lowercase tag used by the latency tracker as the
    /// histogram key. Mirrors the manifest spelling so persisted
    /// rows stay readable in raw SQL.
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Human => "human",
            Self::Mcp => "mcp",
            Self::ComputerUse => "computer_use",
            Self::ComputerUseMacOS => "computer_use_macos",
            Self::ComputerUseLinux => "computer_use_linux",
            Self::ComputerUseWindows => "computer_use_windows",
            Self::Api => "api",
            Self::Browser => "browser",
            Self::Filesystem => "filesystem",
            Self::CodeExecution => "code_execution",
            Self::DocumentCreation => "document_creation",
            Self::DocumentCsv => "document_csv",
            Self::DocumentGsuite => "document_gsuite",
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
/// action_type. The returned executor is wrapped in a
/// [`TimedExecutor`] so every dispatch records its wall-clock
/// duration into `argo_store::latency_tracker::global()` (Stream
/// `claude-latency-percentiles-tracking`).
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
        ExecutorKind::ComputerUseMacOS => {
            Box::new(ComputerUseMacOSExecutor::with_declaration(decl.clone()))
        }
        ExecutorKind::ComputerUseLinux => {
            Box::new(ComputerUseLinuxExecutor::with_declaration(decl.clone()))
        }
        ExecutorKind::ComputerUseWindows => {
            Box::new(ComputerUseWindowsExecutor::with_declaration(decl.clone()))
        }
        ExecutorKind::Api => Box::new(ApiExecutor::with_manifest(decl.clone())),
        ExecutorKind::Browser => Box::new(BrowserExecutor::with_manifest(decl.clone())),
        ExecutorKind::Filesystem => Box::new(FilesystemExecutor::new()),
        ExecutorKind::CodeExecution => Box::new(CodeExecutor::new()),
        ExecutorKind::DocumentCreation => Box::new(DocumentExecutor::new()),
        ExecutorKind::DocumentCsv => Box::new(DocumentCsvExecutor::new()),
        ExecutorKind::DocumentGsuite => Box::new(DocumentGsuiteExecutor::new()),
    };
    Ok(Box::new(TimedExecutor::new(exec)))
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

fn write(path: &Path, body: &str) -> ArgoResult<()> {
    std::fs::write(path, body)
        .map_err(|e| ArgoError::Io(format!("write {}: {e}", path.display())))
}

#[cfg(test)]
#[path = "worker_actions_executor_tests.rs"]
pub(super) mod worker_actions_executor_tests;

#[cfg(test)]
#[path = "worker_actions_executor_stream_a_tests.rs"]
mod stream_a_tests;
