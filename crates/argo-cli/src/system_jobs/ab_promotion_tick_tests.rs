//! Unit tests for the `system:ab.promotion` handler.
use super::ab_promotion_tick::{
    AbDecider, AbDecision, AbExperiment, AbExperimentStore, AbPromotionHandler,
};
use argo_cron::{SystemJobContext, SystemJobHandler};
use argo_types::{ArgoError, ArgoResult};
use async_trait::async_trait;
use std::path::PathBuf;
use std::sync::Arc;

struct FixedStore {
    experiments: Vec<AbExperiment>,
    fail_apply_on: Option<String>,
}
#[async_trait]
impl AbExperimentStore for FixedStore {
    async fn list(&self, limit: usize) -> ArgoResult<Vec<AbExperiment>> {
        Ok(self.experiments.iter().take(limit).cloned().collect())
    }
    async fn apply(&self, exp: &AbExperiment, _d: &AbDecision) -> ArgoResult<()> {
        if self.fail_apply_on.as_deref() == Some(exp.id.as_str()) {
            Err(ArgoError::Provider("apply boom".into()))
        } else {
            Ok(())
        }
    }
}

struct ExtendingDecider;
#[async_trait]
impl AbDecider for ExtendingDecider {
    async fn decide(&self, _exp: &AbExperiment) -> ArgoResult<AbDecision> {
        Ok(AbDecision::Extend)
    }
}

fn exp(id: &str, trials: usize, min: usize) -> AbExperiment {
    AbExperiment {
        id: id.into(),
        skill_name: format!("skill-{id}"),
        trials,
        min_sample: min,
    }
}

fn ctx() -> SystemJobContext {
    SystemJobContext::at_now(PathBuf::from("/tmp"))
}

#[tokio::test]
async fn happy_path_all_eligible_decided() {
    let h = AbPromotionHandler::new(
        Arc::new(FixedStore {
            experiments: vec![exp("a", 200, 100), exp("b", 150, 100)],
            fail_apply_on: None,
        }),
        Arc::new(ExtendingDecider),
    );
    let out = h.run(ctx()).await.unwrap();
    assert_eq!(out.jobs_processed, 2);
    assert_eq!(out.skipped, 0);
    assert!(out.errors.is_empty());
    assert!(out.summary.contains("2 decided"));
}

#[tokio::test]
async fn skip_when_no_experiments_registered() {
    let h = AbPromotionHandler::new(
        Arc::new(FixedStore {
            experiments: vec![],
            fail_apply_on: None,
        }),
        Arc::new(ExtendingDecider),
    );
    let out = h.run(ctx()).await.unwrap();
    assert_eq!(out.jobs_processed, 0);
    assert!(out.summary.contains("no experiments"));
}

#[tokio::test]
async fn failure_mode_propagates_store_list_error() {
    struct FailingStore;
    #[async_trait]
    impl AbExperimentStore for FailingStore {
        async fn list(&self, _: usize) -> ArgoResult<Vec<AbExperiment>> {
            Err(ArgoError::Provider("store down".into()))
        }
        async fn apply(&self, _: &AbExperiment, _: &AbDecision) -> ArgoResult<()> {
            Ok(())
        }
    }
    let h = AbPromotionHandler::new(Arc::new(FailingStore), Arc::new(ExtendingDecider));
    let err = h.run(ctx()).await.unwrap_err();
    assert!(err.to_string().contains("store down"));
}

struct VariantDecider {
    decision: AbDecision,
}
#[async_trait]
impl AbDecider for VariantDecider {
    async fn decide(&self, _exp: &AbExperiment) -> ArgoResult<AbDecision> {
        Ok(self.decision.clone())
    }
}

#[tokio::test]
async fn decision_labels_cover_promote_extend_terminate_variants() {
    let promote = AbDecision::Promote {
        variant: "alpha".into(),
    };
    let terminate = AbDecision::Terminate {
        reason: "regression".into(),
    };
    assert_eq!(promote.label(), "promote(alpha)");
    assert_eq!(AbDecision::Extend.label(), "extend");
    assert_eq!(terminate.label(), "terminate(regression)");

    // Drive the handler through both Promote and Terminate so the
    // variant data is consumed end-to-end (and the dead-code
    // analysis sees both arms exercised).
    for decision in [promote, terminate] {
        let h = AbPromotionHandler::new(
            Arc::new(FixedStore {
                experiments: vec![exp("a", 200, 100)],
                fail_apply_on: None,
            }),
            Arc::new(VariantDecider { decision }),
        );
        let out = h.run(ctx()).await.unwrap();
        assert_eq!(out.jobs_processed, 1);
    }
}

#[tokio::test]
async fn partial_failure_reports_below_threshold_as_skipped_plus_apply_error() {
    let h = AbPromotionHandler::new(
        Arc::new(FixedStore {
            experiments: vec![
                exp("ready", 200, 100),
                exp("not-yet", 50, 100),
                exp("flake", 150, 100),
            ],
            fail_apply_on: Some("flake".into()),
        }),
        Arc::new(ExtendingDecider),
    );
    let out = h.run(ctx()).await.unwrap();
    assert_eq!(out.jobs_processed, 1);
    assert_eq!(out.skipped, 1);
    assert_eq!(out.errors.len(), 1);
    assert!(out.errors[0].contains("flake"));
    assert!(out.summary.contains("1 decided"));
    assert!(out.summary.contains("1 below min-sample"));
}
