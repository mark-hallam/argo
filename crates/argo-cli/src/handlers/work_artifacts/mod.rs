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

mod sections;

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
    /// Worker v2 (#722) — number of multi-step turns the worker
    /// actually executed for this job. `1` for a Worker v1-style
    /// single-turn run (or `--max-steps 1`), up to `--max-steps`
    /// for a fully exercised loop.
    pub steps_completed: u32,
    /// Lower-snake-case wire form: `task_complete`,
    /// `approval_required`, `need_info`, `max_steps_reached`,
    /// `research_tools_unavailable`. See `work_loop::StopReason`.
    pub stop_reason: &'a str,
    /// Free-form one-liner that accompanies `need_info`; empty
    /// for the other variants. Surfaced into the approval
    /// checklist + result.json so a human sees exactly which
    /// fact the worker is missing.
    pub stop_detail: &'a str,
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

    // #722 — split the assistant's final text into the four
    // sectioned deliverables the multi-step worker is prompted
    // to produce. Missing sections get a clear placeholder so
    // operators always see the same file layout regardless of
    // whether the worker actually wrote that section yet.
    let sections = SectionedOutput::from_assistant_text(inputs.assistant_text);
    write_file(
        &output_dir.join("research.md"),
        &sections.research_or_placeholder(),
    )?;
    write_file(
        &output_dir.join("pricing.md"),
        &sections.pricing_or_placeholder(),
    )?;
    write_file(
        &output_dir.join("listings.md"),
        &sections.listings_or_placeholder(),
    )?;
    write_file(
        &output_dir.join("approval_checklist.md"),
        &sections.approval_checklist_or_placeholder(inputs.stop_reason, inputs.stop_detail),
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
        "steps_completed": inputs.steps_completed,
        "stop_reason": inputs.stop_reason,
        "stop_detail": inputs.stop_detail,
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
        "research_path": output_dir.join("research.md").to_string_lossy(),
        "pricing_path": output_dir.join("pricing.md").to_string_lossy(),
        "listings_path": output_dir.join("listings.md").to_string_lossy(),
        "approval_checklist_path": output_dir
            .join("approval_checklist.md")
            .to_string_lossy(),
        "steps_completed": inputs.steps_completed,
        "stop_reason": inputs.stop_reason,
        "stop_detail": inputs.stop_detail,
    });
    write_file(
        &output_dir.join("result.json"),
        &serde_json::to_string_pretty(&result).unwrap_or_default(),
    )?;
    Ok(output_dir)
}

/// The four labelled sections we expect the multi-step worker
/// to emit in its final assistant message. Each is `None` when
/// the model didn't produce that header in this run; the
/// `*_or_placeholder` accessors provide a clear `_(not produced
/// this run)_` stub so the artifact directory layout stays
/// stable across runs.
#[derive(Debug, Default, Clone)]
pub(crate) struct SectionedOutput {
    pub research: Option<String>,
    pub pricing: Option<String>,
    pub listings: Option<String>,
    pub approval_checklist: Option<String>,
}

impl SectionedOutput {
    /// Parse `## research`, `## pricing`, `## listings`, and
    /// `## approval_checklist` headers (case-insensitive,
    /// tolerant of trailing qualifiers, emoji prefixes, and
    /// `_` vs ` ` variants) out of free-form assistant markdown.
    /// Recognises bare `## approval` as a fallback for the
    /// approval_checklist target. See `sections::split_sections`
    /// for the splitter contract; #726 rewrote it to walk
    /// line-by-line rather than slice via lowercased byte
    /// indices, which prevents nested-section bleed-through.
    pub(crate) fn from_assistant_text(text: &str) -> Self {
        let sections = sections::split_sections(text);
        SectionedOutput {
            research: sections::find_section(&sections, &["research"]),
            pricing: sections::find_section(&sections, &["pricing"]),
            listings: sections::find_section(&sections, &["listings", "listing"]),
            approval_checklist: sections::find_section(
                &sections,
                &["approval_checklist", "approval checklist", "approval"],
            ),
        }
    }

    pub(crate) fn research_or_placeholder(&self) -> String {
        section_or_placeholder("research", self.research.as_deref())
    }

    pub(crate) fn pricing_or_placeholder(&self) -> String {
        section_or_placeholder("pricing", self.pricing.as_deref())
    }

    pub(crate) fn listings_or_placeholder(&self) -> String {
        section_or_placeholder("listings", self.listings.as_deref())
    }

    /// The approval checklist file always carries the worker's
    /// stop reason in its preamble — even if the model didn't
    /// emit a `## approval_checklist` header — because the
    /// reason is the most operationally relevant signal for the
    /// human reviewer.
    pub(crate) fn approval_checklist_or_placeholder(
        &self,
        stop_reason: &str,
        stop_detail: &str,
    ) -> String {
        let body = match self.approval_checklist.as_deref() {
            Some(s) if !s.trim().is_empty() => s.to_string(),
            _ => "_(not produced this run — see assistant_output.md for the worker's full reasoning)_\n"
                .to_string(),
        };
        let mut out = String::from("# Approval checklist\n\n");
        out.push_str(&format!("- stop_reason: `{stop_reason}`\n"));
        if !stop_detail.is_empty() {
            out.push_str(&format!("- stop_detail: {stop_detail}\n"));
        }
        out.push_str("\n---\n\n");
        out.push_str(&body);
        out
    }
}

// `extract_section` (the old lower-case-byte-index splitter)
// was removed in #726 — see `sections::split_sections` for
// the line-based replacement. Kept as a removal-marker
// comment so a future archaeologist can find the rationale
// in the PR history.

fn section_or_placeholder(name: &str, body: Option<&str>) -> String {
    match body {
        Some(s) if !s.trim().is_empty() => s.to_string(),
        _ => format!(
            "## {name}\n\n_(not produced this run — see assistant_output.md for the worker's full reasoning)_\n"
        ),
    }
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

    /// #722 — splitter pulls the four labelled deliverables out
    /// of a multi-step worker's final assistant message.
    #[test]
    fn sectioned_output_splits_all_four_named_sections() {
        let body = "\
            preamble that should not be captured\n\n\
            ## research\nOne find with [src](https://e.example).\n\n\
            ## pricing\n| comp | $99 |\n\n\
            ## listings\nDraft: ProSkates 110mm\n\n\
            ## approval_checklist\n- [ ] confirm reserve price\n\n\
            [STOP: APPROVAL_REQUIRED]\n";
        let s = SectionedOutput::from_assistant_text(body);
        assert!(s.research.as_deref().unwrap_or("").contains("[src]"));
        assert!(s.pricing.as_deref().unwrap_or("").contains("comp"));
        assert!(s.listings.as_deref().unwrap_or("").contains("ProSkates"));
        assert!(s
            .approval_checklist
            .as_deref()
            .unwrap_or("")
            .contains("reserve price"));
    }

    #[test]
    fn sectioned_output_handles_missing_sections_with_placeholder() {
        let body = "## research\nfindings here\n";
        let s = SectionedOutput::from_assistant_text(body);
        assert!(s.research.is_some());
        assert!(s.pricing.is_none());
        let pricing = s.pricing_or_placeholder();
        assert!(pricing.contains("not produced this run"));
        let listings = s.listings_or_placeholder();
        assert!(listings.contains("not produced this run"));
    }

    #[test]
    fn approval_checklist_always_carries_stop_reason_in_preamble() {
        let s = SectionedOutput::from_assistant_text("## approval_checklist\n- [ ] human check\n");
        let out = s.approval_checklist_or_placeholder("approval_required", "");
        assert!(out.contains("# Approval checklist"));
        assert!(out.contains("stop_reason: `approval_required`"));
        assert!(out.contains("human check"));
    }

    #[test]
    fn approval_checklist_includes_need_info_detail() {
        let s = SectionedOutput::default();
        let out = s.approval_checklist_or_placeholder("need_info", "seller's reserve price");
        assert!(out.contains("stop_detail: seller's reserve price"));
    }

    #[test]
    fn sectioned_output_recognises_listings_singular_variant() {
        let body = "## listing\ndraft ad copy here\n";
        let s = SectionedOutput::from_assistant_text(body);
        assert!(
            s.listings.is_some(),
            "should match `## listing` (singular) too"
        );
    }

    #[test]
    fn sectioned_output_treats_unrelated_text_as_empty_sections() {
        let body = "the worker said many words but never used a section header";
        let s = SectionedOutput::from_assistant_text(body);
        assert!(s.research.is_none());
        assert!(s.pricing.is_none());
        assert!(s.listings.is_none());
        assert!(s.approval_checklist.is_none());
    }
}
