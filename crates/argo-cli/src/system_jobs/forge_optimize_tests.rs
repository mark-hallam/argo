//! Unit tests for the `system:forge.optimize` handler. Covers the
//! brief's required 4-test matrix: happy path / failure / skip /
//! partial-failure.
use super::forge_optimize::{
    ForgeOptimizeHandler, ForgeOptimizeInvoker, StaleSkill, StalestSkillSelector,
};
use argo_cron::{SystemJobContext, SystemJobHandler};
use argo_types::{ArgoError, ArgoResult};
use async_trait::async_trait;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

struct FixedSelector(Vec<StaleSkill>);
#[async_trait]
impl StalestSkillSelector for FixedSelector {
    async fn select(&self, limit: usize) -> ArgoResult<Vec<StaleSkill>> {
        Ok(self.0.iter().take(limit).cloned().collect())
    }
}

struct CountingInvoker {
    count: AtomicUsize,
    fail_on: Option<String>,
}
#[async_trait]
impl ForgeOptimizeInvoker for CountingInvoker {
    async fn optimize_one(&self, s: &StaleSkill) -> ArgoResult<()> {
        self.count.fetch_add(1, Ordering::SeqCst);
        if self.fail_on.as_deref() == Some(s.name.as_str()) {
            return Err(ArgoError::Provider("synthetic".into()));
        }
        Ok(())
    }
}

fn skill(name: &str, grade: f64) -> StaleSkill {
    StaleSkill {
        name: name.into(),
        version: "v1".into(),
        trust_grade: grade,
        last_forge_at: None,
    }
}

fn ctx() -> SystemJobContext {
    SystemJobContext::at_now(PathBuf::from("/tmp"))
}

#[tokio::test]
async fn happy_path_processes_all_selected() {
    let h = ForgeOptimizeHandler::new(
        Arc::new(FixedSelector(vec![skill("a", 0.1), skill("b", 0.2)])),
        Arc::new(CountingInvoker {
            count: AtomicUsize::new(0),
            fail_on: None,
        }),
    );
    let out = h.run(ctx()).await.unwrap();
    assert_eq!(out.jobs_processed, 2);
    assert!(out.errors.is_empty());
    assert!(out.summary.contains("2/2"));
}

#[tokio::test]
async fn skip_when_no_work_returns_empty_summary() {
    let h = ForgeOptimizeHandler::new(
        Arc::new(FixedSelector(vec![])),
        Arc::new(CountingInvoker {
            count: AtomicUsize::new(0),
            fail_on: None,
        }),
    );
    let out = h.run(ctx()).await.unwrap();
    assert_eq!(out.jobs_processed, 0);
    assert!(out.errors.is_empty());
    assert!(out.summary.contains("no stale skills"));
}

#[tokio::test]
async fn failure_mode_propagates_selector_error() {
    struct FailingSelector;
    #[async_trait]
    impl StalestSkillSelector for FailingSelector {
        async fn select(&self, _: usize) -> ArgoResult<Vec<StaleSkill>> {
            Err(ArgoError::Provider("selector exploded".into()))
        }
    }
    let h = ForgeOptimizeHandler::new(
        Arc::new(FailingSelector),
        Arc::new(CountingInvoker {
            count: AtomicUsize::new(0),
            fail_on: None,
        }),
    );
    let err = h.run(ctx()).await.unwrap_err();
    assert!(err.to_string().contains("selector exploded"));
}

#[tokio::test]
async fn partial_failure_reports_per_skill_error_and_still_processes_others() {
    let h = ForgeOptimizeHandler::new(
        Arc::new(FixedSelector(vec![
            skill("alpha", 0.1),
            skill("beta", 0.2),
            skill("gamma", 0.3),
        ])),
        Arc::new(CountingInvoker {
            count: AtomicUsize::new(0),
            fail_on: Some("beta".into()),
        }),
    );
    let out = h.run(ctx()).await.unwrap();
    assert_eq!(out.jobs_processed, 2);
    assert_eq!(out.errors.len(), 1);
    assert!(out.errors[0].contains("beta"));
    assert!(out.summary.contains("2/3"));
}
