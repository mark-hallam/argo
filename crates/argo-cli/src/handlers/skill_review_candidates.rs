//! Memory-backed skill-candidate plumbing for `argo skill review`
//! (#697). Sibling of `skill_review.rs` so each file stays under
//! the 500-LOC budget as the candidate surface grows.
//!
//! Two helpers:
//!
//! - [`collect_memory_candidates`] — walks every loaded vertical
//!   in the discovery dir, queries vertical-namespaced memory
//!   for rows matching the `learned_skill_candidates` body
//!   convention, and returns them keyed by vertical_id (each
//!   row carries the parsed candidate so the caller doesn't
//!   re-parse). Strict isolation: a vertical never sees
//!   another vertical's candidates.
//!
//! - [`promote_candidate`] — reads one memory entry by id,
//!   parses it as a candidate, materialises a SKILL.md from
//!   the candidate fields, calls
//!   `SkillStore::propose_for_review` so the new row lands in
//!   `pending_review` (NEVER auto-active), and emits a
//!   `skill.candidate.promoted` audit row attributed to the
//!   originating session.
use argo_audit::{AuditKind, AuditSink};
use argo_memory::{parse_skill_candidate, MemoryEntry, MemoryStore, SkillCandidate};
use argo_skill::{Skill, SkillStore};
use argo_store::feedback::Vote;
use argo_types::{ArgoError, ArgoResult};
use std::sync::Arc;
use uuid::Uuid;

/// One candidate row surfaced to the operator: which vertical
/// the memory entry belongs to, the entry itself (for ids/
/// timestamps/source session), and the parsed candidate.
pub(super) struct CandidateRow {
    pub(super) vertical_id: String,
    pub(super) entry: MemoryEntry,
    pub(super) candidate: SkillCandidate,
}

#[derive(Clone, Copy)]
pub(super) struct PromotionFeedback<'a> {
    pub(super) action_id: &'a str,
    pub(super) vote: Vote,
}

/// Walk every loaded vertical + collect memory-backed candidates
/// scoped to its `vertical_id`. Returns an empty list when:
/// - the workspace has no rows yet (no first_workspace),
/// - the discovery dir is missing or unloadable,
/// - no vertical has any matching MEMORY.md entries.
///
/// Per-vertical query failures are silently dropped (an
/// individual vertical's memory schema being broken should not
/// blank the entire list). The aggregate sort is by
/// (vertical_id, learned_at) so output is deterministic.
pub(super) async fn collect_memory_candidates(
    skills: &SkillStore,
) -> ArgoResult<Vec<CandidateRow>> {
    let store = skills.store_arc();
    // #718: cwd-resolved workspace (matches every writer's path).
    // Errors here are folded into "no candidates" so a transient
    // workspace resolution issue doesn't blank the entire surface.
    let workspace = match crate::home::cwd_workspace(&store).await {
        Ok(w) => w,
        Err(_) => return Ok(Vec::new()),
    };
    let memory = MemoryStore::new(Arc::clone(&store));
    let dir = argo_vertical::default_dir();
    let (registry, _errs) = match argo_vertical::load_dir(&dir) {
        Ok(p) => p,
        Err(_) => return Ok(Vec::new()),
    };
    let mut out: Vec<CandidateRow> = Vec::new();
    for (vertical_id, _loaded) in registry.list() {
        let rows = match memory
            .list_skill_candidates_for_vertical(workspace, vertical_id)
            .await
        {
            Ok(r) => r,
            Err(_) => continue,
        };
        for (entry, candidate) in rows {
            out.push(CandidateRow {
                vertical_id: vertical_id.to_string(),
                entry,
                candidate,
            });
        }
    }
    out.sort_by(|a, b| {
        a.vertical_id
            .cmp(&b.vertical_id)
            .then_with(|| a.entry.learned_at.cmp(&b.entry.learned_at))
    });
    Ok(out)
}

/// Materialise one memory candidate into a `pending_review`
/// skill row + emit the audit event. Returns the new skill so
/// the CLI can echo the id back to the operator.
///
/// Fails loudly when:
/// - the memory entry id doesn't exist
/// - the entry has no `vertical_id` (workspace-global memory
///   entries can't be promoted — promotion must be vertical-
///   scoped to preserve isolation)
/// - the entry's body isn't a valid `learned_skill_candidates`
///   row
/// - the entry has no `learned_in` session (we attribute the
///   audit row to that session)
/// - a skill with the same name is already `Active`
///   (`propose_for_review` enforces uniqueness against the
///   active set; conflicts are operator-visible).
pub(super) async fn promote_candidate(
    skills: &SkillStore,
    memory_id_str: &str,
    feedback: Option<PromotionFeedback<'_>>,
) -> ArgoResult<Skill> {
    let memory_id = Uuid::parse_str(memory_id_str)
        .map_err(|e| ArgoError::Provider(format!("invalid memory id '{memory_id_str}': {e}")))?;
    let store = skills.store_arc();
    let memory = MemoryStore::new(Arc::clone(&store));
    let entry: MemoryEntry = memory.get(memory_id).await?;
    let vertical_id = entry.vertical_id.as_deref().ok_or_else(|| {
        ArgoError::Schema(
            "memory entry has no vertical_id — only vertical-scoped candidates are promotable (Rule 7 / #680)".into(),
        )
    })?;
    let candidate = parse_skill_candidate(&entry.body).ok_or_else(|| {
        ArgoError::Schema(
            "memory entry body is not a `learned_skill_candidates` row (need schema_key + skill_name)".into(),
        )
    })?;
    let learned_in = entry.learned_in.ok_or_else(|| {
        ArgoError::Schema(
            "memory entry has no `learned_in` session — promote must attribute the audit row to the originating session".into(),
        )
    })?;
    let raw = build_skill_md(&candidate, vertical_id, memory_id);
    let new_skill = skills
        .propose_for_review(&raw, learned_in, vertical_id)
        .await?;
    let audit = AuditSink::new(Arc::clone(&store));
    let mut payload = serde_json::json!({
        "skill_id": new_skill.id.0.to_string(),
        "name": new_skill.frontmatter.name,
        "vertical_id": vertical_id,
        "memory_entry_id": memory_id.to_string(),
    });
    if let Some(feedback) = feedback {
        payload["feedback"] = serde_json::json!({
            "action_id": feedback.action_id,
            "vote": feedback.vote.as_str(),
        });
    }
    audit
        .write(Some(learned_in), AuditKind::SkillCandidatePromoted, payload, None)
        .await?;
    Ok(new_skill)
}

/// Build a minimum-viable SKILL.md from a parsed candidate.
/// Frontmatter carries `name` + `version` + `description` +
/// `provenance` (source = memory_candidate so the chain shows
/// where it came from). Body is markdown stitched from the
/// candidate's optional fields — missing fields render as
/// "(not supplied)" rather than blank so the operator sees the
/// gap during review instead of silently dropping it.
fn build_skill_md(c: &SkillCandidate, vertical_id: &str, memory_id: Uuid) -> String {
    let description = c
        .problem
        .clone()
        .unwrap_or_else(|| format!("Promoted from memory candidate (vertical: {vertical_id})"));
    let problem = c.problem.as_deref().unwrap_or("(not supplied)");
    let pattern = c.reusable_pattern.as_deref().unwrap_or("(not supplied)");
    let evidence = c.evidence.as_deref().unwrap_or("(not supplied)");
    let expected_reuse = c.expected_reuse.as_deref().unwrap_or("(not supplied)");
    let risk = c.risk.as_deref().unwrap_or("(not supplied)");
    format!(
        "---\nname: {name}\nversion: \"0.1.0\"\ndescription: \"{description}\"\nprovenance:\n  source: memory_candidate\n  hash: sha256-pending\n---\n\n# {name}\n\nPromoted from memory-backed learned skill candidate (#697).\n- vertical: `{vertical_id}`\n- memory_entry: `{memory_id}`\n\n## Problem\n\n{problem}\n\n## Reusable pattern\n\n{pattern}\n\n## Evidence\n\n{evidence}\n\n## Expected reuse\n\n{expected_reuse}\n\n## Risk\n\n{risk}\n\n## Lifecycle\n\nThis skill is `pending_review`. Operator approval via\n`argo skill review approve <skill_id>` is required before the\nagent runtime treats it as active.\n",
        name = c.skill_name,
        description = description.replace('"', "'"),
    )
}

#[cfg(test)]
#[path = "skill_review_candidates_tests.rs"]
mod tests;
