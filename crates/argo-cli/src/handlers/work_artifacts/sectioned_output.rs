//! `SectionedOutput` — parses the four canonical labelled
//! sections (`research`, `pricing`, `listings`,
//! `approval_checklist`) out of a worker's assistant text and
//! renders the file-bound artifacts plus the approval-checklist
//! preamble.
//!
//! Extracted from `mod.rs` to keep the orchestration file under
//! the 500-LOC ceiling. `mod.rs` re-exports `SectionedOutput`
//! via `pub(crate) use sectioned_output::SectionedOutput;` so
//! the dispatcher imports stay unchanged.

use super::sections;

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
