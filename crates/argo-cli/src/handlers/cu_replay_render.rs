//! Rendering and emission helpers for `argo cu replay`.
//!
//! Pure presentation layer extracted from `cu_replay.rs`: markdown
//! and NDJSON serialization, GIF encoding, and the screenshot-link
//! helper used by both the renderer and the test suite. All inputs
//! are owned by `cu_replay.rs` (`Trajectory`, `ScreenshotSource`);
//! this module never touches the workspace store or filesystem
//! lookups.

use super::cu_replay::{ScreenshotSource, Trajectory};
use argo_types::{ArgoError, ArgoResult};
use base64::engine::general_purpose::STANDARD;
use base64::Engine as _;
use std::path::Path;

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
        trajectory.session_id, trajectory.action_type
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

pub(crate) fn screenshot_link(src: &ScreenshotSource, inline: bool) -> ArgoResult<String> {
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

pub(crate) fn write_gif(trajectory: &Trajectory, out: &Path, fps: u32) -> ArgoResult<()> {
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
            .map_err(|e| ArgoError::Invariant(format!("decode step {}: {e}", step.row.step)))?
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
