//! Engine tests for [`Supervisor::run_team`] — parallel fan-out,
//! dependency-layer ordering, failure-drops-dependents, and the
//! bounded-concurrency cap. Executors are deterministic in-test
//! doubles so no store / network is needed; the persisting executor
//! is covered separately in `team_persist_tests.rs`.
use super::*;
use crate::team_plan::DelegationStep;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::Duration;

fn worker() -> WorkerId {
    WorkerId::new()
}

fn step(id: &str, deps: &[&str]) -> DelegationStep {
    DelegationStep::new(StepId::new(id), worker(), format!("task-{id}"))
        .with_depends_on(deps.iter().map(|d| StepId::new(*d)).collect())
}

/// Records start/end ordering + tracks peak concurrency. Every step
/// succeeds. A short sleep widens the concurrency window so parallel
/// steps actually overlap.
#[derive(Default)]
struct RecordingExecutor {
    in_flight: AtomicUsize,
    peak: AtomicUsize,
    // (step_id, "start"|"end") in observed order.
    events: Mutex<Vec<(String, &'static str)>>,
}

#[async_trait]
impl StepExecutor for RecordingExecutor {
    async fn execute(&self, ctx: &StepContext) -> ArgoResult<(String, String)> {
        let now = self.in_flight.fetch_add(1, Ordering::SeqCst) + 1;
        self.peak.fetch_max(now, Ordering::SeqCst);
        self.events
            .lock()
            .expect("lock")
            .push((ctx.step_id.to_string(), "start"));
        tokio::time::sleep(Duration::from_millis(40)).await;
        self.events
            .lock()
            .expect("lock")
            .push((ctx.step_id.to_string(), "end"));
        self.in_flight.fetch_sub(1, Ordering::SeqCst);
        Ok((
            format!("run-{}", ctx.step_id),
            format!("did {}", ctx.task),
        ))
    }
}

/// Fails exactly one named step; everything else succeeds.
struct FailOne {
    fail_id: &'static str,
}

#[async_trait]
impl StepExecutor for FailOne {
    async fn execute(&self, ctx: &StepContext) -> ArgoResult<(String, String)> {
        if ctx.step_id.as_str() == self.fail_id {
            return Err(argo_types::ArgoError::Provider(format!(
                "step {} blew up",
                self.fail_id
            )));
        }
        Ok((format!("run-{}", ctx.step_id), "ok".to_string()))
    }
}

#[tokio::test]
async fn three_parallel_children_all_complete() {
    let exec = Arc::new(RecordingExecutor::default());
    let sup = Supervisor::new(Arc::clone(&exec));
    let plan = TeamPlan::new(
        worker(),
        "fanout-3",
        vec![step("a", &[]), step("b", &[]), step("c", &[])],
    );
    let result = sup
        .run_team(&plan, TenantId::new(), Some("handoff-xyz".into()))
        .await
        .expect("run");
    assert!(result.all_completed());
    assert_eq!(result.completed_count(), 3);
    assert_eq!(result.outcomes.len(), 3);
    assert!(result.outcomes.iter().all(|o| o.run_id.is_some()));
    // All three ran in the same (only) layer → peak concurrency 3.
    assert_eq!(exec.peak.load(Ordering::SeqCst), 3);
    assert!(!result.synthesis_run_id.is_empty());
}

#[tokio::test]
async fn outcomes_preserve_plan_order() {
    let exec = Arc::new(RecordingExecutor::default());
    let sup = Supervisor::new(exec);
    let plan = TeamPlan::new(
        worker(),
        "ordered",
        vec![step("z", &[]), step("m", &[]), step("a", &[])],
    );
    let result = sup.run_team(&plan, TenantId::new(), None).await.expect("run");
    let ids: Vec<&str> = result.outcomes.iter().map(|o| o.step_id.as_str()).collect();
    assert_eq!(ids, vec!["z", "m", "a"], "wire order == plan order");
}

#[tokio::test]
async fn dependency_layers_order_a_before_bc_before_d() {
    let exec = Arc::new(RecordingExecutor::default());
    let sup = Supervisor::new(Arc::clone(&exec));
    // a → {b, c} → d
    let plan = TeamPlan::new(
        worker(),
        "diamond",
        vec![
            step("a", &[]),
            step("b", &["a"]),
            step("c", &["a"]),
            step("d", &["b", "c"]),
        ],
    );
    let result = sup.run_team(&plan, TenantId::new(), None).await.expect("run");
    assert!(result.all_completed());

    let events = exec.events.lock().expect("lock").clone();
    let end_idx = |id: &str| {
        events
            .iter()
            .position(|(s, k)| s == id && *k == "end")
            .expect("end event present")
    };
    let start_idx = |id: &str| {
        events
            .iter()
            .position(|(s, k)| s == id && *k == "start")
            .expect("start event present")
    };
    // a finishes before b and c start (barrier between layer 0 and 1).
    assert!(end_idx("a") < start_idx("b"));
    assert!(end_idx("a") < start_idx("c"));
    // both b and c finish before d starts (barrier into layer 2).
    assert!(end_idx("b") < start_idx("d"));
    assert!(end_idx("c") < start_idx("d"));
    // b and c overlapped → engine ran the middle layer in parallel.
    assert_eq!(exec.peak.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn failed_step_skips_its_dependents_no_hang() {
    let exec = Arc::new(FailOne { fail_id: "b" });
    let sup = Supervisor::new(exec);
    // a → b → d ; c is independent and should still complete.
    let plan = TeamPlan::new(
        worker(),
        "fail-mid",
        vec![
            step("a", &[]),
            step("b", &["a"]),
            step("d", &["b"]),
            step("c", &[]),
        ],
    );
    let result = sup.run_team(&plan, TenantId::new(), None).await.expect("run");

    let by_id = |id: &str| {
        result
            .outcomes
            .iter()
            .find(|o| o.step_id.as_str() == id)
            .expect("outcome present")
            .status
    };
    assert_eq!(by_id("a"), StepStatus::Completed);
    assert_eq!(by_id("b"), StepStatus::Failed);
    assert_eq!(by_id("d"), StepStatus::Skipped, "dependent of failed step");
    assert_eq!(by_id("c"), StepStatus::Completed, "independent still runs");
    assert!(!result.all_completed());
    assert_eq!(result.completed_count(), 2);
}

#[tokio::test]
async fn transitive_skip_propagates_down_the_chain() {
    let exec = Arc::new(FailOne { fail_id: "a" });
    let sup = Supervisor::new(exec);
    // a → b → c ; a fails → b skipped → c skipped (no deadlock).
    let plan = TeamPlan::new(
        worker(),
        "transitive",
        vec![step("a", &[]), step("b", &["a"]), step("c", &["b"])],
    );
    let result = sup.run_team(&plan, TenantId::new(), None).await.expect("run");
    let by_id = |id: &str| {
        result
            .outcomes
            .iter()
            .find(|o| o.step_id.as_str() == id)
            .expect("outcome")
            .status
    };
    assert_eq!(by_id("a"), StepStatus::Failed);
    assert_eq!(by_id("b"), StepStatus::Skipped);
    assert_eq!(by_id("c"), StepStatus::Skipped);
}

#[tokio::test]
async fn concurrency_cap_is_respected() {
    let exec = Arc::new(RecordingExecutor::default());
    // Cap of 2 over a 5-wide single layer → peak must never exceed 2.
    let sup = Supervisor::new(Arc::clone(&exec)).with_concurrency(2);
    let plan = TeamPlan::new(
        worker(),
        "bounded",
        vec![
            step("a", &[]),
            step("b", &[]),
            step("c", &[]),
            step("d", &[]),
            step("e", &[]),
        ],
    );
    let result = sup.run_team(&plan, TenantId::new(), None).await.expect("run");
    assert_eq!(result.completed_count(), 5);
    assert!(
        exec.peak.load(Ordering::SeqCst) <= 2,
        "peak {} exceeded cap 2",
        exec.peak.load(Ordering::SeqCst)
    );
}

#[tokio::test]
async fn cycle_plan_errors_not_hangs() {
    let exec = Arc::new(RecordingExecutor::default());
    let sup = Supervisor::new(exec);
    let plan = TeamPlan::new(
        worker(),
        "cycle",
        vec![step("a", &["b"]), step("b", &["a"])],
    );
    let err = sup
        .run_team(&plan, TenantId::new(), None)
        .await
        .expect_err("cycle must error");
    assert!(format!("{err}").contains("cycle"));
}

#[test]
fn layers_cover_all_guard_holds() {
    let plan = TeamPlan::new(
        worker(),
        "cover",
        vec![step("a", &[]), step("b", &["a"])],
    );
    let layers = plan.layers().expect("acyclic");
    assert!(layers_cover_all(&plan, &layers));
}
