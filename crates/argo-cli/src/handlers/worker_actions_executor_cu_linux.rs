//! Action-level native-Linux computer-use executor adapter
//! (`mission-cu-native-linux-executor-dispatch`).
//!
//! Wires the deferred `ExecutorKind` dispatch arm from #992
//! (claude-cu-native-linux-poc). A vertical declaring
//! `executor.kind = "computer_use_linux"` resolves through here
//! into `argo-executor-computer-use-linux`.
//!
//! ## Boundary (load-bearing)
//!
//! Mirrors the macOS adapter pattern (#955). The Linux crate from
//! #992 ships step primitives (`probe`, `screenshot`, `click`,
//! `key`, `type`, plus a small `Session::run_example`
//! orchestrator) but no full Anthropic-CU `run()` loop. Wiring a
//! full loop in this adapter would duplicate logic that lives in
//! `argo_executor_computer_use::executor::run_loop` and exceed a
//! thin-adapter scope. This brief therefore lands the
//! ExecutorKind dispatch arm with a bundle-and-handoff shape: on
//! Linux the action reaches the Linux crate's probe, a per-action
//! bundle is materialised on disk, the operator drives the work
//! offline (xdotool/scrot ready-to-run) and runs
//! `argo worker actions complete <id>`.
//!
//! Follow-up brief: `mission-cu-native-linux-run-loop` will port
//! the Anthropic-CU loop and swap the `HumanHandoff` outcome for
//! a `Completed` outcome on `stop_success`.
//!
//! ## Sensible deviation from the brief notes
//!
//! The brief mentions `LinuxCuExecutor::new() + preflight() +
//! handoff()` as the executor-crate primitives. The actual public
//! API exposes `LinuxComputerUse::new()` and `probe::probe()` (see
//! `crates/argo-executor-computer-use-linux/src/lib.rs`). The
//! functional intent is the same — host-support check before
//! handoff — so the adapter uses the real names. Wayland gap
//! (`scrot` is X11-only) is documented in #992; on Wayland the
//! probe reports `Unsupported("DISPLAY is unset; ...")` and the
//! summary is surfaced into the bundle.

use super::worker_actions::ActionRecord;
use super::worker_actions_executor::{ActionOutcome, Executor, ExecutorKind};
use argo_types::{ArgoError, ArgoResult};
use argo_vertical::ActionDeclaration;
use std::path::Path;

/// Adapter that bridges the trait-level `Executor` contract into
/// the Linux crate's step-primitives API. Constructed by
/// `resolve_executor` for any action whose manifest declares
/// `executor.kind = "computer_use_linux"`.
pub(crate) struct ComputerUseLinuxExecutor {
    declaration: Option<ActionDeclaration>,
}

impl ComputerUseLinuxExecutor {
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

impl Executor for ComputerUseLinuxExecutor {
    fn kind(&self) -> ExecutorKind {
        ExecutorKind::ComputerUseLinux
    }

    fn dispatch(&self, action: &ActionRecord, cwd: &Path) -> ArgoResult<ActionOutcome> {
        dispatch_on_platform(action, cwd, self.declaration.as_ref())
    }
}

/// Platform-gated dispatch entry point. On Linux this reaches the
/// Linux crate (probe + bundle); on every other target it returns
/// a clean error so the operator sees the boundary.
#[cfg(target_os = "linux")]
fn dispatch_on_platform(
    action: &ActionRecord,
    cwd: &Path,
    decl: Option<&ActionDeclaration>,
) -> ArgoResult<ActionOutcome> {
    use argo_executor_computer_use_linux::LinuxComputerUse;

    let exec = LinuxComputerUse::new();

    // Surface preflight gaps to the operator via the bundle so
    // missing xdotool/scrot/DISPLAY don't silently degrade the run.
    let permission_summary = run_preflight_summary(&exec);

    let bundle_path = build_bundle(action, cwd, decl, permission_summary.as_deref())?;
    Ok(ActionOutcome::HumanHandoff { bundle_path })
}

#[cfg(not(target_os = "linux"))]
fn dispatch_on_platform(
    _action: &ActionRecord,
    _cwd: &Path,
    _decl: Option<&ActionDeclaration>,
) -> ArgoResult<ActionOutcome> {
    Err(ArgoError::Provider(
        "computer-use-linux requires Linux (target_os = \"linux\"); \
         this host cannot dispatch ExecutorKind::ComputerUseLinux — \
         use computer_use sandbox executor on non-Linux hosts"
            .into(),
    ))
}

#[cfg(target_os = "linux")]
fn run_preflight_summary(
    exec: &argo_executor_computer_use_linux::LinuxComputerUse,
) -> Option<String> {
    use argo_executor_computer_use_linux::ProbeStatus;
    let report = exec.probe();
    match &report.status {
        ProbeStatus::Supported => None,
        ProbeStatus::Unsupported(reason) => Some(format!(
            "xdotool={} scrot={} DISPLAY={} — {reason}",
            report.xdotool, report.scrot, report.x11_display,
        )),
    }
}

#[cfg(target_os = "linux")]
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
        "# Action: `{}` (computer-use-linux)\n\n\
         _action_id: `{}`_  \n_job: `{}`_  \n\n\
         The native Linux computer-use executor's full Anthropic loop \
         is a follow-up brief; this run reached the Linux crate's \
         probe check and produced this handoff bundle. \
         Carry the action out via xdotool/scrot yourself, then run \
         `argo worker actions complete {} --note \"<summary>\"` to \
         record the result.\n\n",
        action.action_type, action.action_id, action.job_id, action.action_id,
    );
    if let Some(profile) = decl.and_then(|d| d.executor.profile()) {
        body.push_str(&format!("Target app: `{profile}`\n\n"));
    }
    if let Some(summary) = permission_summary {
        body.push_str("## Linux X11 preflight\n\n```\n");
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
#[path = "worker_actions_executor_cu_linux_tests.rs"]
mod cu_linux_tests;
