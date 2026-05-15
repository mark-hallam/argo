//! Action-level native-macOS computer-use executor adapter
//! (`mission-cu-native-macos-executor-dispatch`).
//!
//! Wires the deferred `ExecutorKind` dispatch arm from #941
//! (claude-computer-use-native-macos-poc). A vertical declaring
//! `executor.kind = "computer_use_macos"` resolves through here
//! into `argo-executor-computer-use-macos`.
//!
//! The macOS crate now implements the platform-neutral
//! `CuStepRunner` seam and runs the autonomous Anthropic CU loop.
//! `HumanHandoff` is preserved for macOS preflight gaps, where
//! missing Screen Recording / Accessibility permissions require
//! operator action before an autonomous run is possible.

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
    use argo_executor_computer_use_macos::{MacosCuError, MacosCuExecutor, MacosCuRunRequest};

    let config = build_config(decl);
    let exec = MacosCuExecutor::new(config).map_err(|e| match e {
        MacosCuError::NotMacos => {
            ArgoError::Provider("argo-executor-computer-use-macos requires macOS".into())
        }
        other => ArgoError::Provider(format!("computer-use-macos config rejected: {other}")),
    })?;

    if let Some(permission_summary) = run_preflight_summary(&exec)? {
        let bundle_path = build_bundle(action, cwd, decl, Some(&permission_summary))?;
        return Ok(ActionOutcome::HumanHandoff { bundle_path });
    }

    let bundle_path = bundle_path(action, cwd);
    let req = MacosCuRunRequest {
        action_id: action.action_id.clone(),
        system_prompt: build_system_prompt(action, decl),
        audit_dir: bundle_path.clone(),
        anthropic_api_key: anthropic_api_key()?,
    };
    let outcome = run_blocking(exec, req)?;
    map_macos_outcome(action, cwd, decl, outcome)
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
    if let Ok(base_url) = std::env::var("SKYCOT_CU_ANTHROPIC_BASE_URL") {
        cfg.anthropic_base_url = Some(base_url);
    }
    if let Ok(model) = std::env::var("SKYCOT_CU_ANTHROPIC_MODEL") {
        cfg.anthropic_model = model;
    }
    cfg
}

#[cfg(target_os = "macos")]
fn run_preflight_summary(
    exec: &argo_executor_computer_use_macos::MacosCuExecutor,
) -> ArgoResult<Option<String>> {
    let report = block_on_macos(exec.preflight())?;
    Ok(if report.all_granted() {
        None
    } else {
        Some(report.actionable_summary())
    })
}

#[cfg(target_os = "macos")]
fn anthropic_api_key() -> ArgoResult<String> {
    argo_config::cred::get("ANTHROPIC_API_KEY").map_err(|_| {
        ArgoError::Provider(
            "computer-use-macos executor not configured: set ANTHROPIC_API_KEY".into(),
        )
    })
}

#[cfg(target_os = "macos")]
fn run_blocking(
    exec: argo_executor_computer_use_macos::MacosCuExecutor,
    req: argo_executor_computer_use_macos::MacosCuRunRequest,
) -> ArgoResult<argo_executor_computer_use_macos::MacosCuOutcome> {
    let outcome = block_on_macos(exec.run(req))?;
    outcome.map_err(|e| ArgoError::Provider(e.to_string()))
}

#[cfg(target_os = "macos")]
fn block_on_macos<F>(future: F) -> ArgoResult<F::Output>
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
                    "computer-use-macos executor requires a multi-thread Tokio runtime \
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
                        "computer-use-macos executor could not build Tokio runtime: {e}"
                    ))
                })?;
            Ok(rt.block_on(future))
        }
    }
}

fn map_macos_outcome(
    action: &ActionRecord,
    cwd: &Path,
    decl: Option<&ActionDeclaration>,
    outcome: argo_executor_computer_use_macos::MacosCuOutcome,
) -> ArgoResult<ActionOutcome> {
    match outcome {
        argo_executor_computer_use_macos::MacosCuOutcome::Completed {
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
        argo_executor_computer_use_macos::MacosCuOutcome::HumanHandoff { reason, .. } => {
            let bundle_path = build_bundle(action, cwd, decl, Some(&reason))?;
            Ok(ActionOutcome::HumanHandoff { bundle_path })
        }
    }
}

fn bundle_path(action: &ActionRecord, cwd: &Path) -> std::path::PathBuf {
    cwd.join(".skycot")
        .join("outputs")
        .join(&action.job_id)
        .join("actions")
        .join(&action.action_id)
}

fn build_bundle(
    action: &ActionRecord,
    cwd: &Path,
    decl: Option<&ActionDeclaration>,
    permission_summary: Option<&str>,
) -> ArgoResult<std::path::PathBuf> {
    let dir = bundle_path(action, cwd);
    std::fs::create_dir_all(&dir).map_err(|e| {
        ArgoError::Provider(format!(
            "create per-action bundle dir {}: {e}",
            dir.display()
        ))
    })?;
    let mut body = format!(
        "# Action: `{}` (computer-use-macos)\n\n\
         _action_id: `{}`_  \n_job: `{}`_  \n\n\
         The native macOS computer-use executor could not start an \
         autonomous run and produced this handoff bundle. Carry the \
         action out via macOS yourself, then run \
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

fn build_system_prompt(action: &ActionRecord, decl: Option<&ActionDeclaration>) -> String {
    let payload_pretty =
        serde_json::to_string_pretty(&action.payload).unwrap_or_else(|_| "{}".to_string());
    let description = decl
        .map(|d| d.description.as_str())
        .filter(|s| !s.is_empty())
        .unwrap_or("(no description in manifest)");
    format!(
        "You are operating the local macOS desktop to carry out one Skycot action.\n\n\
         Action: `{}` on `{}` (item `{}`)\n\
         Description: {description}\n\n\
         Payload (JSON):\n{payload_pretty}\n\n\
         Use the `computer` tool to click, type, scroll, and press keys in the target app. \
         When the action is complete, emit `stop_success` with a concise confirmation. \
         If you cannot complete it, emit `stop_failure` with a one-line reason.",
        action.action_type,
        action.platform.as_deref().unwrap_or("(unspecified)"),
        action.item_id.as_deref().unwrap_or("(none)"),
    )
}

#[cfg(test)]
#[path = "worker_actions_executor_cu_macos_tests.rs"]
mod cu_macos_tests;
