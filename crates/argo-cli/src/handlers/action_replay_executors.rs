//! Executor implementations for `argo action replay` bundles.
//!
//! Split from `action_replay.rs` to keep that file under the 500-LOC
//! ceiling. The parent module owns action lookup and orchestration;
//! this module owns the per-executor bundle logic and its file I/O.
use crate::cli::ActionReplayExecutor;
use crate::handlers::worker_actions::ActionRecord;
use argo_executor_computer_use::{
    ComputerUseExecutor, CuConfig, Outcome, ProfileRegistry, RunRequest,
};
use argo_types::{ArgoError, ArgoResult};
use argo_vertical::ActionDeclaration;
use serde_json::json;
use std::path::Path;

pub(super) async fn run_executor(
    executor: &ActionReplayExecutor,
    action: &ActionRecord,
    decl: &ActionDeclaration,
    replay_dir: &Path,
) -> ArgoResult<serde_json::Value> {
    match executor {
        ActionReplayExecutor::Human => run_human(action, replay_dir),
        ActionReplayExecutor::Api => Err(ArgoError::Provider(
            "action replay executor `api` is not implemented yet; pass --executor human to render a replay bundle".into(),
        )),
        ActionReplayExecutor::ComputerUse => run_computer_use(action, decl, replay_dir).await,
    }
}

fn run_human(action: &ActionRecord, replay_dir: &Path) -> ArgoResult<serde_json::Value> {
    std::fs::create_dir_all(replay_dir).map_err(|e| {
        ArgoError::Provider(format!(
            "create replay bundle dir {}: {e}",
            replay_dir.display()
        ))
    })?;
    write_text(
        &replay_dir.join("instructions.md"),
        &render_instructions(action),
    )?;
    write_text(&replay_dir.join("evidence.md"), &render_evidence(action))?;
    write_json(
        &replay_dir.join("action.json"),
        &serde_json::to_value(action).unwrap_or(json!({})),
    )?;
    Ok(json!({
        "outcome": "human_handoff",
        "executor": "human",
        "action_id": action.action_id,
        "source_state": action.state,
        "source_result": action.result,
        "bundle_path": replay_dir.to_string_lossy().to_string(),
    }))
}

async fn run_computer_use(
    action: &ActionRecord,
    decl: &ActionDeclaration,
    replay_dir: &Path,
) -> ArgoResult<serde_json::Value> {
    let config = CuConfig::from_env().map_err(|e| {
        ArgoError::Provider(format!(
            "computer-use replay not configured: {e}; set E2B_API_KEY and ANTHROPIC_API_KEY"
        ))
    })?;
    let mut registry = ProfileRegistry::load_from_path(&config.profiles_path)
        .map_err(|e| ArgoError::Provider(e.to_string()))?;
    if registry.is_empty() {
        registry = ProfileRegistry::with_builtin_marketplace_browser();
    }
    let profile_name = decl
        .executor
        .profile()
        .unwrap_or("marketplace_browser")
        .to_string();
    let req = RunRequest {
        action_id: action.action_id.clone(),
        job_id: action.job_id.clone(),
        tenant_id: "replay-tenant".into(),
        persona_id: "replay-persona".into(),
        profile_name,
        system_prompt: computer_use_prompt(action, decl),
        budget_usd: decl.executor.budget_usd(),
        bundle_dir: Some(replay_dir.to_path_buf()),
        session_id: None,
    };
    let outcome = ComputerUseExecutor::new(config, registry)
        .run(req)
        .await
        .map_err(ArgoError::from)?;
    Ok(match outcome {
        Outcome::Completed {
            result,
            bundle_dir,
            budget,
        } => json!({
            "outcome": "completed",
            "executor": "computer_use",
            "bundle_path": bundle_dir,
            "final_text": result.final_text,
            "actions_taken": result.actions_taken,
            "tokens_used": result.tokens_used,
            "budget": budget,
        }),
        Outcome::HumanHandoff {
            reason,
            bundle_dir,
            budget,
        } => json!({
            "outcome": "human_handoff",
            "executor": "computer_use",
            "bundle_path": bundle_dir,
            "reason": reason,
            "budget": budget,
        }),
    })
}

fn render_instructions(action: &ActionRecord) -> String {
    let payload = serde_json::to_string_pretty(&action.payload).unwrap_or_else(|_| "{}".into());
    format!(
        "# Replay action `{}`\n\n_action_id: `{}`_  \n_job: `{}`_  \n\n## Payload\n\n```json\n{}\n```\n",
        action.action_type, action.action_id, action.job_id, payload
    )
}

fn render_evidence(action: &ActionRecord) -> String {
    if action.evidence.is_empty() {
        return "_(no evidence attached to this action)_\n".into();
    }
    let mut out = String::from("# Evidence\n\n");
    for e in &action.evidence {
        out.push_str("- ");
        out.push_str(e);
        out.push('\n');
    }
    out
}

fn computer_use_prompt(action: &ActionRecord, decl: &ActionDeclaration) -> String {
    let payload = serde_json::to_string_pretty(&action.payload).unwrap_or_else(|_| "{}".into());
    format!(
        "Replay this SKYCOT action in a fresh sandbox.\n\nAction: `{}`\nDescription: {}\nPayload:\n{}",
        action.action_type, decl.description, payload
    )
}

fn write_text(path: &Path, body: &str) -> ArgoResult<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            ArgoError::Io(format!("create replay dir {}: {e}", parent.display()))
        })?;
    }
    std::fs::write(path, body)
        .map_err(|e| ArgoError::Io(format!("write {}: {e}", path.display())))
}

pub(super) fn write_json(path: &Path, value: &serde_json::Value) -> ArgoResult<()> {
    let body = serde_json::to_string_pretty(value)
        .map_err(|e| ArgoError::Provider(format!("serialize replay json: {e}")))?;
    write_text(path, &body)
}
