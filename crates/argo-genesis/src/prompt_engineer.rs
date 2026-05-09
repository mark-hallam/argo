//! Iterative system-prompt engineering for Genesis-generated workers.
//!
//! The single-shot prompt writer stays deterministic. This module
//! layers a small revision loop on top of it so callers can collect
//! operator feedback, preserve approved `[KEEP]...[/KEEP]` spans, and
//! show a compact diff between revisions.

use crate::archetype::{classify_charter, Archetype};
use crate::error::GenesisError;
use crate::prompt_writer::build_system_prompt;
use std::collections::BTreeSet;

const MIN_INITIAL_PROMPT_WORDS: usize = 200;
const DEFAULT_PERSONA: &str = "genesis-worker";

/// In-memory state for one prompt-engineering conversation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromptEngineerSession {
    /// Operator-provided role description used for the initial prompt.
    pub role_description: String,
    /// Latest prompt text after applying all accepted revisions.
    pub current_prompt: String,
    /// Full revision history, including revision 0 from [`start`].
    pub history: Vec<PromptRevision>,
}

/// One generated prompt revision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromptRevision {
    /// Monotonic revision number. The initial prompt is revision 0.
    pub rev: u32,
    /// Complete prompt body for this revision.
    pub prompt: String,
    /// Operator feedback applied to produce this revision, if any.
    pub feedback: Option<String>,
    /// Verbatim `[KEEP]...[/KEEP]` spans known at this revision.
    pub locked_sections: Vec<String>,
}

/// Start a prompt-engineering session from a role description.
///
/// # Example
/// ```
/// # fn demo() -> Result<(), argo_genesis::GenesisError> {
/// let session = argo_genesis::prompt_engineer::start(
///     "be my customer support teammate for priority ticket triage",
/// )?;
/// assert_eq!(session.history[0].rev, 0);
/// # Ok(())
/// # }
/// ```
pub fn start(role_description: &str) -> Result<PromptEngineerSession, GenesisError> {
    let role = role_description.trim();
    if role.is_empty() {
        return Err(GenesisError::Validation {
            stage: "prompt_engineer.start",
            detail: "role_description is empty".into(),
        });
    }
    let archetype = classify_charter(role).unwrap_or(Archetype::ResearchAnalyst);
    let mut prompt = build_system_prompt(role, archetype, DEFAULT_PERSONA);
    ensure_initial_prompt_budget(&mut prompt);
    let locked_sections = extract_locked_sections(&prompt);
    let initial = PromptRevision {
        rev: 0,
        prompt: prompt.clone(),
        feedback: None,
        locked_sections,
    };
    Ok(PromptEngineerSession {
        role_description: role.to_string(),
        current_prompt: prompt,
        history: vec![initial],
    })
}

/// Revise a session's current prompt using operator feedback.
///
/// `[KEEP]...[/KEEP]` sections in the current prompt or any prior
/// revision are carried forward verbatim.
///
/// # Example
/// ```
/// # fn demo() -> Result<(), argo_genesis::GenesisError> {
/// let mut session = argo_genesis::prompt_engineer::start(
///     "be my marketing intern and draft campaign plans",
/// )?;
/// let revision = argo_genesis::prompt_engineer::revise(&mut session, "shorter")?;
/// assert_eq!(revision.rev, 1);
/// # Ok(())
/// # }
/// ```
pub fn revise<'a>(
    session: &'a mut PromptEngineerSession,
    feedback: &str,
) -> Result<&'a PromptRevision, GenesisError> {
    let feedback = feedback.trim();
    if feedback.is_empty() {
        return Err(GenesisError::Validation {
            stage: "prompt_engineer.revise",
            detail: "feedback is empty".into(),
        });
    }

    let locked_sections = collect_locked_sections(session);
    let prompt = if asks_for_shorter(feedback) {
        terse_revision(&session.role_description, feedback, &locked_sections)
    } else {
        guided_revision(&session.current_prompt, feedback, &locked_sections)
    };
    let rev = session.history.last().map_or(0, |last| last.rev + 1);
    session.current_prompt = prompt.clone();
    session.history.push(PromptRevision {
        rev,
        prompt,
        feedback: Some(feedback.to_string()),
        locked_sections,
    });
    session
        .history
        .last()
        .ok_or_else(|| GenesisError::Validation {
            stage: "prompt_engineer.revise",
            detail: "revision history unexpectedly empty".into(),
        })
}

/// Render a compact unified-diff-style view between two revisions.
///
/// # Example
/// ```
/// # fn demo() -> Result<(), argo_genesis::GenesisError> {
/// let mut session = argo_genesis::prompt_engineer::start(
///     "be my support teammate and triage customer tickets",
/// )?;
/// let before = session.history[0].clone();
/// let after = argo_genesis::prompt_engineer::revise(&mut session, "shorter")?.clone();
/// let rendered = argo_genesis::prompt_engineer::diff(&before, &after);
/// assert!(rendered.contains("--- rev 0"));
/// # Ok(())
/// # }
/// ```
#[must_use]
pub fn diff(prev: &PromptRevision, next: &PromptRevision) -> String {
    let prev_lines: BTreeSet<&str> = prev.prompt.lines().collect();
    let next_lines: BTreeSet<&str> = next.prompt.lines().collect();
    let mut out = format!("--- rev {}\n+++ rev {}\n", prev.rev, next.rev);
    for line in prev
        .prompt
        .lines()
        .filter(|line| !next_lines.contains(line))
    {
        out.push_str("- ");
        out.push_str(line);
        out.push('\n');
    }
    for line in next
        .prompt
        .lines()
        .filter(|line| !prev_lines.contains(line))
    {
        out.push_str("+ ");
        out.push_str(line);
        out.push('\n');
    }
    if out.lines().count() == 2 {
        out.push_str("  <no textual changes>\n");
    }
    out
}

fn ensure_initial_prompt_budget(prompt: &mut String) {
    while word_count(prompt) < MIN_INITIAL_PROMPT_WORDS {
        prompt.push_str(
            "\n## Operating detail\n\n\
             Turn the role description into repeatable worker behavior: name the immediate \
             task, identify the evidence required before acting, prefer drafts before \
             external changes, and ask the operator for approval when the action could \
             send, publish, delete, pay, share, or message. Keep memory updates specific, \
             cite source material, and escalate gracefully when the charter is ambiguous.\n",
        );
    }
}

fn collect_locked_sections(session: &PromptEngineerSession) -> Vec<String> {
    let mut locked = Vec::new();
    for revision in &session.history {
        append_unique(&mut locked, revision.locked_sections.iter().cloned());
    }
    append_unique(
        &mut locked,
        extract_locked_sections(&session.current_prompt),
    );
    locked
}

fn terse_revision(role_description: &str, feedback: &str, locked_sections: &[String]) -> String {
    let role_summary = first_words(role_description, 70);
    let mut out = format!(
        "# System prompt - concise worker\n\n\
         You are a Skycot worker generated from this role description:\n\n\
         {role_summary}\n\n\
         ## Operating rules\n\n\
         - Stay inside the role charter.\n\
         - Cite evidence before making claims.\n\
         - Ask before sensitive actions: send, post, pay, publish, delete, share, message.\n\
         - Escalate when the charter is ambiguous.\n\n\
         ## Revision guidance\n\n\
         Apply operator feedback: {feedback}. Favor short plans, direct wording, and only \
         the context needed for the current task.\n",
    );
    ensure_locked_sections(&mut out, locked_sections);
    out
}

fn guided_revision(current_prompt: &str, feedback: &str, locked_sections: &[String]) -> String {
    let mut out = format!(
        "{}\n\n## Revision guidance\n\n\
         Operator feedback: {feedback}\n\n\
         Apply this guidance to future behavior while preserving approved wording and \
         the existing safety contract.\n",
        current_prompt.trim(),
    );
    ensure_locked_sections(&mut out, locked_sections);
    out
}

fn ensure_locked_sections(prompt: &mut String, locked_sections: &[String]) {
    let missing: Vec<&String> = locked_sections
        .iter()
        .filter(|section| !prompt.contains(section.as_str()))
        .collect();
    if missing.is_empty() {
        return;
    }
    prompt.push_str("\n## Locked sections\n\n");
    for section in missing {
        prompt.push_str(section);
        prompt.push_str("\n\n");
    }
}

fn asks_for_shorter(feedback: &str) -> bool {
    let lowered = feedback.to_lowercase();
    ["shorter", "terse", "concise", "trim", "brief"]
        .iter()
        .any(|needle| lowered.contains(needle))
}

fn extract_locked_sections(prompt: &str) -> Vec<String> {
    let start_marker = "[KEEP]";
    let end_marker = "[/KEEP]";
    let mut locked = Vec::new();
    let mut offset = 0;
    while let Some(relative_start) = prompt[offset..].find(start_marker) {
        let start = offset + relative_start;
        let body_start = start + start_marker.len();
        let Some(relative_end) = prompt[body_start..].find(end_marker) else {
            break;
        };
        let end = body_start + relative_end + end_marker.len();
        append_unique(&mut locked, std::iter::once(prompt[start..end].to_string()));
        offset = end;
    }
    locked
}

fn append_unique(out: &mut Vec<String>, values: impl IntoIterator<Item = String>) {
    for value in values {
        if !out.iter().any(|existing| existing == &value) {
            out.push(value);
        }
    }
}

fn first_words(text: &str, limit: usize) -> String {
    let words: Vec<&str> = text.split_whitespace().take(limit).collect();
    words.join(" ")
}

fn word_count(text: &str) -> usize {
    text.split_whitespace().count()
}

#[cfg(test)]
#[path = "prompt_engineer_tests.rs"]
mod tests;
