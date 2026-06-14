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
    use argo_executor_computer_use_windows::{
        WindowsCuError, WindowsCuExecutor, WindowsCuRunRequest,
    };

    let config = build_config(decl);
    let exec = WindowsCuExecutor::new(config).map_err(|e| match e {
        WindowsCuError::NotWindows => {
            ArgoError::Provider("argo-executor-computer-use-windows requires Windows".into())
        }
        other => ArgoError::Provider(format!("computer-use-windows config rejected: {other}")),
    })?;

    // Preflight gaps (UAC / DPI / a11y) → honest handoff bundle; never start an
    // autonomous run on a degraded host.
    if let Some(summary) = run_preflight_summary(&exec) {
        let bundle_path = build_bundle(action, cwd, decl, Some(&summary))?;
        return Ok(ActionOutcome::HumanHandoff { bundle_path });
    }

    // Real model-in-loop run (win-cu-executor-windows-run-loop, DEC-054) —
    // replaces the old preflight+HumanHandoff stub. Risk-tiered floor
    // (DEC-051): the auto-dispatch path never pre-approves ACT ops
    // (`act_approved: false`), so the loop holds at the first committing op and
    // hands off; the operator's "Approve & run" is the only grant. Same
    // posture as the macOS native executor.
    let req = WindowsCuRunRequest {
        action_id: action.action_id.clone(),
        system_prompt: build_system_prompt(action, decl),
        audit_dir: bundle_path(action, cwd),
        anthropic_api_key: anthropic_api_key()?,
        act_approved: false,
    };
    let outcome = run_blocking(exec, req)?;
    map_windows_outcome(action, cwd, decl, outcome)
}

/// Per-action bundle dir (audit + handoff instructions live here).
/// Windows-gated: both callers (`dispatch_on_platform`'s windows arm and
/// `map_windows_outcome`) are `#[cfg(target_os = "windows")]`, so this is dead
/// code off-Windows (the windows-CU crate it serves is a windows-only dep).
#[cfg(target_os = "windows")]
fn bundle_path(action: &ActionRecord, cwd: &Path) -> std::path::PathBuf {
    cwd.join(".skycot")
        .join("outputs")
        .join(&action.job_id)
        .join("actions")
        .join(&action.action_id)
}

/// Map the executor's [`WindowsCuOutcome`] onto the dispatcher's
/// `ActionOutcome` — `Completed` auto-completes, `HumanHandoff` writes a bundle.
#[cfg(target_os = "windows")]
fn map_windows_outcome(
    action: &ActionRecord,
    cwd: &Path,
    decl: Option<&ActionDeclaration>,
    outcome: argo_executor_computer_use_windows::WindowsCuOutcome,
) -> ArgoResult<ActionOutcome> {
    use argo_executor_computer_use_windows::WindowsCuOutcome;
    match outcome {
        WindowsCuOutcome::Completed {
            summary,
            steps,
            wallclock_seconds,
        } => Ok(ActionOutcome::Completed {
            bundle_path: bundle_path(action, cwd),
            result_json: serde_json::json!({
                "outcome": "completed",
                "summary": summary,
                "steps": steps,
                "wallclock_seconds": wallclock_seconds,
            }),
        }),
        WindowsCuOutcome::HumanHandoff { reason, .. } => {
            let bundle_path = build_bundle(action, cwd, decl, Some(&reason))?;
            Ok(ActionOutcome::HumanHandoff { bundle_path })
        }
    }
}

#[cfg(target_os = "windows")]
fn build_system_prompt(action: &ActionRecord, decl: Option<&ActionDeclaration>) -> String {
    let payload_pretty =
        serde_json::to_string_pretty(&action.payload).unwrap_or_else(|_| "{}".to_string());
    let description = decl
        .map(|d| d.description.as_str())
        .filter(|s| !s.is_empty())
        .unwrap_or("(no description in manifest)");
    let item = action.item_id.as_deref().unwrap_or("-");
    format!(
        "You are operating the local Windows desktop to carry out one Skycot action.\n\n\
         Action: `{}` (item `{item}`)\n\
         Description: {description}\n\n\
         Payload (JSON):\n{payload_pretty}\n\n\
         Use the `computer` tool to click, type, scroll, and press keys in the target app. \
         When the action is complete, emit `stop_success` with a concise confirmation. \
         If you cannot complete it, emit `stop_failure` with a one-line reason.",
        action.action_type,
    )
}

#[cfg(target_os = "windows")]
fn anthropic_api_key() -> ArgoResult<String> {
    // tenant-unscoped: worker action handler called from the queue without a
    // tenant context yet (follow-up: pass via the WorkerAction envelope).
    argo_config::cred::get_unscoped("ANTHROPIC_API_KEY").map_err(|_| {
        ArgoError::Provider(
            "computer-use-windows executor not configured: set ANTHROPIC_API_KEY".into(),
        )
    })
}

#[cfg(target_os = "windows")]
fn run_blocking(
    exec: argo_executor_computer_use_windows::WindowsCuExecutor,
    req: argo_executor_computer_use_windows::WindowsCuRunRequest,
) -> ArgoResult<argo_executor_computer_use_windows::WindowsCuOutcome> {
    let outcome = block_on_windows(exec.run(req))?;
    outcome.map_err(|e| ArgoError::Provider(e.to_string()))
}

#[cfg(target_os = "windows")]
fn block_on_windows<F>(future: F) -> ArgoResult<F::Output>
where
    F: std::future::Future,
{
    match tokio::runtime::Handle::try_current() {
        Ok(handle) => {
            if matches!(
                handle.runtime_flavor(),
                tokio::runtime::RuntimeFlavor::MultiThread
            ) {
                Ok(tokio::task::block_in_place(|| handle.block_on(future)))
            } else {
                Err(ArgoError::Provider(
                    "computer-use-windows executor requires a multi-thread Tokio runtime \
                     or no active runtime"
                        .into(),
                ))
            }
        }
        Err(_) => {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|e| {
                    ArgoError::Provider(format!(
                        "computer-use-windows executor could not build Tokio runtime: {e}"
                    ))
                })?;
            Ok(rt.block_on(future))
        }
    }
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

// Windows-gated: its only caller, `map_windows_outcome`, is
// `#[cfg(target_os = "windows")]` (the windows-CU outcome type it maps is a
// windows-only dep), so this is dead code off-Windows. It touches only
// `std::fs` / `serde_json` and could compile anywhere — restoring cross-host
// mapper coverage is tracked in
// `win-cu-windows-types-cross-platform-for-mapper-coverage`.
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
    body.push_str(&serde_json::to_string_pretty(&action.payload).unwrap_or_else(|_| "{}".into()));
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
