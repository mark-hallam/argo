//! Action-level native-Windows computer-use executor adapter
//! (`G1-cu-native-windows-executor-dispatch`).
//!
//! Wires the deferred `ExecutorKind` dispatch arm from the
//! Windows POC crate (`mission-cu-native-windows-poc`). A
//! vertical declaring `executor.kind = "computer_use_windows"`
//! resolves through here into
//! `argo-executor-computer-use-windows`.
//!
//! ## Boundary (load-bearing)
//!
//! Mirrors the macOS (#955) and Linux dispatch adapters. The
//! Windows crate ships step primitives (`preflight`, `warm_up`,
//! `execute_step_recorded`, `WindowsCuOutcome`) but no full
//! Anthropic-CU `run()` loop. Wiring a full loop in this adapter
//! would duplicate logic that lives in
//! `argo_executor_computer_use::executor::run_loop` and exceed a
//! thin-adapter scope. This brief therefore lands the
//! ExecutorKind dispatch arm with a bundle-and-handoff shape: on
//! Windows the action reaches the Windows crate's preflight, a
//! per-action bundle is materialised on disk with any preflight
//! gaps surfaced, the operator drives the work offline (Notepad
//! or the configured target app) and runs
//! `argo worker actions complete <id>`.
//!
//! Follow-up brief: port the Anthropic-CU run-loop into the
//! adapter and swap the `HumanHandoff` outcome for a `Completed`
//! outcome on `stop_success`.
//!
//! ## Sensible deviation from the brief paths
//!
//! The brief authored against an older layout cited paths under
//! `crates/argo-runtime/src/handlers/...`. That handlers tree
//! does not exist in the current workspace; the worker-action
//! dispatch wiring lives in `crates/argo-cli/src/handlers/...`
//! alongside the macOS and Linux siblings. Adapter files,
//! `mod.rs`, and the `Cargo.toml` Windows-target dep are placed
//! there to keep symmetry with the existing native CU adapters.

use super::worker_actions::ActionRecord;
use super::worker_actions_executor::{ActionOutcome, Executor, ExecutorKind};
use argo_types::{ArgoError, ArgoResult};
use argo_vertical::ActionDeclaration;
use std::path::Path;

/// Adapter that bridges the trait-level `Executor` contract into
/// the Windows crate's step-primitives API. Constructed by
/// `resolve_executor` for any action whose manifest declares
/// `executor.kind = "computer_use_windows"`.
pub(crate) struct ComputerUseWindowsExecutor {
    declaration: Option<ActionDeclaration>,
}

impl ComputerUseWindowsExecutor {
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

impl Executor for ComputerUseWindowsExecutor {
    fn kind(&self) -> ExecutorKind {
        ExecutorKind::ComputerUseWindows
    }

    fn dispatch(&self, action: &ActionRecord, cwd: &Path) -> ArgoResult<ActionOutcome> {
        dispatch_on_platform(action, cwd, self.declaration.as_ref())
    }
}

/// Platform-gated dispatch entry point. On Windows this reaches
/// the Windows crate (preflight + bundle); on every other target
/// it returns a clean error so the operator sees the boundary.
#[cfg(target_os = "windows")]
fn dispatch_on_platform(
    action: &ActionRecord,
    cwd: &Path,
    decl: Option<&ActionDeclaration>,
) -> ArgoResult<ActionOutcome> {
    use argo_executor_computer_use_windows::{WindowsCuConfig, WindowsCuError, WindowsCuExecutor};

    let config = build_config(decl);
    let exec = WindowsCuExecutor::new(config).map_err(|e| match e {
        WindowsCuError::NotWindows => ArgoError::Provider(
            "argo-executor-computer-use-windows requires Windows".into(),
        ),
        other => ArgoError::Provider(format!(
            "computer-use-windows config rejected: {other}"
        )),
    })?;

    // Surface preflight gaps to the operator via the bundle so
    // missing UAC / DPI / a11y settings don't silently degrade
    // the run.
    let permission_summary = run_preflight_summary(&exec);

    let bundle_path = build_bundle(action, cwd, decl, permission_summary.as_deref())?;
    Ok(ActionOutcome::HumanHandoff { bundle_path })
}

#[cfg(not(target_os = "windows"))]
fn dispatch_on_platform(
    _action: &ActionRecord,
    _cwd: &Path,
    _decl: Option<&ActionDeclaration>,
) -> ArgoResult<ActionOutcome> {
    Err(ArgoError::Provider(
        "computer-use-windows requires Windows (target_os = \"windows\"); \
         this host cannot dispatch ExecutorKind::ComputerUseWindows — \
         use computer_use sandbox executor on non-Windows hosts"
            .into(),
    ))
}

#[cfg(target_os = "windows")]
fn build_config(
    decl: Option<&ActionDeclaration>,
) -> argo_executor_computer_use_windows::WindowsCuConfig {
    use argo_executor_computer_use_windows::WindowsCuConfig;
    let mut cfg = WindowsCuConfig::for_notepad_app();
    if let Some(d) = decl {
        if let Some(profile) = d.executor.profile() {
            cfg.allowed_apps.insert(profile.to_string());
            cfg.target_app = profile.to_string();
        }
    }
    cfg
}

#[cfg(target_os = "windows")]
fn run_preflight_summary(
    exec: &argo_executor_computer_use_windows::WindowsCuExecutor,
) -> Option<String> {
    let handle = tokio::runtime::Handle::try_current().ok()?;
    let report = tokio::task::block_in_place(|| handle.block_on(exec.preflight()));
    if report.all_passed() {
        None
    } else {
        Some(report.actionable_summary())
    }
}

#[cfg(target_os = "windows")]
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
        "# Action: `{}` (computer-use-windows)\n\n\
         _action_id: `{}`_  \n_job: `{}`_  \n\n\
         The native Windows computer-use executor's full Anthropic loop \
         is a follow-up brief; this run reached the Windows crate's \
         preflight check and produced this handoff bundle. \
         Carry the action out on Windows yourself, then run \
         `argo worker actions complete {} --note \"<summary>\"` to \
         record the result.\n\n",
        action.action_type, action.action_id, action.job_id, action.action_id,
    );
    if let Some(profile) = decl.and_then(|d| d.executor.profile()) {
        body.push_str(&format!("Target app: `{profile}`\n\n"));
    }
    if let Some(summary) = permission_summary {
        body.push_str("## Windows preflight\n\n```\n");
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
#[path = "worker_actions_executor_cu_windows_tests.rs"]
mod cu_windows_tests;
