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

mod evidence;
mod sections;
mod transcript;

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

    let transcript = transcript::render_transcript_md(&session, &messages);
    write_file(&output_dir.join("transcript.md"), &transcript)?;
    write_file(
        &output_dir.join("assistant_output.md"),
        inputs.assistant_text,
    )?;

    // #727 Issue 1 — assemble sections from EVERY assistant
    // turn in the session, not just the final one. A research-
    // heavy job often produces pricing/listings in step 1 and
    // only the approval_checklist in step 2; pre-#727 the step-1
    // sections were lost because the splitter only saw the last
    // assistant text. `from_assistant_messages` merges per-step
    // results with "latest non-empty wins" semantics so step 1's
    // pricing survives when step 2 doesn't re-emit it.
    let assistant_texts: Vec<String> = messages
        .iter()
        .filter(|m| m.role == "assistant")
        .map(|m| extract_text_blocks(&m.content_json))
        .filter(|t| !t.trim().is_empty())
        .collect();
    let sections = SectionedOutput::from_assistant_messages(&assistant_texts);

    // #727 Issue 2 — research-evidence gate. Walk the session's
    // tool_result blocks, count successful web_search hits + the
    // total chars web_extract returned, and flag the bundle as
    // BLOCKED when the model claims approval-readiness on top of
    // weak research. We don't override stop_reason — operators
    // still see what the model claimed — but the artifact bundle
    // makes the gap loud via `not_ready_to_post: true` in
    // result.json and a BLOCKED preamble on approval_checklist.md.
    let row_pairs: Vec<(&str, &str)> = messages
        .iter()
        .map(|m| (m.role.as_str(), m.content_json.as_str()))
        .collect();
    let ev = evidence::summarize_evidence(row_pairs);
    let blocked = inputs.stop_reason == "approval_required" && !ev.is_sufficient();

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
        &sections.approval_checklist_or_placeholder(
            inputs.stop_reason,
            inputs.stop_detail,
            blocked,
            &ev.insufficiency_reason(),
        ),
    )?;

    let missing = sections.missing_sections();
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
        // #727 — visibility for operators inspecting the bundle.
        "sections_present": json!({
            "research": sections.research.is_some(),
            "pricing": sections.pricing.is_some(),
            "listings": sections.listings.is_some(),
            "approval_checklist": sections.approval_checklist.is_some(),
        }),
        "missing_sections": missing,
        "evidence": ev,
        "not_ready_to_post": blocked,
        "blocked_reason": if blocked { ev.insufficiency_reason() } else { String::new() },
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

    /// Walk every assistant message in document order and merge
    /// the per-message section results so the artifact bundle
    /// reflects the WHOLE worker run, not just the final
    /// assistant turn (#727 / Issue 1).
    ///
    /// Merge rule: for each named section, the LATEST non-empty
    /// version wins. If step 1 produced `## pricing` and step 2
    /// only produced `## approval_checklist`, the step-1 pricing
    /// body is preserved (instead of the prior behaviour of
    /// silently replacing it with a placeholder).
    pub(crate) fn from_assistant_messages<I, S>(messages: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut acc = SectionedOutput::default();
        for text in messages {
            let s = SectionedOutput::from_assistant_text(text.as_ref());
            if s.research.is_some() {
                acc.research = s.research;
            }
            if s.pricing.is_some() {
                acc.pricing = s.pricing;
            }
            if s.listings.is_some() {
                acc.listings = s.listings;
            }
            if s.approval_checklist.is_some() {
                acc.approval_checklist = s.approval_checklist;
            }
        }
        acc
    }

    /// Names of the four canonical sections that this run did
    /// NOT produce. Surfaced into result.json (#727) so the
    /// operator sees at a glance which deliverables landed.
    pub(crate) fn missing_sections(&self) -> Vec<&'static str> {
        let mut out = Vec::new();
        if self.research.is_none() {
            out.push("research");
        }
        if self.pricing.is_none() {
            out.push("pricing");
        }
        if self.listings.is_none() {
            out.push("listings");
        }
        if self.approval_checklist.is_none() {
            out.push("approval_checklist");
        }
        out
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
    ///
    /// When `blocked` is true, a prominent BLOCKED banner is
    /// prepended ahead of the model's checklist (#727 Issue 2).
    /// `blocked_reason` is the one-liner from
    /// `EvidenceSummary::insufficiency_reason()` that names what
    /// failed (zero-result searches, fetch errors, etc.).
    pub(crate) fn approval_checklist_or_placeholder(
        &self,
        stop_reason: &str,
        stop_detail: &str,
        blocked: bool,
        blocked_reason: &str,
    ) -> String {
        let body = match self.approval_checklist.as_deref() {
            Some(s) if !s.trim().is_empty() => s.to_string(),
            _ => "_(not produced this run — see assistant_output.md for the worker's full reasoning)_\n"
                .to_string(),
        };
        let mut out = String::from("# Approval checklist\n\n");
        if blocked {
            out.push_str(
                "> **🛑 BLOCKED — NOT READY TO POST.**\n>\n\
                 > The worker's research evidence was insufficient \
                 to back the approval-ready claims in this bundle. \
                 Do not post listings, message buyers, or commit \
                 spend until the items below are independently \
                 verified by a human.\n>\n",
            );
            if !blocked_reason.is_empty() {
                out.push_str(&format!("> - reason: {blocked_reason}\n"));
            }
            out.push('\n');
        }
        out.push_str(&format!("- stop_reason: `{stop_reason}`\n"));
        if !stop_detail.is_empty() {
            out.push_str(&format!("- stop_detail: {stop_detail}\n"));
        }
        if blocked {
            out.push_str("- not_ready_to_post: `true`\n");
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

/// Pull just the `text` blocks out of a stored `content_json`
/// payload, joining with `\n`. Used for multi-step section
/// assembly (#727) — we want the model's prose, NOT tool_use
/// or tool_result blocks (those would otherwise interleave
/// JSON noise into the splitter's input).
fn extract_text_blocks(content_json: &str) -> String {
    let parsed: Value = serde_json::from_str(content_json).unwrap_or(Value::Null);
    let arr: Vec<Value> = match parsed {
        Value::Array(a) => a,
        Value::String(s) => return s,
        _ => return String::new(),
    };
    let mut out = String::new();
    for b in arr {
        if b.get("type").and_then(|t| t.as_str()) == Some("text") {
            if let Some(t) = b.get("text").and_then(|t| t.as_str()) {
                out.push_str(t);
                out.push('\n');
            }
        }
    }
    out
}

#[cfg(test)]
mod tests;
