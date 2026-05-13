//! `argo action replay` — read-only action replay bundles.
//!
//! Per-executor bundle logic and file I/O live in the sibling
//! `action_replay_executors.rs` so this file stays under the
//! 500-LOC ceiling.
use crate::cli::{ActionCmd, ActionReplayExecutor};
use crate::handlers::worker_actions::ActionRecord;
use crate::home::{db_path, ensure_dir, resolve_home};
use action_replay_executors::{run_executor, write_json};
use argo_memory::{MemoryFile, MemoryStore};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, WorkspaceId};
use argo_vertical::{ActionDeclaration, LoadedVertical};
use chrono::Utc;
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
        _ => Err(ArgoError::Invariant(
            "non-replay action command routed to action_replay".into(),
        )),
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

#[path = "action_replay_executors.rs"]
mod action_replay_executors;

#[cfg(test)]
mod tests {
    use super::*;
    use argo_types::WorkspaceId;
    use serde_json::json;
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
