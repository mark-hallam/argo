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
use base64::engine::general_purpose::STANDARD;
use base64::Engine as _;
use std::path::{Path, PathBuf};
use std::sync::Arc;

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
        .map_err(|e| ArgoError::Provider(format!("cwd: {e}")))?;
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
        ArgoError::Provider(format!("read {}: {e}", path.display()))
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

pub(crate) fn emit_markdown(
    trajectory: &Trajectory,
    out: Option<&Path>,
    inline: bool,
) -> ArgoResult<()> {
    let body = render_markdown(trajectory, inline)?;
    match out {
        Some(path) => std::fs::write(path, body)
            .map_err(|e| ArgoError::Provider(format!("write {}: {e}", path.display()))),
        None => {
            print!("{body}");
            Ok(())
        }
    }
}

pub(crate) fn render_markdown(trajectory: &Trajectory, inline: bool) -> ArgoResult<String> {
    let mut out = String::new();
    out.push_str(&format!(
        "# CU session `{}` — {}\n\n",
        trajectory.session_id,
        trajectory.action_type
    ));
    out.push_str(&format!(
        "_status: **{}**_  \n_job: `{}`_  \n_steps: {}_  \n\n",
        trajectory.status.label(),
        trajectory.job_id,
        trajectory.steps.len()
    ));
    for (idx, step) in trajectory.steps.iter().enumerate() {
        let row = &step.row;
        let summary = row.action.summary();
        out.push_str(&format!(
            "## Step {} — {}\n\n_at: {}_  \n_tokens: {}_  \n",
            row.step, summary, row.at, row.tokens_used
        ));
        out.push_str(&format!(
            "_budget: tokens {}/{} • steps {}/{} • wallclock {}s/{}s_\n\n",
            row.budget.tokens_used,
            row.budget.tokens_cap,
            row.budget.steps_used,
            row.budget.steps_cap,
            row.budget.wallclock_seconds_used,
            row.budget.wallclock_seconds_cap,
        ));
        if !row.result.ok {
            out.push_str("_result: **failed**_\n\n");
        }
        if let Some(text) = row.result.text.as_deref() {
            out.push_str(&format!("```\n{}\n```\n\n", text.trim_end()));
        }
        out.push_str(&format!(
            "![step {}]({})\n\n",
            row.step,
            screenshot_link(&step.screenshot, inline)?
        ));
        if idx + 1 < trajectory.steps.len() {
            out.push_str("---\n\n");
        }
    }
    Ok(out)
}

fn screenshot_link(src: &ScreenshotSource, inline: bool) -> ArgoResult<String> {
    match src {
        ScreenshotSource::Disk(path) => {
            if inline {
                let bytes = std::fs::read(path).map_err(|e| {
                    ArgoError::Provider(format!("read {}: {e}", path.display()))
                })?;
                Ok(format!("data:image/png;base64,{}", STANDARD.encode(bytes)))
            } else {
                Ok(format!("file://{}", path.display()))
            }
        }
        ScreenshotSource::Blob(blob_ref) => Ok(format!("argo-blob://{}", blob_ref.id)),
    }
}

pub(crate) fn emit_ndjson(trajectory: &Trajectory, out: Option<&Path>) -> ArgoResult<()> {
    let mut buf = String::new();
    let header = serde_json::json!({
        "event": "session",
        "session_id": trajectory.session_id,
        "job_id": trajectory.job_id,
        "action_type": trajectory.action_type,
        "status": trajectory.status.label(),
        "steps": trajectory.steps.len(),
    });
    buf.push_str(&header.to_string());
    buf.push('\n');
    for step in &trajectory.steps {
        let payload = serde_json::json!({
            "event": "step",
            "step": step.row.step,
            "at": step.row.at,
            "action": step.row.action,
            "result": step.row.result,
            "screenshot_relpath": step.row.screenshot_relpath,
            "tokens_used": step.row.tokens_used,
        });
        buf.push_str(&payload.to_string());
        buf.push('\n');
    }
    match out {
        Some(path) => std::fs::write(path, buf)
            .map_err(|e| ArgoError::Provider(format!("write {}: {e}", path.display()))),
        None => {
            print!("{buf}");
            Ok(())
        }
    }
}

pub(crate) fn write_gif(
    trajectory: &Trajectory,
    out: &Path,
    fps: u32,
) -> ArgoResult<()> {
    use image::codecs::gif::{GifEncoder, Repeat};
    use image::{Delay, Frame};
    use std::fs::File;
    use std::io::BufWriter;
    use std::time::Duration;

    let frame_delay = Duration::from_millis(1000 / u64::from(fps));
    let file = File::create(out)
        .map_err(|e| ArgoError::Provider(format!("create {}: {e}", out.display())))?;
    let mut enc = GifEncoder::new(BufWriter::new(file));
    enc.set_repeat(Repeat::Infinite)
        .map_err(|e| ArgoError::Provider(format!("gif repeat: {e}")))?;
    for step in &trajectory.steps {
        let bytes = read_screenshot_bytes(&step.screenshot)?;
        let img = image::load_from_memory(&bytes)
            .map_err(|e| ArgoError::Provider(format!("decode step {}: {e}", step.row.step)))?
            .to_rgba8();
        let frame =
            Frame::from_parts(img, 0, 0, Delay::from_saturating_duration(frame_delay));
        enc.encode_frame(frame)
            .map_err(|e| ArgoError::Provider(format!("encode step {}: {e}", step.row.step)))?;
    }
    Ok(())
}

fn read_screenshot_bytes(src: &ScreenshotSource) -> ArgoResult<Vec<u8>> {
    match src {
        ScreenshotSource::Disk(path) => std::fs::read(path).map_err(|e| {
            ArgoError::Provider(format!("read {}: {e}", path.display()))
        }),
        ScreenshotSource::Blob(_) => Err(ArgoError::Provider(
            "BlobStore screenshot source not yet wired (depends on \
             claude-cu-screenshot-storage)"
                .into(),
        )),
    }
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
