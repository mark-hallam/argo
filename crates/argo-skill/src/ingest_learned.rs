//! Bridge that mirrors a [`LearnedPlanTemplateRow`] into a row in
//! the `SkillStore` so promoted Dreams-distilled templates
//! participate in the `argo-skill::trust::TrustGrade` ladder.
//!
//! Brief: `mission-learned-plan-template-trust-ladder`. This is
//! the runtime half of the
//! [`argo_store::learned_plan_template::SkillIngestSink`] trait —
//! the concrete impl in `argo-cli` calls
//! [`from_learned_plan_template`] from
//! `SkillIngestSink::ingest_from_template`, and
//! [`demote_learned_template`] from
//! `SkillIngestSink::demote_from_template`.
//!
//! Naming convention: ingested skills are named
//! `learned:<template_id>` so downstream tools can pattern-match
//! to distinguish hand-authored from Dreams-promoted entries.
use argo_store::learned_plan_template::LearnedPlanTemplateRow;
use argo_types::{ArgoResult, SkillId};

use crate::store::SkillStore;
use crate::trust::TrustGrade;

/// Prefix added to the template id when synthesising a skill name.
/// Stable wire string — operators query the catalog with this
/// prefix to enumerate Dreams-promoted entries.
pub const LEARNED_SKILL_NAME_PREFIX: &str = "learned:";

/// Default version stamped onto a freshly ingested skill row.
const INITIAL_LEARNED_SKILL_VERSION: &str = "0.1";

/// Synthesise (or refresh) the skill row mirrored from `row`.
///
/// - Fresh ingest: inserts a new skill at
///   `name = "learned:<row.id>"`, version `0.1`, provenance source
///   chosen so [`TrustGrade::from_source`] maps it to `grade`.
/// - Re-ingest of the same template id: supersedes the existing
///   active row (bumping the trailing version segment), so the
///   `SkillStore::find_active_by_name` lookup keeps returning the
///   newest mirror — never two rows.
pub async fn from_learned_plan_template(
    row: &LearnedPlanTemplateRow,
    skills: &SkillStore,
    grade: TrustGrade,
) -> ArgoResult<SkillId> {
    let name = skill_name_for_template(&row.id);
    let prior = skills.find_active_by_name(&name).await?;
    let next_version = prior
        .as_ref()
        .map(|s| bump_version(&s.frontmatter.version))
        .unwrap_or_else(|| INITIAL_LEARNED_SKILL_VERSION.to_string());
    let raw = synthetic_skill_md(&name, &next_version, row, grade);
    let installed = match prior {
        None => skills.install(&raw, None).await?,
        Some(_) => {
            skills
                .supersede(&raw, None, crate::skill::PrincipalOverride::Force)
                .await?
        }
    };
    Ok(installed.id)
}

/// Demote the mirrored skill for `row.id` to the lowest trust rung
/// (`TrustGrade::LearnedUnreviewed`). The skill is preserved (not
/// deleted) so an operator can re-promote later.
///
/// No-op when no mirror exists. Returns `Some(id)` of the new
/// supersession row when a demote happened, or `None` when nothing
/// was mirrored.
pub async fn demote_learned_template(
    row: &LearnedPlanTemplateRow,
    skills: &SkillStore,
) -> ArgoResult<Option<SkillId>> {
    let name = skill_name_for_template(&row.id);
    let Some(prior) = skills.find_active_by_name(&name).await? else {
        return Ok(None);
    };
    if TrustGrade::from_source(&prior.frontmatter.provenance.source)
        == TrustGrade::LearnedUnreviewed
    {
        return Ok(Some(prior.id));
    }
    let next_version = bump_version(&prior.frontmatter.version);
    let raw = synthetic_skill_md(&name, &next_version, row, TrustGrade::LearnedUnreviewed);
    let new_skill = skills
        .supersede(&raw, None, crate::skill::PrincipalOverride::Force)
        .await?;
    Ok(Some(new_skill.id))
}

/// Stable name applied to the mirrored skill row.
#[must_use]
pub fn skill_name_for_template(template_id: &str) -> String {
    format!("{LEARNED_SKILL_NAME_PREFIX}{template_id}")
}

fn synthetic_skill_md(
    name: &str,
    version: &str,
    row: &LearnedPlanTemplateRow,
    grade: TrustGrade,
) -> String {
    let source = provenance_source_for(grade);
    let description = escape_yaml_scalar(&row.description);
    let parent = escape_inline(&row.parent_goal);
    let yaml = row.yaml.replace("\r\n", "\n");
    format!(
        "---\nname: {name}\nversion: \"{version}\"\ndescription: \"{description}\"\nprovenance:\n  source: {source}\n  hash: sha256-pending\n---\n# {name}\n\nDistilled from cohort of {cohort} successful Planner decomposition(s) of:\n\n> {parent}\n\nKeywords: `{keywords}`.\n\n```yaml\n{yaml}\n```\n",
        cohort = row.cohort_size,
        keywords = row.keywords_csv,
    )
}

fn provenance_source_for(grade: TrustGrade) -> &'static str {
    match grade {
        TrustGrade::Builtin => "builtin",
        TrustGrade::PublishedVerified => "published:verified:dreams",
        TrustGrade::Tenant => "forge",
        TrustGrade::PublishedCommunity => "published:community:dreams",
        TrustGrade::LearnedUnreviewed => "learned",
    }
}

/// Bump the trailing numeric segment of a version (`"0.1"` → `"0.2"`)
/// so the `UNIQUE(name, version)` skill constraint never collides
/// on repeat ingest. Non-numeric tails get a `-bumped` suffix.
fn bump_version(v: &str) -> String {
    let mut parts: Vec<String> = v.split('.').map(String::from).collect();
    if let Some(last) = parts.last_mut() {
        if let Ok(n) = last.parse::<u64>() {
            *last = (n + 1).to_string();
            return parts.join(".");
        }
    }
    format!("{v}-bumped")
}

fn escape_yaml_scalar(s: &str) -> String {
    s.replace('"', "\\\"").replace('\n', " ")
}

fn escape_inline(s: &str) -> String {
    s.replace('\n', " ")
}

#[cfg(test)]
#[path = "ingest_learned_tests.rs"]
mod tests;
