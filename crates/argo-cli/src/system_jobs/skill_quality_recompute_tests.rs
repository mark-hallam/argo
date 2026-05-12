//! Unit tests for the `system:skill.quality_recompute` handler.
use super::skill_quality_recompute::{
    CatalogSkill, QualityWriter, SkillCatalog, SkillQualityRecomputeHandler,
};
use argo_cron::{SystemJobContext, SystemJobHandler};
use argo_types::{ArgoError, ArgoResult, SkillId};
use async_trait::async_trait;
use std::path::PathBuf;
use std::sync::Arc;

struct FixedCatalog(Vec<CatalogSkill>);
#[async_trait]
impl SkillCatalog for FixedCatalog {
    async fn list_active(&self, limit: usize) -> ArgoResult<Vec<CatalogSkill>> {
        Ok(self.0.iter().take(limit).cloned().collect())
    }
}

struct ConditionalWriter {
    fail_on: Option<String>,
}
#[async_trait]
impl QualityWriter for ConditionalWriter {
    async fn recompute(&self, s: &CatalogSkill) -> ArgoResult<()> {
        if self.fail_on.as_deref() == Some(s.name.as_str()) {
            Err(ArgoError::Provider("synthetic".into()))
        } else {
            Ok(())
        }
    }
}

fn skill(name: &str) -> CatalogSkill {
    CatalogSkill {
        id: SkillId::new(),
        name: name.into(),
    }
}

fn ctx() -> SystemJobContext {
    SystemJobContext::at_now(PathBuf::from("/tmp"))
}

#[tokio::test]
async fn happy_path_all_recomputed() {
    let h = SkillQualityRecomputeHandler::new(
        Arc::new(FixedCatalog(vec![skill("a"), skill("b")])),
        Arc::new(ConditionalWriter { fail_on: None }),
    );
    let out = h.run(ctx()).await.unwrap();
    assert_eq!(out.jobs_processed, 2);
    assert!(out.errors.is_empty());
    assert!(out.summary.contains("2/2"));
}

#[tokio::test]
async fn skip_when_catalog_empty() {
    let h = SkillQualityRecomputeHandler::new(
        Arc::new(FixedCatalog(vec![])),
        Arc::new(ConditionalWriter { fail_on: None }),
    );
    let out = h.run(ctx()).await.unwrap();
    assert_eq!(out.jobs_processed, 0);
    assert!(out.summary.contains("catalog empty"));
}

#[tokio::test]
async fn failure_mode_propagates_catalog_error() {
    struct FailingCatalog;
    #[async_trait]
    impl SkillCatalog for FailingCatalog {
        async fn list_active(&self, _: usize) -> ArgoResult<Vec<CatalogSkill>> {
            Err(ArgoError::Provider("catalog down".into()))
        }
    }
    let h = SkillQualityRecomputeHandler::new(
        Arc::new(FailingCatalog),
        Arc::new(ConditionalWriter { fail_on: None }),
    );
    let err = h.run(ctx()).await.unwrap_err();
    assert!(err.to_string().contains("catalog down"));
}

#[tokio::test]
async fn partial_failure_reports_per_skill_error() {
    let h = SkillQualityRecomputeHandler::new(
        Arc::new(FixedCatalog(vec![skill("a"), skill("flake"), skill("c")])),
        Arc::new(ConditionalWriter {
            fail_on: Some("flake".into()),
        }),
    );
    let out = h.run(ctx()).await.unwrap();
    assert_eq!(out.jobs_processed, 2);
    assert_eq!(out.errors.len(), 1);
    assert!(out.errors[0].contains("flake"));
    assert!(out.summary.contains("2/3"));
}
