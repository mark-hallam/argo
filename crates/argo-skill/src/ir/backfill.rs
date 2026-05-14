//! One-shot backfill of installed skills into the canonical
//! [`SkillGraph`] IR (Track L brief `L3-skill-ir-backfill`).
//!
//! Every existing skill produces a *minimal* graph: one [`NodeKind::Action`]
//! whose `instruction` is the skill body verbatim. The node carries
//! curator-authored annotations (`confidence: 1.0`, `sample_count: 0`,
//! `policy: auto`); graph metadata mirrors the skill's frontmatter and
//! tags `source: "backfill"` so downstream consumers can distinguish
//! synthesised IR from canvas-authored or Ghostwriter-emitted graphs.
//!
//! [`backfill`] iterates the supplied skills in order. For each one it
//! synthesises the graph, looks up `(tenant, skill_id, version)` in the
//! [`SkillIrStore`], and skips when a row already exists — re-running
//! the backfill produces zero writes (acceptance criterion 2). With
//! `dry_run` the function never writes; the per-skill classification is
//! still produced so the operator sees what *would* be backfilled.
use crate::ir::store::{SkillIrStore as _, SqliteSkillIrStore};
use crate::ir::{Node, NodeAnnotations, NodeId, NodeKind, NodePolicy, SkillGraph, SkillGraphMetadata};
use crate::skill::Skill;
use argo_types::{ArgoResult, SkillId, TenantId};

/// Marker recorded on every backfilled graph's metadata so downstream
/// consumers can filter synthesised IR from canvas-authored or
/// Ghostwriter-emitted graphs.
pub const BACKFILL_SOURCE: &str = "backfill";

/// Stable id of the single action node every backfilled graph contains.
pub const BACKFILL_ENTRY_NODE_ID: &str = "n1";

/// Per-skill classification emitted by [`backfill`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackfillStatus {
    /// IR was synthesised and persisted (or would have been, under `--dry-run`).
    Backfilled,
    /// `(tenant, skill_id, version)` already had an IR row; left untouched.
    Skipped,
}

/// One row of the backfill report.
#[derive(Debug, Clone)]
pub struct BackfillEntry {
    /// Skill identity from the source row.
    pub skill_id: SkillId,
    /// Frontmatter `name`.
    pub name: String,
    /// Frontmatter `version`.
    pub version: String,
    /// Whether the row was backfilled or skipped.
    pub status: BackfillStatus,
}

/// Aggregate counts produced by [`backfill`].
#[derive(Debug, Clone, Default)]
pub struct BackfillReport {
    /// Number of skills the run persisted (or would have, with `dry_run`).
    pub backfilled: usize,
    /// Number of skills skipped because an IR row already existed.
    pub skipped: usize,
    /// One [`BackfillEntry`] per processed skill, in input order.
    pub entries: Vec<BackfillEntry>,
}

/// Synthesise the minimal IR for one installed skill.
///
/// The graph has a single [`NodeKind::Action`] whose `instruction` is
/// `skill.body` verbatim. The node's [`NodeAnnotations`] are populated
/// with curator-defaults (`confidence: 1.0`, `sample_count: 0`,
/// `policy: auto`). [`SkillGraphMetadata`] copies `name`, `version`,
/// `description` from frontmatter and stamps `source: "backfill"`.
///
/// # Example
/// ```
/// use argo_skill::ir::backfill::synthesize_graph;
/// use argo_skill::{Skill, SkillFrontmatter, Provenance};
/// use argo_skill::LifecycleState;
/// use argo_types::SkillId;
/// use chrono::Utc;
/// let skill = Skill {
///     id: SkillId::new(),
///     frontmatter: SkillFrontmatter {
///         name: "demo".into(),
///         version: "0.1".into(),
///         description: "demo skill".into(),
///         capabilities: Default::default(),
///         provenance: Provenance { source: "builtin".into(), hash: "sha256-x".into() },
///         lock: Default::default(),
///         evals: vec![],
///     },
///     body: "do the thing".into(),
///     hash: [0u8; 32],
///     learned_in: None,
///     superseded_by: None,
///     created_at: Utc::now(),
///     vertical_id: None,
///     lifecycle_state: LifecycleState::Active,
/// };
/// let g = synthesize_graph(&skill);
/// assert_eq!(g.nodes.len(), 1);
/// assert_eq!(g.metadata.source.as_deref(), Some("backfill"));
/// ```
#[must_use]
pub fn synthesize_graph(skill: &Skill) -> SkillGraph {
    let entry = NodeId::from(BACKFILL_ENTRY_NODE_ID);
    let node = Node {
        id: entry.clone(),
        label: skill.frontmatter.name.clone(),
        kind: NodeKind::Action {
            instruction: skill.body.clone(),
        },
        annotations: NodeAnnotations {
            confidence: Some(1.0),
            sample_count: Some(0),
            policy: Some(NodePolicy::Auto),
            ..NodeAnnotations::default()
        },
    };
    SkillGraph {
        entry,
        nodes: vec![node],
        edges: vec![],
        variables: vec![],
        metadata: SkillGraphMetadata {
            name: Some(skill.frontmatter.name.clone()),
            version: Some(skill.frontmatter.version.clone()),
            description: Some(skill.frontmatter.description.clone()),
            source: Some(BACKFILL_SOURCE.into()),
        },
    }
}

/// Backfill the supplied `skills` into `ir` under `tenant`.
///
/// Skills whose `(tenant, skill_id, version)` already has an IR row are
/// classified [`BackfillStatus::Skipped`] (idempotency). With `dry_run`,
/// no rows are written, but the same classifications are produced so
/// the caller can preview the run.
pub async fn backfill(
    skills: &[Skill],
    ir: &SqliteSkillIrStore<'_>,
    tenant: TenantId,
    dry_run: bool,
) -> ArgoResult<BackfillReport> {
    let mut report = BackfillReport::default();
    for skill in skills {
        let version = skill.frontmatter.version.clone();
        let already = ir.get(tenant, skill.id, &version).await?.is_some();
        let status = if already {
            report.skipped += 1;
            BackfillStatus::Skipped
        } else {
            if !dry_run {
                let graph = synthesize_graph(skill);
                ir.put(tenant, skill.id, &version, &graph).await?;
            }
            report.backfilled += 1;
            BackfillStatus::Backfilled
        };
        report.entries.push(BackfillEntry {
            skill_id: skill.id,
            name: skill.frontmatter.name.clone(),
            version,
            status,
        });
    }
    Ok(report)
}

#[cfg(test)]
#[path = "backfill_tests.rs"]
mod backfill_tests;
