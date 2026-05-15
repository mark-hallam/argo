#![cfg(test)]

use super::*;
use crate::SkillStore;
use argo_store::skill_provenance::{record_provenance, NewSkillProvenance, SkillProvenanceKind};
use argo_store::SessionStore;
use std::sync::Arc;

fn raw_skill(name: &str, version: &str, source: &str) -> String {
    format!(
        "---\nname: {name}\nversion: \"{version}\"\ndescription: \"test\"\nprovenance:\n  source: {source}\n  hash: sha256-x\n---\nbody"
    )
}

async fn setup() -> (Arc<SessionStore>, SkillStore) {
    let store = Arc::new(SessionStore::open(":memory:").await.unwrap());
    store.migrate().await.unwrap();
    let skills = SkillStore::new(Arc::clone(&store));
    (store, skills)
}

#[tokio::test]
async fn missing_skill_returns_typed_error() {
    let (_store, skills) = setup().await;
    let err = explain_skill(&skills, "missing", 20).await.unwrap_err();
    assert!(matches!(err, ArgoError::NotFound(_)));
}

#[tokio::test]
async fn single_version_backfill_explains_bootstrap() {
    let (_store, skills) = setup().await;
    skills
        .install(&raw_skill("alpha", "0.1", "builtin"), None)
        .await
        .unwrap();

    let explanation = explain_skill(&skills, "alpha", 20).await.unwrap();

    assert_eq!(explanation.name, "alpha");
    assert_eq!(explanation.current_version, "0.1");
    assert_eq!(explanation.current_trust_grade, TrustGrade::Builtin);
    assert_eq!(explanation.version_history.len(), 1);
}

#[tokio::test]
async fn history_depth_limits_versions_forge_and_trust() {
    let (store, skills) = setup().await;
    skills
        .install(&raw_skill("alpha", "0.1", "user"), None)
        .await
        .unwrap();
    for version in ["0.2", "0.3", "0.4"] {
        skills
            .supersede(
                &raw_skill("alpha", version, "user"),
                None,
                crate::PrincipalOverride::Force,
            )
            .await
            .unwrap();
    }
    for version in ["0.1", "0.2", "0.3", "0.4"] {
        record_provenance(
            store.pool(),
            NewSkillProvenance {
                skill_name: "alpha",
                skill_version: version,
                kind: SkillProvenanceKind::OperatorAuthored,
                actor: Some("operator"),
                parent_id: None,
                change_reason: Some("edit"),
                breaking_change: false,
            },
        )
        .await
        .unwrap();
    }

    let explanation = explain_skill(&skills, "alpha", 2).await.unwrap();

    assert_eq!(explanation.version_history.len(), 2);
    assert!(matches!(
        explanation.provenance,
        SkillProvenance::OperatorAuthored { .. }
    ));
}

#[tokio::test]
async fn dreams_lineage_comes_from_current_provenance() {
    let (store, skills) = setup().await;
    skills
        .install(&raw_skill("dreamed", "1.0", "learned:cohort"), None)
        .await
        .unwrap();
    record_provenance(
        store.pool(),
        NewSkillProvenance {
            skill_name: "dreamed",
            skill_version: "1.0",
            kind: SkillProvenanceKind::DreamsDerived,
            actor: None,
            parent_id: Some("template-7"),
            change_reason: Some("dreams cohort"),
            breaking_change: false,
        },
    )
    .await
    .unwrap();

    let explanation = explain_skill(&skills, "dreamed", 20).await.unwrap();

    assert_eq!(
        explanation
            .dreams_lineage
            .and_then(|lineage| lineage.parent_template_id),
        Some("template-7".into())
    );
}
