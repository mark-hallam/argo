//! Skill-to-skill dependency extraction + DAG building.
//!
//! v0 patterns (regex-light, no new deps; false negatives over
//! false positives):
//! - `argo skill run NAME` / `argo skill use NAME` → `Calls`
//! - `` `skill:NAME` `` → `References`
//! - `extends:` reserved (no typed field on `SkillFrontmatter`
//!   today; follow-up brief).

use crate::skill::SkillFrontmatter;
use crate::store::SkillStore;
use argo_types::{ArgoError, ArgoResult};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashSet, VecDeque};

/// Default depth cap when callers don't set one.
pub const DEFAULT_MAX_DEPTH: u32 = 5;

/// Kind of dependency between two skills.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum RefKind {
    /// Explicit invocation (`argo skill run X`).
    Calls,
    /// Marker mention (`` `skill:X` ``).
    References,
    /// Frontmatter `extends:` field. Reserved — not emitted
    /// in v0 because [`SkillFrontmatter`] does not currently
    /// surface the field.
    Extends,
}

impl RefKind {
    /// Stable string label for the kind, matching serde's
    /// snake_case form. Used by the CLI tree renderer.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Calls => "calls",
            Self::References => "references",
            Self::Extends => "extends",
        }
    }
}

/// A single reference extracted from a skill body.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillRef {
    /// Name of the referenced skill (raw, unresolved).
    pub target_name: String,
    /// How the body referred to it.
    pub kind: RefKind,
}

/// One edge in the dependency DAG.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DepEdge {
    /// Source skill name.
    pub from: String,
    /// Target skill name.
    pub to: String,
    /// Why the edge exists.
    pub kind: RefKind,
}

/// Resolved dependency graph for a root skill.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DepGraph {
    /// Root skill the walk started from.
    pub root: String,
    /// Edges discovered by the walk. Order is BFS-stable for
    /// snapshot tests.
    pub edges: Vec<DepEdge>,
    /// Maximum depth reached during the walk.
    pub depth: u32,
    /// Skill names where the walker hit an already-visited
    /// node and stopped recursing. Markdown renderer annotates
    /// these with `(cycle)`.
    pub cycles: Vec<String>,
}

/// Extract every skill reference from `body`. `frontmatter`
/// is accepted for forward-compatibility (an `extends:` field
/// would be sourced from it) but is currently unused.
///
/// Order of returned refs is deterministic: matches the
/// scanning order through the body text.
#[must_use]
pub fn extract_refs(body: &str, _frontmatter: &SkillFrontmatter) -> Vec<SkillRef> {
    let mut out = Vec::new();
    let mut seen = BTreeSet::<(String, RefKind)>::new();
    let mut push = |target: &str, kind: RefKind, sink: &mut Vec<SkillRef>| {
        let cleaned = target.trim_matches(|c: char| {
            c.is_whitespace() || c == '`' || c == ',' || c == '.' || c == ';' || c == ')'
        });
        if cleaned.is_empty() {
            return;
        }
        let key = (cleaned.to_string(), kind);
        if seen.insert(key.clone()) {
            sink.push(SkillRef {
                target_name: cleaned.to_string(),
                kind,
            });
        }
    };
    // Pattern 1: `argo skill (run|use) NAME`.
    for token in ["argo skill run ", "argo skill use "] {
        let mut rest = body;
        while let Some(idx) = rest.find(token) {
            let after = &rest[idx + token.len()..];
            let end = after
                .find(|c: char| c.is_whitespace() || c == '`')
                .unwrap_or(after.len());
            push(&after[..end], RefKind::Calls, &mut out);
            rest = &after[end..];
        }
    }
    // Pattern 2: `` `skill:NAME` ``.
    let mut rest = body;
    while let Some(idx) = rest.find("`skill:") {
        let after = &rest[idx + "`skill:".len()..];
        let end = after.find('`').unwrap_or(after.len());
        push(&after[..end], RefKind::References, &mut out);
        rest = &after[end..];
    }
    out
}

/// Build the forward dependency graph rooted at `root`.
/// Walks BFS up to `max_depth`. Names that resolve to a missing
/// skill are still emitted as edges (with a synthetic empty
/// target body — the operator wants to see the broken edge).
pub async fn build_graph(
    store: &SkillStore,
    root: &str,
    max_depth: u32,
) -> ArgoResult<DepGraph> {
    let depth_cap = if max_depth == 0 { DEFAULT_MAX_DEPTH } else { max_depth };
    let _ = store
        .find_active_by_name(root)
        .await?
        .ok_or_else(|| ArgoError::Provider(format!("no active skill named '{root}'")))?;
    let mut edges = Vec::new();
    let mut cycles_set = BTreeSet::<String>::new();
    let mut visited = HashSet::<String>::new();
    visited.insert(root.to_string());
    let mut queue: VecDeque<(String, u32)> = VecDeque::new();
    queue.push_back((root.to_string(), 0));
    let mut max_observed = 0u32;
    while let Some((name, depth)) = queue.pop_front() {
        max_observed = max_observed.max(depth);
        if depth >= depth_cap {
            continue;
        }
        let Some(skill) = store.find_active_by_name(&name).await? else {
            continue;
        };
        for r in extract_refs(&skill.body, &skill.frontmatter) {
            edges.push(DepEdge {
                from: name.clone(),
                to: r.target_name.clone(),
                kind: r.kind,
            });
            if !visited.insert(r.target_name.clone()) {
                cycles_set.insert(r.target_name);
                continue;
            }
            queue.push_back((r.target_name, depth + 1));
        }
    }
    Ok(DepGraph {
        root: root.to_string(),
        edges,
        depth: max_observed,
        cycles: cycles_set.into_iter().collect(),
    })
}

/// Build the reverse dependency graph: skills that mention
/// `target` (transitively up to `max_depth`). Useful for
/// "what would break if I rolled back X?"
pub async fn build_reverse_graph(
    store: &SkillStore,
    target: &str,
    max_depth: u32,
) -> ArgoResult<DepGraph> {
    let depth_cap = if max_depth == 0 { DEFAULT_MAX_DEPTH } else { max_depth };
    let _ = store
        .find_active_by_name(target)
        .await?
        .ok_or_else(|| ArgoError::Provider(format!("no active skill named '{target}'")))?;
    let active = store.list_active().await?;
    // Pre-compute every skill's outgoing edges so we can flip
    // them in O(N*M) rather than re-scanning per BFS layer.
    let mut by_target: std::collections::BTreeMap<String, Vec<(String, RefKind)>> =
        std::collections::BTreeMap::new();
    for skill in &active {
        let from = skill.frontmatter.name.clone();
        for r in extract_refs(&skill.body, &skill.frontmatter) {
            by_target
                .entry(r.target_name)
                .or_default()
                .push((from.clone(), r.kind));
        }
    }
    let mut edges = Vec::new();
    let mut cycles_set = BTreeSet::<String>::new();
    let mut visited = HashSet::<String>::new();
    visited.insert(target.to_string());
    let mut queue: VecDeque<(String, u32)> = VecDeque::new();
    queue.push_back((target.to_string(), 0));
    let mut max_observed = 0u32;
    while let Some((name, depth)) = queue.pop_front() {
        max_observed = max_observed.max(depth);
        if depth >= depth_cap {
            continue;
        }
        let Some(parents) = by_target.get(&name) else {
            continue;
        };
        for (from, kind) in parents {
            edges.push(DepEdge {
                from: from.clone(),
                to: name.clone(),
                kind: *kind,
            });
            if !visited.insert(from.clone()) {
                cycles_set.insert(from.clone());
                continue;
            }
            queue.push_back((from.clone(), depth + 1));
        }
    }
    Ok(DepGraph {
        root: target.to_string(),
        edges,
        depth: max_observed,
        cycles: cycles_set.into_iter().collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skill::Provenance;
    use argo_types::{CapabilitySet, Lock};

    fn fm(name: &str) -> SkillFrontmatter {
        SkillFrontmatter {
            name: name.into(),
            version: "0.1".into(),
            description: String::new(),
            capabilities: CapabilitySet::default(),
            provenance: Provenance {
                source: "user".into(),
                hash: "sha256-x".into(),
            },
            lock: Lock::None,
        }
    }

    #[test]
    fn extract_refs_finds_run_and_use_invocations() {
        let body = "Step 1: argo skill run mailer\nStep 2: argo skill use audit-export";
        let refs = extract_refs(body, &fm("root"));
        let names: Vec<&str> = refs.iter().map(|r| r.target_name.as_str()).collect();
        assert_eq!(names, vec!["mailer", "audit-export"]);
        assert!(refs.iter().all(|r| r.kind == RefKind::Calls));
    }

    #[test]
    fn extract_refs_finds_marker_mentions() {
        let body = "see `skill:fancy-helper` for details";
        let refs = extract_refs(body, &fm("root"));
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].target_name, "fancy-helper");
        assert_eq!(refs[0].kind, RefKind::References);
    }

    #[test]
    fn extract_refs_dedupes_same_kind() {
        let body =
            "argo skill run mailer\nlater: argo skill run mailer\nand also `skill:mailer`";
        let refs = extract_refs(body, &fm("root"));
        // Calls and References to "mailer" both surface (different
        // kinds), but the duplicate Calls is dropped.
        assert_eq!(refs.len(), 2);
        assert!(refs.iter().any(|r| r.kind == RefKind::Calls));
        assert!(refs.iter().any(|r| r.kind == RefKind::References));
    }

    #[test]
    fn extract_refs_strips_trailing_punctuation() {
        let body = "first argo skill run mailer, then argo skill use audit-export.";
        let refs = extract_refs(body, &fm("root"));
        let names: Vec<&str> = refs.iter().map(|r| r.target_name.as_str()).collect();
        assert_eq!(names, vec!["mailer", "audit-export"]);
    }

    #[test]
    fn extract_refs_ignores_partial_matches() {
        let body = "skill:not-in-backticks should not match";
        let refs = extract_refs(body, &fm("root"));
        assert!(refs.is_empty());
    }

    #[test]
    fn ref_kind_as_str_matches_serde() {
        for k in [RefKind::Calls, RefKind::References, RefKind::Extends] {
            let json = serde_json::to_string(&k).unwrap();
            // serde produces "calls" with quotes; as_str returns
            // the bare token. Strip quotes for comparison.
            assert_eq!(json.trim_matches('"'), k.as_str());
        }
    }
}
