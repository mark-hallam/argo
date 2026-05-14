//! `argo cu replay <session-id>` — render a recorded computer-use
//! session as markdown (+ optional GIF). Read-only consumer of the
//! per-action bundle that `AuditLog` writes (`browser.jsonl` +
//! `step-NNN.png` under `.skycot/outputs/<job_id>/actions/<id>/`).
//! `session_id` is the CU dispatch `action_id`. Cross-workspace
//! lookups surface as `ArgoError::NotFound`.

use crate::cli::cli_cu_cmds::CuCmd;
use crate::handlers::worker_actions::ActionRecord;
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_executor_computer_use::audit::AuditRow;
use argo_memory::{MemoryFile, MemoryStore};
use argo_store::attached_blob::BlobRef;
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, WorkspaceId};
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub(crate) use super::cu_replay_render::{emit_markdown, emit_ndjson, write_gif};
#[cfg(test)]
pub(crate) use super::cu_replay_render::{render_markdown, screenshot_link};

/// Where a per-step screenshot lives. `Disk` is the v0 source;
/// `Blob` is the follow-up path once `claude-cu-screenshot-storage`
/// lands and CU writes through `BlobStore`.
#[derive(Debug, Clone)]
pub enum ScreenshotSource {
    Disk(PathBuf),
    /// Reserved for `claude-cu-screenshot-storage`; renderer
    /// already handles it (`screenshot_link`/`read_screenshot_bytes`)
    /// so the storage-layer brief can flip a single construction
    /// site without changing this surface.
    #[allow(dead_code)]
    Blob(BlobRef),
}

pub(crate) async fn run(cmd: CuCmd) -> ArgoResult<()> {
    match cmd {
        CuCmd::Replay {
            session_id,
            out,
            gif,
            gif_fps,
            from,
            to,
            inline,
            format,
        } => {
            cmd_replay(
                &session_id,
                ReplayOpts {
                    out,
                    gif,
                    gif_fps,
                    from,
                    to,
                    inline,
                    format,
                },
            )
            .await
        }
    }
}

pub(crate) struct ReplayOpts {
    pub out: Option<PathBuf>,
    pub gif: Option<PathBuf>,
    pub gif_fps: u32,
    pub from: Option<u32>,
    pub to: Option<u32>,
    pub inline: bool,
    pub format: String,
}

pub(crate) async fn cmd_replay(session_id: &str, opts: ReplayOpts) -> ArgoResult<()> {
    if !(1..=30).contains(&opts.gif_fps) {
        return Err(ArgoError::Provider(format!(
            "--gif-fps {} out of range; must be 1..=30",
            opts.gif_fps
        )));
    }
    let cwd = std::env::current_dir()
        .map_err(|e| ArgoError::Io(format!("cwd: {e}")))?;
    let (memory, workspace) = open_workspace_memory(&cwd).await?;
    let verticals_dir = crate::handlers::vertical::resolve_verticals_dir();
    let action = find_action(&memory, workspace, session_id, &verticals_dir)
        .await?
        .ok_or_else(|| ArgoError::NotFound(format!("cu session {session_id}")))?;
    let bundle = bundle_dir(&cwd, &action);
    let rows = load_trajectory(&bundle)?;
    let trajectory = build_trajectory(&action, &bundle, &rows, opts.from, opts.to)?;
    if trajectory.steps.is_empty() {
        return Err(ArgoError::Provider(format!(
            "cu session {session_id} has no steps in the requested range"
        )));
    }
    if let Some(path) = opts.gif.as_ref() {
        write_gif(&trajectory, path, opts.gif_fps)?;
    }
    match opts.format.as_str() {
        "markdown" => emit_markdown(&trajectory, opts.out.as_deref(), opts.inline),
        "json" => emit_ndjson(&trajectory, opts.out.as_deref()),
        other => Err(ArgoError::Provider(format!(
            "--format {other} not supported; use 'markdown' or 'json'"
        ))),
    }
}

#[derive(Debug)]
pub struct RenderedStep {
    pub row: AuditRow,
    pub screenshot: ScreenshotSource,
}

#[derive(Debug)]
pub struct Trajectory {
    pub session_id: String,
    pub job_id: String,
    pub action_type: String,
    pub status: TrajectoryStatus,
    pub steps: Vec<RenderedStep>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TrajectoryStatus {
    Success,
    Failed,
    Aborted,
    InProgress,
}

impl TrajectoryStatus {
    pub fn label(self) -> &'static str {
        match self {
            Self::Success => "success",
            Self::Failed => "failed",
            Self::Aborted => "aborted",
            Self::InProgress => "in_progress",
        }
    }
}

pub(crate) fn build_trajectory(
    action: &ActionRecord,
    bundle: &Path,
    rows: &[AuditRow],
    from: Option<u32>,
    to: Option<u32>,
) -> ArgoResult<Trajectory> {
    let lo = from.unwrap_or(u32::MIN);
    let hi = to.unwrap_or(u32::MAX);
    if lo > hi {
        return Err(ArgoError::Provider(format!(
            "--from {lo} > --to {hi}: range is empty"
        )));
    }
    let mut steps = Vec::new();
    for row in rows.iter().filter(|r| r.step >= lo && r.step <= hi) {
        let screenshot = ScreenshotSource::Disk(bundle.join(&row.screenshot_relpath));
        steps.push(RenderedStep {
            row: row.clone(),
            screenshot,
        });
    }
    Ok(Trajectory {
        session_id: action.action_id.clone(),
        job_id: action.job_id.clone(),
        action_type: action.action_type.clone(),
        status: classify_status(action),
        steps,
    })
}

fn classify_status(action: &ActionRecord) -> TrajectoryStatus {
    match action.state.as_str() {
        "completed" => TrajectoryStatus::Success,
        "failed" => TrajectoryStatus::Failed,
        "blocked" | "aborted" => TrajectoryStatus::Aborted,
        _ => TrajectoryStatus::InProgress,
    }
}

pub(crate) fn bundle_dir(cwd: &Path, action: &ActionRecord) -> PathBuf {
    cwd.join(".skycot")
        .join("outputs")
        .join(&action.job_id)
        .join("actions")
        .join(&action.action_id)
}

pub(crate) fn load_trajectory(bundle: &Path) -> ArgoResult<Vec<AuditRow>> {
    let path = bundle.join("browser.jsonl");
    let body = std::fs::read_to_string(&path).map_err(|e| {
        ArgoError::Io(format!("read {}: {e}", path.display()))
    })?;
    let mut rows = Vec::new();
    for (idx, line) in body.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let row: AuditRow = serde_json::from_str(line).map_err(|e| {
            ArgoError::Provider(format!(
                "parse {}:{}: {e}",
                path.display(),
                idx + 1
            ))
        })?;
        rows.push(row);
    }
    Ok(rows)
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

pub(crate) async fn find_action(
    memory: &MemoryStore,
    workspace: WorkspaceId,
    action_id: &str,
    verticals_dir: &Path,
) -> ArgoResult<Option<ActionRecord>> {
    let (registry, _errors) = argo_vertical::load_dir(verticals_dir)?;
    let mut latest: Option<ActionRecord> = None;
    for (vid, _) in registry.list() {
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
                .map(|prev| rec.created_at > prev.created_at)
                .unwrap_or(true);
            if take {
                latest = Some(rec);
            }
        }
    }
    Ok(latest)
}

#[cfg(test)]
#[path = "cu_replay_tests.rs"]
mod tests;
