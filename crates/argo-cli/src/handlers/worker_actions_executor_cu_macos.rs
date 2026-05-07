//! Action-level native-macOS computer-use executor adapter
//! (`mission-cu-native-macos-executor-dispatch`).
//!
//! Wires the deferred `ExecutorKind` dispatch arm from #941
//! (claude-computer-use-native-macos-poc). A vertical declaring
//! `executor.kind = "computer_use_macos"` resolves through here
//! into `argo-executor-computer-use-macos`.
//!
//! ## Boundary (load-bearing)
//!
//! The macOS crate from #941 ships *step primitives*
//! (`preflight`, `warm_up`, `execute_step_recorded`) but no
//! `run()` orchestrator. Wiring a full Anthropic CU loop in this
//! adapter would duplicate the loop logic that lives in
//! `argo_executor_computer_use::executor::run_loop` and exceed a
//! thin-adapter scope. This brief therefore lands the
//! ExecutorKind dispatch arm with an honest *bundle-and-handoff*
//! shape — the action reaches the macOS crate (preflight runs),
//! a per-action bundle is materialised on disk, the operator
//! drives the work via macOS Notes/Finder/Whatever and runs
//! `argo worker actions complete <id>`.
//!
//! Follow-up brief: port the run-loop into either the macOS
//! crate or the adapter, then swap the `HumanHandoff` outcome
//! for a `Completed` outcome on `stop_success`.

use super::worker_actions::ActionRecord;
use super::worker_actions_executor::{ActionOutcome, Executor, ExecutorKind};
use argo_types::{ArgoError, ArgoResult};
use argo_vertical::ActionDeclaration;
use std::path::Path;

/// Adapter that bridges the trait-level `Executor` contract into
/// the macOS crate's step-primitives API. Constructed by
/// `resolve_executor` for any action whose manifest declares
/// `executor.kind = "computer_use_macos"`.
pub(crate) struct ComputerUseMacOSExecutor {
    declaration: Option<ActionDeclaration>,
}

impl ComputerUseMacOSExecutor {
    /// Constructor used by `resolve_executor` — passes through
    /// the manifest declaration so dispatch can read profile +
    /// budget hints.
    pub(crate) fn with_declaration(decl: ActionDeclaration) -> Self {
        Self {
            declaration: Some(decl),
        }
    }

    /// Constructor for tests + the non-manifest dispatch path.
    #[allow(dead_code)]
    pub(crate) fn without_declaration() -> Self {
        Self { declaration: None }
    }
}

impl Executor for ComputerUseMacOSExecutor {
    fn kind(&self) -> ExecutorKind {
        ExecutorKind::ComputerUseMacOS
    }

    fn dispatch(&self, action: &ActionRecord, cwd: &Path) -> ArgoResult<ActionOutcome> {
        dispatch_on_platform(action, cwd, self.declaration.as_ref())
    }
}

/// Platform-gated dispatch entry point. On macOS this reaches
/// the macOS crate (preflight + bundle); on every other target
/// it returns a clean error so the operator sees the boundary.
#[cfg(target_os = "macos")]
fn dispatch_on_platform(
    action: &ActionRecord,
    cwd: &Path,
    decl: Option<&ActionDeclaration>,
) -> ArgoResult<ActionOutcome> {
    use argo_executor_computer_use_macos::{MacosCuError, MacosCuExecutor};

    let config = build_config(decl);
    let exec = MacosCuExecutor::new(config).map_err(|e| match e {
        MacosCuError::NotMacos => ArgoError::Provider(
            "argo-executor-computer-use-macos requires macOS".into(),
        ),
        other => ArgoError::Provider(format!(
            "computer-use-macos config rejected: {other}"
        )),
    })?;

    // Surface preflight gaps to the operator via the bundle so
    // missing macOS permissions don't silently degrade the run.
    let permission_summary = run_preflight_summary(&exec);

    let bundle_path = build_bundle(action, cwd, decl, permission_summary.as_deref())?;
    Ok(ActionOutcome::HumanHandoff { bundle_path })
}

#[cfg(not(target_os = "macos"))]
fn dispatch_on_platform(
    _action: &ActionRecord,
    _cwd: &Path,
    _decl: Option<&ActionDeclaration>,
) -> ArgoResult<ActionOutcome> {
    Err(ArgoError::Provider(
        "computer-use-macos requires macOS (target_os = \"macos\"); \
         this host cannot dispatch ExecutorKind::ComputerUseMacOS"
            .into(),
    ))
}

#[cfg(target_os = "macos")]
fn build_config(
    decl: Option<&ActionDeclaration>,
) -> argo_executor_computer_use_macos::MacosCuConfig {
    use argo_executor_computer_use_macos::MacosCuConfig;
    let mut cfg = MacosCuConfig::for_notes_app();
    if let Some(d) = decl {
        if let Some(profile) = d.executor.profile() {
            cfg.allowed_apps.insert(profile.to_string());
            cfg.target_app = profile.to_string();
        }
    }
    cfg
}

#[cfg(target_os = "macos")]
fn run_preflight_summary(
    exec: &argo_executor_computer_use_macos::MacosCuExecutor,
) -> Option<String> {
    let handle = tokio::runtime::Handle::try_current().ok()?;
    let report = tokio::task::block_in_place(|| handle.block_on(exec.preflight()));
    if report.all_granted() {
        None
    } else {
        Some(report.actionable_summary())
    }
}

fn build_bundle(
    action: &ActionRecord,
    cwd: &Path,
    decl: Option<&ActionDeclaration>,
    permission_summary: Option<&str>,
) -> ArgoResult<std::path::PathBuf> {
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
    let mut body = format!(
        "# Action: `{}` (computer-use-macos)\n\n\
         _action_id: `{}`_  \n_job: `{}`_  \n\n\
         The native macOS computer-use executor's full Anthropic loop \
         is a follow-up brief; this run reached the macOS crate's \
         preflight check and produced this handoff bundle. \
         Carry the action out via macOS yourself, then run \
         `argo worker actions complete {} --note \"<summary>\"` to \
         record the result.\n\n",
        action.action_type, action.action_id, action.job_id, action.action_id,
    );
    if let Some(profile) = decl.and_then(|d| d.executor.profile()) {
        body.push_str(&format!("Target app: `{profile}`\n\n"));
    }
    if let Some(summary) = permission_summary {
        body.push_str("## macOS permissions\n\n```\n");
        body.push_str(summary);
        body.push_str("\n```\n\n");
    }
    body.push_str("## Payload\n\n```json\n");
    body.push_str(
        &serde_json::to_string_pretty(&action.payload).unwrap_or_else(|_| "{}".into()),
    );
    body.push_str("\n```\n");
    std::fs::write(dir.join("instructions.md"), &body).map_err(|e| {
        ArgoError::Provider(format!(
            "write {}: {e}",
            dir.join("instructions.md").display()
        ))
    })?;
    Ok(dir)
}

#[cfg(test)]
#[path = "worker_actions_executor_cu_macos_tests.rs"]
mod cu_macos_tests;
