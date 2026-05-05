//! `argo work` output artifacts (#721).
//!
//! For every completed worker job, write a small per-job
//! directory under `.skycot/outputs/<job_id>/` containing:
//!
//!   transcript.md       full user/assistant chat (markdown)
//!   assistant_output.md final assistant text in full
//!   job.json            job metadata
//!   result.json         pointers + preview
//!
//! This complements the in-DB `completed_jobs` memory entry
//! (which carries a 2000-char preview and the session id) by
//! letting operators inspect a run with regular shell tools —
//! cat / grep / editor — instead of fishing the full body out
//! of SQLite via `argo session transcript`.
//!
//! The directory is owned by the worker run; we only ever
//! create + write, never delete. Rerunning a job (which uses a
//! fresh job_id under the dedup scheme) leaves prior artifacts
//! intact.

use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, SessionId};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use uuid::Uuid;

/// All the pieces `process_job` already has by the time the
/// turn completes — bundled into a single struct so the call
/// site stays readable. Borrowed strs to avoid clones.
pub(crate) struct ArtifactInputs<'a> {
    pub job_id: Uuid,
    pub session_id: SessionId,
    pub vertical_id: &'a str,
    pub provider: &'a str,
    pub model: &'a str,
    pub created_at: &'a str,
    pub completed_at: &'a str,
    pub assistant_id: Uuid,
    pub assistant_text: &'a str,
    pub assistant_text_preview: &'a str,
}

/// Materialise the `.skycot/outputs/<job_id>/` directory and
/// return its absolute path. Caller stores that path in the
/// `completed_jobs` memory entry so downstream tooling can
/// rediscover the artifacts without re-deriving the layout.
pub(crate) async fn write_artifacts(
    store: &Arc<SessionStore>,
    cwd: &Path,
    inputs: &ArtifactInputs<'_>,
) -> ArgoResult<PathBuf> {
    let output_dir = cwd
        .join(".skycot")
        .join("outputs")
        .join(inputs.job_id.to_string());
    std::fs::create_dir_all(&output_dir)
        .map_err(|e| ArgoError::Provider(format!("create output dir: {e}")))?;

    let session = store
        .get_session(inputs.session_id)
        .await?
        .ok_or_else(|| ArgoError::Provider(format!("no session {}", inputs.session_id.0)))?;
    let messages = store.list_messages(inputs.session_id).await?;

    let transcript = render_transcript_md(&session, &messages);
    write_file(&output_dir.join("transcript.md"), &transcript)?;
    write_file(
        &output_dir.join("assistant_output.md"),
        inputs.assistant_text,
    )?;

    let job = json!({
        "job_id": inputs.job_id.to_string(),
        "vertical_id": inputs.vertical_id,
        "session_id": inputs.session_id.0.to_string(),
        "provider": inputs.provider,
        "model": inputs.model,
        "status": "completed",
        "created_at": inputs.created_at,
        "completed_at": inputs.completed_at,
    });
    write_file(
        &output_dir.join("job.json"),
        &serde_json::to_string_pretty(&job).unwrap_or_default(),
    )?;

    let result = json!({
        "assistant_id": inputs.assistant_id.to_string(),
        "assistant_text_preview": inputs.assistant_text_preview,
        "full_assistant_text_path": output_dir
            .join("assistant_output.md")
            .to_string_lossy(),
        "transcript_path": output_dir.join("transcript.md").to_string_lossy(),
    });
    write_file(
        &output_dir.join("result.json"),
        &serde_json::to_string_pretty(&result).unwrap_or_default(),
    )?;
    Ok(output_dir)
}

fn write_file(path: &Path, body: &str) -> ArgoResult<()> {
    std::fs::write(path, body)
        .map_err(|e| ArgoError::Provider(format!("write {}: {e}", path.display())))
}

fn render_transcript_md(
    session: &argo_store::SessionHeader,
    messages: &[argo_store::MessageRow],
) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "# Worker run transcript — session {}\n\n",
        session.id.0
    ));
    if let Some(v) = &session.vertical_id {
        out.push_str(&format!("- vertical: `{v}`\n"));
    }
    out.push_str(&format!(
        "- provider: `{}/{}`\n",
        session.provider, session.model
    ));
    out.push_str(&format!("- started: {}\n", session.started_at.to_rfc3339()));
    if let Some(end) = session.ended_at {
        out.push_str(&format!("- ended: {}\n", end.to_rfc3339()));
    }
    out.push_str(&format!("- messages: {}\n\n---\n\n", messages.len()));

    for (i, row) in messages.iter().enumerate() {
        out.push_str(&format!("## [{:02}] {}\n\n", i + 1, row.role));
        let body = render_blocks_md(&row.content_json);
        if body.trim().is_empty() {
            out.push_str("_(empty)_\n\n");
        } else {
            out.push_str(&body);
            if !body.ends_with('\n') {
                out.push('\n');
            }
            out.push('\n');
        }
    }
    out
}

/// Stored content_json may be a typed-block array, a raw string
/// (legacy), or null. Render to readable markdown either way.
fn render_blocks_md(content_json: &str) -> String {
    let parsed: Value = serde_json::from_str(content_json).unwrap_or(Value::Null);
    let arr: Vec<Value> = match parsed {
        Value::Array(a) => a,
        Value::String(s) => return s,
        Value::Null => return String::new(),
        other => vec![other],
    };
    let mut out = String::new();
    for b in arr {
        let kind = b.get("type").and_then(|t| t.as_str()).unwrap_or("?");
        match kind {
            "text" => {
                if let Some(t) = b.get("text").and_then(|t| t.as_str()) {
                    out.push_str(t);
                    out.push('\n');
                }
            }
            "tool_use" => {
                let name = b.get("name").and_then(|n| n.as_str()).unwrap_or("?");
                let input = b
                    .get("input")
                    .map(|i| i.to_string())
                    .unwrap_or_else(|| "{}".into());
                out.push_str(&format!("→ tool_use `{name}`({input})\n"));
            }
            "tool_result" => {
                let id = b.get("id").and_then(|i| i.as_str()).unwrap_or("?");
                let body = b
                    .get("content")
                    .and_then(|c| c.as_array())
                    .and_then(|arr| {
                        arr.iter()
                            .find_map(|inner| inner.get("text").and_then(|t| t.as_str()))
                    })
                    .unwrap_or("(no text)");
                let is_error = b.get("is_error").and_then(|e| e.as_bool()).unwrap_or(false);
                let mark = if is_error { "✗" } else { "✓" };
                out.push_str(&format!("{mark} tool_result `{id}`: {body}\n"));
            }
            other => {
                out.push_str(&format!("[{other}]\n"));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_blocks_md_handles_typed_text_blocks() {
        let out = render_blocks_md(r#"[{"type":"text","text":"hi there"}]"#);
        assert!(out.contains("hi there"));
    }

    #[test]
    fn render_blocks_md_handles_raw_string_legacy() {
        let out = render_blocks_md(r#""legacy raw body""#);
        assert!(out.contains("legacy raw body"));
    }

    #[test]
    fn render_blocks_md_passes_full_text_no_truncation() {
        let long = "x".repeat(5000);
        let json = format!(r#"[{{"type":"text","text":"{long}"}}]"#);
        let out = render_blocks_md(&json);
        // No 80-char snippet, no 2000-char cap.
        assert!(out.len() >= 5000);
    }

    #[test]
    fn render_blocks_md_handles_tool_use_and_tool_result() {
        let json = r#"[
            {"type":"tool_use","name":"web_search","input":{"q":"GTM"}},
            {"type":"tool_result","id":"abc","is_error":false,
             "content":[{"type":"text","text":"ok"}]}
        ]"#;
        let out = render_blocks_md(json);
        assert!(out.contains("→ tool_use `web_search`"));
        assert!(out.contains("✓ tool_result `abc`: ok"));
    }
}
