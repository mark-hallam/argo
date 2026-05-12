//! Unit tests for the `system:dreams.curation_tick` handler.
use super::dreams_curation_tick::{DreamsCurationHandler, DreamsCurationReport, DreamsCurator};
use argo_cron::{SystemJobContext, SystemJobHandler};
use argo_types::{ArgoError, ArgoResult};
use async_trait::async_trait;
use std::path::PathBuf;
use std::sync::Arc;

struct FixedCurator(DreamsCurationReport);
#[async_trait]
impl DreamsCurator for FixedCurator {
    async fn curate(&self, _window_hours: i64) -> ArgoResult<DreamsCurationReport> {
        Ok(self.0.clone())
    }
}

fn ctx() -> SystemJobContext {
    SystemJobContext::at_now(PathBuf::from("/tmp"))
}

#[tokio::test]
async fn happy_path_full_curation_pass() {
    let h = DreamsCurationHandler::new(Arc::new(FixedCurator(DreamsCurationReport {
        templates_proposed: 12,
        templates_promoted: 7,
        templates_discarded: 5,
        trajectories_scanned: 240,
    })));
    let out = h.run(ctx()).await.unwrap();
    assert_eq!(out.jobs_processed, 7);
    assert_eq!(out.skipped, 5);
    assert!(out.summary.contains("scanned 240"));
    assert!(out.summary.contains("proposed 12"));
}

#[tokio::test]
async fn skip_when_no_trajectories_in_window() {
    let h = DreamsCurationHandler::new(Arc::new(FixedCurator(DreamsCurationReport::default())));
    let out = h.run(ctx()).await.unwrap();
    assert_eq!(out.jobs_processed, 0);
    assert!(out.summary.contains("no trajectories"));
}

#[tokio::test]
async fn failure_mode_propagates_curator_error() {
    struct FailingCurator;
    #[async_trait]
    impl DreamsCurator for FailingCurator {
        async fn curate(&self, _: i64) -> ArgoResult<DreamsCurationReport> {
            Err(ArgoError::Provider("distillation crashed".into()))
        }
    }
    let h = DreamsCurationHandler::new(Arc::new(FailingCurator));
    let err = h.run(ctx()).await.unwrap_err();
    assert!(err.to_string().contains("distillation crashed"));
}

#[tokio::test]
async fn partial_failure_records_discards_as_skipped() {
    // The "partial failure" mode for dreams is: some templates
    // proposed but all discarded. The handler should still report
    // success-style (no errors), but the outcome's `skipped` counter
    // surfaces what was thrown away.
    let h = DreamsCurationHandler::new(Arc::new(FixedCurator(DreamsCurationReport {
        templates_proposed: 5,
        templates_promoted: 0,
        templates_discarded: 5,
        trajectories_scanned: 12,
    })));
    let out = h.run(ctx()).await.unwrap();
    assert_eq!(out.jobs_processed, 0);
    assert_eq!(out.skipped, 5);
    assert!(out.errors.is_empty());
    assert!(out.summary.contains("discarded 5"));
}
