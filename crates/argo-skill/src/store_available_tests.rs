//! Unit tests for [`SkillStore::available_for_worker`] — Stream
//! `mission-skill-store-available-for-worker` (M6 precursor 2/3).
#![cfg(test)]

use crate::trust::TrustGrade;
use crate::trust_reader::WorkerTrustReader;
use crate::SkillStore;
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, TenantId};
use async_trait::async_trait;
use std::sync::Arc;
use uuid::Uuid;

/// Test double for [`WorkerTrustReader`]. `Some(grade)` answers
/// every lookup with that grade; `None` answers every lookup with
/// `Ok(None)` — i.e. "no such worker".
struct FakeTrustReader(Option<u8>);

#[async_trait]
impl WorkerTrustReader for FakeTrustReader {
    async fn trust_grade(
        &self,
        _tenant_id: TenantId,
        _worker_id: Uuid,
    ) -> ArgoResult<Option<u8>> {
        Ok(self.0)
    }
}

async fn fresh_store() -> SkillStore {
    let store = Arc::new(SessionStore::open(":memory:").await.unwrap());
    store.migrate().await.unwrap();
    SkillStore::new(store)
}

fn skill_md(name: &str) -> String {
    format!(
        "---\nname: {name}\nversion: \"0.1.0\"\ndescription: \"fixture\"\n\
         provenance:\n  source: builtin\n  hash: sha256-x\n---\nbody",
    )
}

#[tokio::test]
async fn no_restriction_skills_always_returned() {
    // Skills with `min_trust_grade IS NULL` surface for every worker,
    // including the most-restrictive grade=0.
    let skills = fresh_store().await;
    skills.install(&skill_md("open"), None).await.unwrap();
    let reader = FakeTrustReader(Some(0));
    let got = skills
        .available_for_worker(&reader, TenantId::new(), Uuid::now_v7())
        .await
        .unwrap();
    assert_eq!(got.len(), 1);
    assert_eq!(got[0].frontmatter.name, "open");
}

#[tokio::test]
async fn restricted_skill_filtered_when_worker_grade_too_low() {
    // Skill requires Tenant (discriminant 2); worker is at
    // PublishedVerified (discriminant 1) on the same numeric ladder.
    // 1 >= 2 is false, so the skill must NOT appear in the list.
    let skills = fresh_store().await;
    skills.install(&skill_md("restricted"), None).await.unwrap();
    skills
        .set_min_trust_grade("restricted", Some(TrustGrade::Tenant))
        .await
        .unwrap();
    let reader = FakeTrustReader(Some(1));
    let got = skills
        .available_for_worker(&reader, TenantId::new(), Uuid::now_v7())
        .await
        .unwrap();
    assert!(
        got.iter().all(|s| s.frontmatter.name != "restricted"),
        "restricted skill should be filtered, got {:?}",
        got.iter().map(|s| &s.frontmatter.name).collect::<Vec<_>>(),
    );
}

#[tokio::test]
async fn restricted_skill_returned_when_worker_grade_equal() {
    // Skill requires Tenant (=2); worker is also Tenant (=2). 2 >= 2
    // is true; skill must surface.
    let skills = fresh_store().await;
    skills.install(&skill_md("gated"), None).await.unwrap();
    skills
        .set_min_trust_grade("gated", Some(TrustGrade::Tenant))
        .await
        .unwrap();
    let reader = FakeTrustReader(Some(2));
    let got = skills
        .available_for_worker(&reader, TenantId::new(), Uuid::now_v7())
        .await
        .unwrap();
    assert_eq!(got.len(), 1);
    assert_eq!(got[0].frontmatter.name, "gated");
}

#[tokio::test]
async fn restricted_skill_returned_when_worker_grade_higher() {
    // Skill floor at PublishedVerified (=1); worker at
    // PublishedCommunity (=3). 3 >= 1 is true.
    let skills = fresh_store().await;
    skills.install(&skill_md("loose"), None).await.unwrap();
    skills
        .set_min_trust_grade("loose", Some(TrustGrade::PublishedVerified))
        .await
        .unwrap();
    let reader = FakeTrustReader(Some(3));
    let got = skills
        .available_for_worker(&reader, TenantId::new(), Uuid::now_v7())
        .await
        .unwrap();
    assert_eq!(got.len(), 1);
    assert_eq!(got[0].frontmatter.name, "loose");
}

#[tokio::test]
async fn legacy_zero_grade_worker_only_sees_unrestricted_skills() {
    // A "legacy" worker reads as grade=0 (the migration default for
    // `worker.trust_grade INTEGER NOT NULL DEFAULT 0`). It can only
    // surface skills with NULL min_trust_grade or min_trust_grade
    // <= 0; any positive floor filters them out.
    let skills = fresh_store().await;
    skills.install(&skill_md("open"), None).await.unwrap();
    skills.install(&skill_md("gated"), None).await.unwrap();
    skills
        .set_min_trust_grade("gated", Some(TrustGrade::PublishedVerified))
        .await
        .unwrap();
    let reader = FakeTrustReader(Some(0));
    let got = skills
        .available_for_worker(&reader, TenantId::new(), Uuid::now_v7())
        .await
        .unwrap();
    let names: Vec<&str> = got.iter().map(|s| s.frontmatter.name.as_str()).collect();
    assert_eq!(names, vec!["open"]);
}

#[tokio::test]
async fn nonexistent_worker_returns_provider_error() {
    // Reader returns Ok(None) ⇒ no row for the (tenant, worker)
    // pair. The brief asks for an explicit error rather than an
    // empty list so a wrong-tenant typo surfaces loudly.
    let skills = fresh_store().await;
    skills.install(&skill_md("any"), None).await.unwrap();
    let reader = FakeTrustReader(None);
    let err = skills
        .available_for_worker(&reader, TenantId::new(), Uuid::now_v7())
        .await
        .expect_err("missing worker must error");
    match err {
        ArgoError::Provider(msg) => {
            assert!(
                msg.contains("not found"),
                "error should mention not-found, got {msg:?}",
            );
        }
        other => panic!("expected Provider error, got {other:?}"),
    }
}
