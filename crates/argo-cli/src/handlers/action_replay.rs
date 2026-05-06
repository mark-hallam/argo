//! `argo action replay` — read-only action replay bundles.
use crate::cli::{ActionCmd, ActionReplayExecutor};
use crate::handlers::worker_actions::ActionRecord;
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_executor_computer_use::{
    ComputerUseExecutor, CuConfig, Outcome, ProfileRegistry, RunRequest,
};
use argo_memory::{MemoryFile, MemoryStore};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, WorkspaceId};
use argo_vertical::{ActionDeclaration, LoadedVertical};
use chrono::Utc;
use serde_json::json;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub(crate) async fn run(cmd: ActionCmd) -> ArgoResult<()> {
    match cmd {
        ActionCmd::Replay {
            action_id,
            executor,
            dry_run,
            bundle_dir,
        } => replay(&action_id, executor, dry_run, bundle_dir).await,
    }
}

async fn replay(
    action_id: &str,
    executor: Option<ActionReplayExecutor>,
    dry_run: bool,
    bundle_dir: Option<PathBuf>,
) -> ArgoResult<()> {
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Provider(format!("cwd: {e}")))?;
    let (memory, workspace) = open_workspace_memory(&cwd).await?;
    let verticals_dir = crate::handlers::vertical::resolve_verticals_dir();
    replay_in_workspace(
        action_id,
        executor,
        dry_run,
        bundle_dir,
        ReplayWorkspace {
            cwd: &cwd,
            memory: &memory,
            workspace,
            verticals_dir: &verticals_dir,
        },
    )
    .await
}

async fn replay_in_workspace(
    action_id: &str,
    executor: Option<ActionReplayExecutor>,
    dry_run: bool,
    bundle_dir: Option<PathBuf>,
    ctx: ReplayWorkspace<'_>,
) -> ArgoResult<()> {
    let located = find_action(ctx.memory, ctx.workspace, action_id, ctx.verticals_dir)
        .await?
        .ok_or_else(|| ArgoError::Provider(format!("no action with action_id '{action_id}'")))?;
    if located.record.state != "completed" {
        return Err(ArgoError::Provider(format!(
            "action {action_id} is in state '{}', not 'completed'; replay only applies to completed actions",
            located.record.state
        )));
    }
    let decl = action_decl(&located.loaded, &located.record.action_type)?;
    let original = decl.executor.kind_str();
    let selected = match executor {
        Some(kind) => kind,
        None => executor_from_manifest(original)?,
    };
    let replay_dir = bundle_dir.unwrap_or_else(|| default_replay_dir(ctx.cwd, &located.record));

    if dry_run {
        println!("would replay action {}", located.record.action_id);
        println!("  action_type {}", located.record.action_type);
        println!("  original_executor {original}");
        println!("  replay_executor {}", selected.as_str());
        println!("  bundle {}", replay_dir.display());
        return Ok(());
    }

    let result = run_executor(&selected, &located.record, decl, &replay_dir).await?;
    write_json(&replay_dir.join("replay_result.json"), &result)?;
    println!(
        "replayed action {} via {}",
        located.record.action_id,
        selected.as_str()
    );
    println!("  bundle {}", replay_dir.display());
    Ok(())
}

struct ReplayWorkspace<'a> {
    cwd: &'a Path,
    memory: &'a MemoryStore,
    workspace: WorkspaceId,
    verticals_dir: &'a Path,
}

async fn open_workspace_memory(cwd: &Path) -> ArgoResult<(MemoryStore, WorkspaceId)> {
    let home = resolve_home(cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(&db_path(&home)).await?;
    store.migrate().await?;
    let name = cwd
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("workspace");
    let root = cwd.to_string_lossy();
    let (_tenant, workspace) = store.ensure_workspace(name, &root).await?;
    Ok((MemoryStore::new(Arc::new(store)), workspace))
}

struct LocatedReplay {
    record: ActionRecord,
    loaded: LoadedVertical,
}

async fn find_action(
    memory: &MemoryStore,
    workspace: WorkspaceId,
    action_id: &str,
    verticals_dir: &Path,
) -> ArgoResult<Option<LocatedReplay>> {
    let (registry, _errors) = argo_vertical::load_dir(verticals_dir)?;
    let mut latest: Option<LocatedReplay> = None;
    for (vid, loaded) in registry.list() {
        let rows = memory
            .list_for_vertical(workspace, MemoryFile::Memory, vid)
            .await?;
        for row in rows {
            let Ok(rec) = serde_json::from_str::<ActionRecord>(&row.body) else {
                continue;
            };
            if rec.schema_key != ActionRecord::SCHEMA_KEY || rec.action_id != action_id {
                continue;
            }
            let take = latest
                .as_ref()
                .map(|prev| rec.created_at > prev.record.created_at)
                .unwrap_or(true);
            if take {
                latest = Some(LocatedReplay {
                    record: rec,
                    loaded: loaded.clone(),
                });
            }
        }
    }
    Ok(latest)
}

fn action_decl<'a>(
    loaded: &'a LoadedVertical,
    action_type: &str,
) -> ArgoResult<&'a ActionDeclaration> {
    loaded
        .manifest
        .actions
        .as_ref()
        .and_then(|a| a.actions.get(action_type))
        .ok_or_else(|| {
            ArgoError::Provider(format!(
                "vertical '{}' does not declare action_type '{action_type}'",
                loaded.manifest.vertical.id
            ))
        })
}

fn executor_from_manifest(kind: &str) -> ArgoResult<ActionReplayExecutor> {
    match kind {
        "human" => Ok(ActionReplayExecutor::Human),
        "api" => Ok(ActionReplayExecutor::Api),
        "computer_use" => Ok(ActionReplayExecutor::ComputerUse),
        other => Err(ArgoError::Provider(format!(
            "action replay does not support original executor '{other}'; pass --executor human, --executor api, or --executor computer_use"
        ))),
    }
}

fn default_replay_dir(cwd: &Path, action: &ActionRecord) -> PathBuf {
    cwd.join(".skycot")
        .join("outputs")
        .join(&action.job_id)
        .join("actions")
        .join(&action.action_id)
        .join(format!(
            "replay-{}",
            Utc::now().format("%Y%m%dT%H%M%S%.6fZ")
        ))
}

async fn run_executor(
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
            ArgoError::Provider(format!("create replay dir {}: {e}", parent.display()))
        })?;
    }
    std::fs::write(path, body)
        .map_err(|e| ArgoError::Provider(format!("write {}: {e}", path.display())))
}

fn write_json(path: &Path, value: &serde_json::Value) -> ArgoResult<()> {
    let body = serde_json::to_string_pretty(value)
        .map_err(|e| ArgoError::Provider(format!("serialize replay json: {e}")))?;
    write_text(path, &body)
}

#[cfg(test)]
mod tests {
    use super::*;
    use argo_types::WorkspaceId;
    use tempfile::TempDir;

    fn workspace_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .unwrap()
            .to_path_buf()
    }

    fn completed_action(action_id: &str) -> ActionRecord {
        ActionRecord {
            schema_key: ActionRecord::SCHEMA_KEY.into(),
            action_id: action_id.into(),
            job_id: "job-replay".into(),
            worker_id: argo_identity::WorkerId::nil(),
            item_id: Some("item-1".into()),
            action_type: "create_listing_draft".into(),
            risk_level: "low".into(),
            requires_approval: false,
            state: "completed".into(),
            payload: json!({"title": "Demo skates"}),
            approved_by: None,
            approved_at: None,
            executed_at: Some("2026-01-01T00:00:00Z".into()),
            platform: Some("marketplace".into()),
            result: Some(json!({"draft_url": "https://example.test/draft"})),
            evidence: vec!["argo://search/demo".into()],
            blocked_reason: None,
            created_at: Utc::now().to_rfc3339(),
        }
    }

    async fn seed_action(
        home: &TempDir,
        work: &Path,
        rec: &ActionRecord,
    ) -> (MemoryStore, WorkspaceId) {
        let store = SessionStore::open(&db_path(home.path())).await.unwrap();
        store.migrate().await.unwrap();
        let root = work.to_string_lossy();
        let (_tenant, ws) = store.ensure_workspace("replay-test", &root).await.unwrap();
        let memory = MemoryStore::new(Arc::new(store));
        memory
            .append_for_vertical(
                ws,
                MemoryFile::Memory,
                serde_json::to_string(rec).unwrap(),
                None,
                "gtm-engineer",
            )
            .await
            .unwrap();
        (memory, ws)
    }

    #[tokio::test(flavor = "current_thread")]
    async fn replay_human_writes_sibling_bundle_with_result() {
        let work = tempfile::tempdir().unwrap();
        let home = tempfile::tempdir().unwrap();
        let rec = completed_action("act-replay-test");
        let (memory, ws) = seed_action(&home, work.path(), &rec).await;
        let verticals = workspace_root().join("verticals");
        replay_in_workspace(
            &rec.action_id,
            Some(ActionReplayExecutor::Human),
            false,
            None,
            ReplayWorkspace {
                cwd: work.path(),
                memory: &memory,
                workspace: ws,
                verticals_dir: &verticals,
            },
        )
        .await
        .unwrap();

        let action_dir = work
            .path()
            .join(".skycot/outputs/job-replay/actions/act-replay-test");
        let replay = std::fs::read_dir(&action_dir)
            .unwrap()
            .filter_map(Result::ok)
            .find(|e| e.file_name().to_string_lossy().starts_with("replay-"))
            .expect("replay dir");
        let result = std::fs::read_to_string(replay.path().join("replay_result.json")).unwrap();
        assert!(result.contains("human_handoff"));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn human_executor_writes_requested_bundle_dir() {
        let action = completed_action("act-dry-run");
        let dir = tempfile::tempdir().unwrap().path().join("dry-run-bundle");
        let loaded = argo_vertical::load_dir(&workspace_root().join("verticals"))
            .unwrap()
            .0
            .get("gtm-engineer")
            .unwrap()
            .clone();
        let decl = action_decl(&loaded, &action.action_type).unwrap();
        let result = run_executor(&ActionReplayExecutor::Human, &action, decl, &dir)
            .await
            .unwrap();
        assert_eq!(result["outcome"], "human_handoff");
        assert!(dir.exists());
    }

    #[tokio::test(flavor = "current_thread")]
    async fn dry_run_does_not_create_requested_bundle() {
        let work = tempfile::tempdir().unwrap();
        let home = tempfile::tempdir().unwrap();
        let rec = completed_action("act-replay-dry-run");
        let (memory, ws) = seed_action(&home, work.path(), &rec).await;
        let bundle = work.path().join("would-be-replay");
        let verticals = workspace_root().join("verticals");

        replay_in_workspace(
            &rec.action_id,
            Some(ActionReplayExecutor::Human),
            true,
            Some(bundle.clone()),
            ReplayWorkspace {
                cwd: work.path(),
                memory: &memory,
                workspace: ws,
                verticals_dir: &verticals,
            },
        )
        .await
        .unwrap();

        assert!(!bundle.exists());
    }
}
