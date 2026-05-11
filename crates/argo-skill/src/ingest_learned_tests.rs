use std::sync::Arc;

use argo_store::learned_plan_template::{insert, LearnedPlanTemplateRow, NewLearnedPlanTemplate};
use argo_store::SessionStore;

use super::{
    demote_learned_template, from_learned_plan_template, skill_name_for_template,
    LEARNED_SKILL_NAME_PREFIX,
};
use crate::store::SkillStore;
use crate::trust::TrustGrade;

async fn fresh_skills_and_seed() -> (SkillStore, LearnedPlanTemplateRow) {
    let session = Arc::new(SessionStore::open(":memory:").await.expect("open store"));
    session.migrate().await.expect("migrate");
    let id = insert(
        session.pool(),
        &NewLearnedPlanTemplate {
            name: "marketing-launch-distilled",
            description: "Distilled from a cohort of 3 marketing-launch decompositions.",
            parent_goal: "promote our blog post launch",
            keywords_csv: "promote,launch,announce",
            yaml: "id: marketing-launch-distilled\ntasks:\n  - id: draft\n    worker: \"\"",
            cohort_size: 3,
        },
    )
    .await
    .expect("insert seed");
    let row = argo_store::learned_plan_template::get(session.pool(), &id)
        .await
        .expect("get seed")
        .expect("seed row exists");
    let skills = SkillStore::new(session);
    (skills, row)
}

#[tokio::test]
async fn fresh_ingest_creates_skill() {
    let (skills, row) = fresh_skills_and_seed().await;
    let new_id = from_learned_plan_template(&row, &skills, TrustGrade::Tenant)
        .await
        .expect("ingest");
    let name = skill_name_for_template(&row.id);
    let installed = skills
        .find_active_by_name(&name)
        .await
        .expect("lookup")
        .expect("skill row exists");
    assert_eq!(installed.id, new_id);
    assert_eq!(installed.frontmatter.name, name);
    assert!(
        installed.body.contains("Distilled from cohort of 3"),
        "body should embed the cohort marker, got: {}",
        installed.body
    );
}

#[tokio::test]
async fn re_ingest_supersedes_does_not_dup() {
    let (skills, row) = fresh_skills_and_seed().await;
    from_learned_plan_template(&row, &skills, TrustGrade::Tenant)
        .await
        .expect("first ingest");
    from_learned_plan_template(&row, &skills, TrustGrade::Tenant)
        .await
        .expect("second ingest");
    let name = skill_name_for_template(&row.id);
    let active = skills.list_active().await.expect("list_active");
    let matches: Vec<_> = active
        .iter()
        .filter(|s| s.frontmatter.name == name)
        .collect();
    assert_eq!(
        matches.len(),
        1,
        "re-ingest must keep exactly one active row, got {}",
        matches.len()
    );
    let versions = skills
        .versions_for_name(&name)
        .await
        .expect("versions_for_name");
    assert_eq!(versions.len(), 2, "supersession history preserved");
    assert_eq!(matches[0].frontmatter.version, "0.2");
}

#[tokio::test]
async fn namespace_prefix_lands_in_name() {
    let (skills, row) = fresh_skills_and_seed().await;
    from_learned_plan_template(&row, &skills, TrustGrade::Tenant)
        .await
        .expect("ingest");
    let active = skills.list_active().await.expect("list_active");
    let learned: Vec<&str> = active
        .iter()
        .map(|s| s.frontmatter.name.as_str())
        .filter(|n| n.starts_with(LEARNED_SKILL_NAME_PREFIX))
        .collect();
    assert_eq!(learned.len(), 1);
    assert!(learned[0].starts_with("learned:"));
}

#[tokio::test]
async fn default_trust_grade_is_tenant() {
    let (skills, row) = fresh_skills_and_seed().await;
    from_learned_plan_template(&row, &skills, TrustGrade::Tenant)
        .await
        .expect("ingest");
    let installed = skills
        .find_active_by_name(&skill_name_for_template(&row.id))
        .await
        .expect("lookup")
        .expect("skill exists");
    assert_eq!(
        TrustGrade::from_source(&installed.frontmatter.provenance.source),
        TrustGrade::Tenant,
        "default ingest grade should round-trip back to Tenant"
    );
}

#[tokio::test]
async fn reject_demotes_existing_skill_not_deletes() {
    let (skills, row) = fresh_skills_and_seed().await;
    from_learned_plan_template(&row, &skills, TrustGrade::Tenant)
        .await
        .expect("ingest");
    let demoted = demote_learned_template(&row, &skills)
        .await
        .expect("demote")
        .expect("demote returned Some");
    let active = skills
        .find_active_by_name(&skill_name_for_template(&row.id))
        .await
        .expect("lookup")
        .expect("skill still present");
    assert_eq!(active.id, demoted, "active row should be the new demoted one");
    assert_eq!(
        TrustGrade::from_source(&active.frontmatter.provenance.source),
        TrustGrade::LearnedUnreviewed,
        "post-demote grade should be the lowest rung"
    );
}

#[tokio::test]
async fn reject_when_no_skill_ingested_is_noop() {
    let (skills, row) = fresh_skills_and_seed().await;
    let outcome = demote_learned_template(&row, &skills)
        .await
        .expect("demote no-op");
    assert!(
        outcome.is_none(),
        "demote against a never-promoted template must be a no-op"
    );
    let active = skills.list_active().await.expect("list_active");
    assert!(
        active.is_empty(),
        "no skill rows should have been created by the no-op demote"
    );
}

#[tokio::test]
async fn reject_when_already_lowest_grade_is_noop() {
    let (skills, row) = fresh_skills_and_seed().await;
    from_learned_plan_template(&row, &skills, TrustGrade::LearnedUnreviewed)
        .await
        .expect("ingest at lowest grade");
    let before = skills
        .find_active_by_name(&skill_name_for_template(&row.id))
        .await
        .unwrap()
        .unwrap();
    let outcome = demote_learned_template(&row, &skills)
        .await
        .expect("demote idempotent");
    assert_eq!(
        outcome,
        Some(before.id),
        "already-at-lowest demote returns the existing skill id"
    );
    let after = skills
        .find_active_by_name(&skill_name_for_template(&row.id))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        after.frontmatter.version, before.frontmatter.version,
        "no new version row should be created on a no-op demote"
    );
}
