//! JD-to-charter synthesizer (`mission-audit-genesis-jd-llm-summarizer`).
//!
//! Before this brief, [`crate::job_description::genesis_from_job_description`]
//! passed the raw JD text straight through to
//! [`crate::pipeline::plan_with`] — fine for clean demo JDs, broken
//! for real customer JDs (long PDF extracts, multi-role boilerplate).
//!
//! The synthesizer is the seam that fixes that. Two impls ship:
//!
//! - [`StubSynthesizer`] — identity-charter (`mission` = raw JD).
//!   Preserves byte-for-byte v0 behaviour so existing tests pass and
//!   `genesis.jd_synthesizer = "stub"` (the default config) stays a
//!   back-compat no-op.
//! - [`LlmSynthesizer`] — wraps a [`crate::archetype::ProviderClient`]
//!   (the same narrow trait the archetype classifier uses). Builds a
//!   structured-output prompt, parses the JSON response into a
//!   [`WorkerCharter`], validates field-level bounds.
//!
//! ## Fail-soft helper
//!
//! Acceptance criterion (3) demands that a malformed provider
//! response surfaces as a clear [`ArgoError::Provider`] (not a
//! panic). The `Notes for the implementer` block in the brief
//! suggests an additional fail-soft mode for onboarding UX: log a
//! warning, fall back to the stub charter. Both reads are honoured —
//! the trait method is strict (returns the error), and the standalone
//! [`synthesize_or_stub`] helper wraps the strict call with a
//! `tracing::warn!` + stub fallback for production paths that
//! prioritize "operator gets *some* charter".
//!
//! ## Pre-processing
//!
//! Real JDs are messy — PDF extraction inserts double-spaces, page
//! footers, and 8KB of boilerplate before the actual role description.
//! [`preprocess`] strips excessive whitespace, dedupes blank-line
//! runs, and truncates to [`MAX_PROMPT_INPUT_BYTES`] before the
//! prompt is built. Stub path skips preprocess (its charter is the
//! raw JD by definition).

use crate::archetype::ProviderClient;
use crate::charter::WorkerCharter;
use argo_types::{ArgoError, ArgoResult};
use async_trait::async_trait;
use serde::Deserialize;
use std::sync::Arc;

/// Hard cap on the JD text passed to the model — anything past this
/// gets truncated by [`preprocess`]. 8KB is the brief-mandated value;
/// roughly 2K tokens at English-prose density.
pub const MAX_PROMPT_INPUT_BYTES: usize = 8 * 1024;

/// Structured trait surface — every JD-to-charter implementation
/// (stub or LLM) honours this shape. Async because [`LlmSynthesizer`]
/// blocks on a provider call; [`StubSynthesizer`] is async-but-
/// instant so callers don't have to branch on impl.
#[async_trait]
pub trait Synthesizer: Send + Sync {
    /// Produce a structured charter from the raw JD text. The
    /// implementor decides whether to validate, pre-process, fall
    /// back, or strict-error — see each impl's docs.
    async fn synthesize(&self, raw: &str) -> ArgoResult<WorkerCharter>;
}

/// Identity synthesizer — `mission` = raw JD, every other field
/// empty. Preserves byte-for-byte v0 behaviour. Used in tests, by
/// the `"stub"` config default, and as the fallback target inside
/// [`synthesize_or_stub`].
#[derive(Debug, Default, Clone, Copy)]
pub struct StubSynthesizer;

impl StubSynthesizer {
    /// Construct. Stateless; `StubSynthesizer::default()` works too.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Synthesizer for StubSynthesizer {
    async fn synthesize(&self, raw: &str) -> ArgoResult<WorkerCharter> {
        Ok(WorkerCharter::from_identity(raw))
    }
}

/// Provider-backed synthesizer. Wraps a
/// [`crate::archetype::ProviderClient`] (the same narrow async trait
/// the archetype LLM classifier uses), builds a JSON-mode prompt,
/// parses the response, and runs [`WorkerCharter::validate`] on it.
///
/// ## Strict failure modes
///
/// - Provider transport error → propagates as [`ArgoError::Provider`].
/// - Response is not parseable JSON → [`ArgoError::Provider`]
///   with the parse error message and the offending response head.
/// - Parsed charter fails [`WorkerCharter::validate`] → [`ArgoError::Schema`]
///   with the validation rule that fired.
///
/// Callers wanting fail-soft semantics (warn + fall back to identity
/// charter) wrap this with [`synthesize_or_stub`].
pub struct LlmSynthesizer {
    provider: Arc<dyn ProviderClient>,
}

impl LlmSynthesizer {
    /// Construct around any [`ProviderClient`]. Implementations MUST
    /// set temperature to 0 (or the model's nearest deterministic
    /// equivalent) so the same JD compresses to the same charter on
    /// every call.
    #[must_use]
    pub fn new(provider: Arc<dyn ProviderClient>) -> Self {
        Self { provider }
    }
}

#[async_trait]
impl Synthesizer for LlmSynthesizer {
    async fn synthesize(&self, raw: &str) -> ArgoResult<WorkerCharter> {
        let cleaned = preprocess(raw);
        let prompt = build_prompt(&cleaned);
        let response = self.provider.classify(&prompt).await?;
        let mut charter = parse_response(&response, raw)?;
        charter.source_jd_hash = WorkerCharter::sha256_jd(raw);
        charter
            .validate()
            .map_err(|e| ArgoError::Schema(format!("synthesized charter failed validation: {e}")))?;
        Ok(charter)
    }
}

/// Wraps any [`Synthesizer`] with fail-soft semantics: if `synth`
/// errors (transport, parse, or validation), emit a `tracing::warn!`
/// and return [`WorkerCharter::from_identity`] for `raw`. Use for
/// onboarding UX paths where the operator must always get *some*
/// charter and the LLM is best-effort. Tests + strict callers use
/// the synthesizer directly.
pub async fn synthesize_or_stub(synth: &dyn Synthesizer, raw: &str) -> WorkerCharter {
    match synth.synthesize(raw).await {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!(error = %e, "jd synthesis failed; falling back to identity charter");
            WorkerCharter::from_identity(raw)
        }
    }
}

/// Strip excessive whitespace, dedupe blank-line runs, and truncate
/// to [`MAX_PROMPT_INPUT_BYTES`]. Public so the LLM-synthesizer's
/// pre-processing is testable in isolation.
#[must_use]
pub fn preprocess(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len().min(MAX_PROMPT_INPUT_BYTES));
    let mut prev_blank = false;
    for line in raw.lines() {
        let collapsed = collapse_whitespace(line);
        let is_blank = collapsed.is_empty();
        if is_blank && prev_blank {
            continue;
        }
        out.push_str(&collapsed);
        out.push('\n');
        prev_blank = is_blank;
    }
    if out.len() > MAX_PROMPT_INPUT_BYTES {
        out.truncate(safe_truncate_boundary(&out, MAX_PROMPT_INPUT_BYTES));
    }
    out
}

fn collapse_whitespace(line: &str) -> String {
    line.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn safe_truncate_boundary(s: &str, max: usize) -> usize {
    let mut idx = max.min(s.len());
    while idx > 0 && !s.is_char_boundary(idx) {
        idx -= 1;
    }
    idx
}

/// Build the structured-output prompt. Schema is inlined verbatim so
/// the model has zero ambiguity about field names / types. JSON-only
/// output is requested; `parse_response` is tolerant of a leading
/// code fence.
fn build_prompt(cleaned_jd: &str) -> String {
    format!(
        r#"You are summarizing a job description (JD) into a structured worker charter for an autonomous AI worker.

Return EXACTLY ONE JSON object — no surrounding prose, no markdown code fence — matching this schema:
{{
  "mission": "<1-paragraph mission, <= 280 chars>",
  "responsibilities": ["<resp 1>", "<resp 2>", ...],   // 1-7 items, each <= 140 chars
  "tools_hint": ["<tool 1>", ...],                     // 0-5 items
  "escalation_contact": "<role name or email, or null>",
  "vertical_hint": "<one of: sales_sdr, customer_support_agent, marketing_intern, recruiter_screener, ops_on_call, research_analyst, calendar_coordinator, social_media_drafter, lead_researcher, bookkeeping_assistant, project_pm, gtm_engineer, or null>"
}}

Rules:
- mission MUST be non-empty.
- If the JD spans multiple roles, choose the most senior / primary role.
- Strip employer boilerplate (benefits, EEO clauses, application instructions).
- If a field has no clear answer, use an empty array or null — never invent.

JD:
{cleaned_jd}"#
    )
}

/// Parse the model response into a [`WorkerCharter`]. Tolerant of
/// a leading ```json fence (some models emit it despite the prompt
/// instruction). The `source_jd_hash` is set by the caller after
/// parsing — the model is not asked to compute it.
fn parse_response(response: &str, raw_jd: &str) -> ArgoResult<WorkerCharter> {
    let trimmed = strip_code_fence(response.trim());
    let mut charter: ParsedCharter = serde_json::from_str(trimmed).map_err(|e| {
        ArgoError::Provider(format!(
            "synthesizer response is not valid JSON: {e}; head={:?}",
            head(trimmed, 120)
        ))
    })?;
    // Defensive trim — the model often emits ` "Mission ..." `.
    charter.mission = charter.mission.trim().to_string();
    charter.responsibilities = charter
        .responsibilities
        .into_iter()
        .map(|r| r.trim().to_string())
        .filter(|r| !r.is_empty())
        .collect();
    charter.tools_hint = charter
        .tools_hint
        .into_iter()
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
        .collect();
    Ok(WorkerCharter {
        mission: charter.mission,
        responsibilities: charter.responsibilities,
        tools_hint: charter.tools_hint,
        escalation_contact: charter.escalation_contact.filter(|s| !s.trim().is_empty()),
        vertical_hint: charter.vertical_hint.filter(|s| !s.trim().is_empty()),
        source_jd_hash: WorkerCharter::sha256_jd(raw_jd),
    })
}

#[derive(Debug, Deserialize)]
struct ParsedCharter {
    mission: String,
    #[serde(default)]
    responsibilities: Vec<String>,
    #[serde(default)]
    tools_hint: Vec<String>,
    #[serde(default)]
    escalation_contact: Option<String>,
    #[serde(default)]
    vertical_hint: Option<String>,
}

fn strip_code_fence(s: &str) -> &str {
    if let Some(rest) = s.strip_prefix("```json") {
        return rest.trim_start().trim_end_matches("```").trim();
    }
    if let Some(rest) = s.strip_prefix("```") {
        return rest.trim_start().trim_end_matches("```").trim();
    }
    s
}

fn head(s: &str, n: usize) -> &str {
    let mut idx = n.min(s.len());
    while idx > 0 && !s.is_char_boundary(idx) {
        idx -= 1;
    }
    &s[..idx]
}

#[cfg(test)]
#[path = "synthesizer_tests.rs"]
mod tests;
