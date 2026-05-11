//! Unit tests for [`crate::turn_planner::maybe_decompose`].
//!
//! Run against in-memory stubs (no real provider, no SQLite) so
//! the four failure-mode invariants from
//! mission-planner-dispatch-loop-wire stay fast and independent of
//! upstream flakes.

use super::turn_planner::{maybe_decompose, DecomposeOutcome};
use argo_runtime::{
    DecomposeGate, DecomposeResult, GateError, PlannedSubtask, Planner, PlannerContext,
    PlannerError, SubtaskSink,
};
use argo_workflow::{TaskNode, Workflow};
use async_trait::async_trait;
use std::collections::BTreeMap;
use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    Arc, Mutex,
};

#[derive(Default)]
struct VecSink {
    rows: Mutex<Vec<PlannedSubtask>>,
    fail_immediately: bool,
}

#[async_trait]
impl SubtaskSink for VecSink {
    async fn record_subtask(
        &self,
        _parent: &str,
        _sid: Option<&str>,
        s: &PlannedSubtask,
    ) -> Result<(), PlannerError> {
        if self.fail_immediately {
            return Err(PlannerError::Llm("sink: synthetic".into()));
        }
        self.rows.lock().expect("poisoned").push(s.clone());
        Ok(())
    }
}

struct CountingGate {
    answer: bool,
    calls: AtomicUsize,
}

impl CountingGate {
    fn ok(answer: bool) -> Self {
        Self { answer, calls: AtomicUsize::new(0) }
    }
}

#[async_trait]
impl DecomposeGate for CountingGate {
    async fn should_decompose(
        &self,
        _: &str,
        _: &PlannerContext,
    ) -> Result<bool, GateError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(self.answer)
    }
}

struct StaticPlanner {
    workflow: Workflow,
    called: AtomicBool,
    fail: bool,
}

impl StaticPlanner {
    fn ok_with(tasks: usize) -> Self {
        let mut workflow = Workflow {
            id: "wf-stub".into(),
            description: "stub workflow".into(),
            tasks: Vec::new(),
            edges: Vec::new(),
        };
        for i in 0..tasks {
            workflow.tasks.push(TaskNode {
                id: format!("t{i}"),
                worker: String::new(),
                persona: None,
                inputs: BTreeMap::new(),
                outputs: BTreeMap::new(),
                depends_on: Vec::new(),
                cache_key: None,
            });
        }
        Self { workflow, called: AtomicBool::new(false), fail: false }
    }

    fn failing() -> Self {
        let mut s = Self::ok_with(0);
        s.fail = true;
        s
    }
}

#[async_trait]
impl Planner for StaticPlanner {
    async fn decompose(
        &self,
        _: &str,
        _: &PlannerContext,
    ) -> Result<DecomposeResult, PlannerError> {
        self.called.store(true, Ordering::SeqCst);
        if self.fail {
            return Err(PlannerError::Llm("planner: synthetic".into()));
        }
        Ok(DecomposeResult {
            workflow: self.workflow.clone(),
            reasoning: "stub".into(),
            planner_kind: "stub".into(),
            template_name: None,
        })
    }
}

fn ctx() -> PlannerContext {
    PlannerContext::for_worker("wkr-test")
}

#[tokio::test]
async fn dispatch_without_gate_acts_single_task() {
    let planner = Arc::new(StaticPlanner::ok_with(3));
    let gate = Arc::new(CountingGate::ok(false));
    let sink = Arc::new(VecSink::default());
    let outcome = maybe_decompose(
        planner.as_ref(), gate.as_ref(), sink.as_ref(),
        Some("session-1"), "trivial goal", &ctx(),
    ).await;
    assert_eq!(outcome, DecomposeOutcome::SkippedByGate);
    assert_eq!(gate.calls.load(Ordering::SeqCst), 1);
    assert!(!planner.called.load(Ordering::SeqCst));
    assert!(sink.rows.lock().unwrap().is_empty());
}

#[tokio::test]
async fn gate_true_decomposes_and_records() {
    let planner = Arc::new(StaticPlanner::ok_with(3));
    let gate = Arc::new(CountingGate::ok(true));
    let sink = Arc::new(VecSink::default());
    let outcome = maybe_decompose(
        planner.as_ref(), gate.as_ref(), sink.as_ref(),
        Some("session-2"), "do A; do B; do C", &ctx(),
    ).await;
    assert_eq!(outcome, DecomposeOutcome::Decomposed { children: 3 });
    let rows = sink.rows.lock().unwrap();
    assert_eq!(rows.len(), 3);
    assert_eq!(rows[0].worker_id, "wkr-test", "ctx worker_id flows through apply_context");
    assert_eq!(rows[0].plan_node_path, "/wf-stub/t0");
}

#[tokio::test]
async fn planner_error_falls_back_to_single() {
    let planner = Arc::new(StaticPlanner::failing());
    let gate = Arc::new(CountingGate::ok(true));
    let sink = Arc::new(VecSink::default());
    let outcome = maybe_decompose(
        planner.as_ref(), gate.as_ref(), sink.as_ref(),
        Some("session-3"), "do A and then B", &ctx(),
    ).await;
    assert_eq!(outcome, DecomposeOutcome::PlannerError);
    assert!(planner.called.load(Ordering::SeqCst));
    assert!(sink.rows.lock().unwrap().is_empty());
}

#[tokio::test]
async fn record_error_logs_and_continues() {
    let planner = Arc::new(StaticPlanner::ok_with(2));
    let gate = Arc::new(CountingGate::ok(true));
    let sink = Arc::new(VecSink { rows: Mutex::new(Vec::new()), fail_immediately: true });
    let outcome = maybe_decompose(
        planner.as_ref(), gate.as_ref(), sink.as_ref(),
        None, "long\n- bullet\n- list", &ctx(),
    ).await;
    assert_eq!(outcome, DecomposeOutcome::SinkError { children: 2 });
    assert!(planner.called.load(Ordering::SeqCst));
    assert!(sink.rows.lock().unwrap().is_empty());
}
