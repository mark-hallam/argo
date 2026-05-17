//! Conservative prompt-skill relevance selector.
use crate::relevance_selector_markers::{marker_number, norm, parse_trust_label, values};
use crate::{LayeredSkill, Skill, SkillLayer, TrustGrade};
use std::cmp::Ordering;

/// Default maximum marketplace skills rendered into one prompt.
pub const DEFAULT_MAX_MARKETPLACE_SKILLS: usize = 5;
const DEFAULT_PROMPT_BUDGET_CHARS: usize = 12_000;

/// Operator-tunable selector policy.
#[derive(Debug, Clone, Copy)]
pub struct SkillRelevanceConfig {
    /// Maximum marketplace/library skills to render after filtering.
    pub max_marketplace_skills: usize,
    /// Minimum marketplace trust grade required for prompt exposure.
    pub min_trust_grade: TrustGrade,
    /// Approximate character budget for the rendered skill block.
    pub prompt_budget_chars: usize,
}

impl Default for SkillRelevanceConfig {
    fn default() -> Self {
        Self {
            max_marketplace_skills: DEFAULT_MAX_MARKETPLACE_SKILLS,
            min_trust_grade: TrustGrade::PublishedVerified,
            prompt_budget_chars: DEFAULT_PROMPT_BUDGET_CHARS,
        }
    }
}

impl SkillRelevanceConfig {
    /// Build config from operator-facing TOML labels.
    #[must_use]
    pub fn from_operator_config(max_marketplace_skills: usize, min_trust_grade: &str) -> Self {
        Self {
            max_marketplace_skills,
            min_trust_grade: parse_trust_label(min_trust_grade),
            ..Self::default()
        }
    }
}

/// Per-turn context used by the selector.
#[derive(Debug, Clone, Copy, Default)]
pub struct SkillSelectionContext<'a> {
    /// Current action type, when known by the caller.
    pub action_type: Option<&'a str>,
    /// Worker/session vertical id, when bound.
    pub vertical_id: Option<&'a str>,
    /// Lightweight task tags supplied by the caller.
    pub task_tags: &'a [&'a str],
}

/// Filters layered skill rows for prompt rendering.
#[derive(Debug, Clone, Copy, Default)]
pub struct SkillRelevanceSelector {
    config: SkillRelevanceConfig,
}

impl SkillRelevanceSelector {
    /// Construct a selector from explicit config.
    #[must_use]
    pub fn new(config: SkillRelevanceConfig) -> Self {
        Self { config }
    }

    /// Select skills in prompt order.
    #[must_use]
    pub fn select(&self, rows: Vec<LayeredSkill>, ctx: SkillSelectionContext<'_>) -> Vec<Skill> {
        let (mut workspace, mut marketplace) = (Vec::new(), Vec::new());
        for row in rows {
            match row.layer {
                SkillLayer::Workspace if workspace_matches(&row.skill, ctx.vertical_id) => {
                    workspace.push(row.skill);
                }
                SkillLayer::Library if self.marketplace_matches(&row.skill, &ctx) => {
                    marketplace.push(row.skill);
                }
                _ => {}
            }
        }
        marketplace.sort_by(compare_marketplace);
        let mut used_chars: usize = workspace.iter().map(rendered_prompt_chars).sum();
        let mut count = 0usize;
        for skill in marketplace {
            if count >= self.config.max_marketplace_skills {
                break;
            }
            let chars = rendered_prompt_chars(&skill);
            if used_chars.saturating_add(chars) > self.config.prompt_budget_chars {
                continue;
            }
            used_chars = used_chars.saturating_add(chars);
            count = count.saturating_add(1);
            workspace.push(skill);
        }
        workspace
    }

    fn marketplace_matches(&self, skill: &Skill, ctx: &SkillSelectionContext<'_>) -> bool {
        skill
            .frontmatter
            .provenance
            .source
            .starts_with("published:")
            && TrustGrade::from_source(&skill.frontmatter.provenance.source)
                <= self.config.min_trust_grade
            && vertical_matches(skill, ctx.vertical_id)
            && action_matches(skill, ctx.action_type)
            && tags_match(skill, ctx)
    }
}

fn workspace_matches(skill: &Skill, vertical_id: Option<&str>) -> bool {
    match vertical_id {
        Some(vid) => skill.vertical_id.as_deref() == Some(vid),
        None => true,
    }
}

fn vertical_matches(skill: &Skill, vertical_id: Option<&str>) -> bool {
    match vertical_id {
        Some(vid) => {
            skill.vertical_id.as_deref() == Some(vid)
                || values(skill, &["tag", "tags"]).contains("cross_vertical")
        }
        None => true,
    }
}

fn action_matches(skill: &Skill, action_type: Option<&str>) -> bool {
    let declared = values(skill, &["action_type", "action_types", "actions"]);
    declared.is_empty()
        || action_type
            .map(norm)
            .is_some_and(|action| declared.contains(&action))
}

fn tags_match(skill: &Skill, ctx: &SkillSelectionContext<'_>) -> bool {
    let tags = values(skill, &["tag", "tags"]);
    tags.is_empty()
        || ctx.task_tags.is_empty()
        || ctx
            .task_tags
            .iter()
            .map(|tag| norm(tag))
            .any(|tag| tags.contains(&tag))
}

fn compare_marketplace(a: &Skill, b: &Skill) -> Ordering {
    // Cap ordering: higher trust, then higher rating marker, then newer row.
    TrustGrade::from_source(&a.frontmatter.provenance.source)
        .cmp(&TrustGrade::from_source(&b.frontmatter.provenance.source))
        .then_with(|| {
            marker_number(b, &["rating", "score"]).cmp(&marker_number(a, &["rating", "score"]))
        })
        .then_with(|| b.created_at.cmp(&a.created_at))
        .then_with(|| a.frontmatter.name.cmp(&b.frontmatter.name))
}

fn rendered_prompt_chars(skill: &Skill) -> usize {
    skill.frontmatter.name.len()
        + skill.frontmatter.version.len()
        + skill.frontmatter.description.len()
        + skill.body.len()
        + 16
}
