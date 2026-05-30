//! Unit tests for [`TeamPlan`] validation + topological layering.
use super::*;

fn worker() -> WorkerId {
    WorkerId::new()
}

fn step(id: &str, deps: &[&str]) -> DelegationStep {
    DelegationStep::new(StepId::new(id), worker(), format!("task-{id}"))
        .with_depends_on(deps.iter().map(|d| StepId::new(*d)).collect())
}

#[test]
fn single_layer_when_no_dependencies() {
    let plan = TeamPlan::new(
        worker(),
        "fanout",
        vec![step("a", &[]), step("b", &[]), step("c", &[])],
    );
    let layers = plan.layers().expect("acyclic");
    assert_eq!(layers.len(), 1, "no deps → one parallel layer");
    assert_eq!(layers[0].len(), 3);
}

#[test]
fn a_then_b_c_then_d_three_layers() {
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
    let layers = plan.layers().expect("acyclic");
    assert_eq!(layers.len(), 3);
    assert_eq!(layers[0], vec![StepId::new("a")]);
    // b and c share layer 1 (BTree-ordered).
    assert_eq!(layers[1], vec![StepId::new("b"), StepId::new("c")]);
    assert_eq!(layers[2], vec![StepId::new("d")]);
}

#[test]
fn linear_chain_is_one_step_per_layer() {
    let plan = TeamPlan::new(
        worker(),
        "chain",
        vec![step("a", &[]), step("b", &["a"]), step("c", &["b"])],
    );
    let layers = plan.layers().expect("acyclic");
    assert_eq!(layers.len(), 3);
    assert!(layers.iter().all(|l| l.len() == 1));
}

#[test]
fn cycle_is_rejected() {
    // a → b → a
    let plan = TeamPlan::new(
        worker(),
        "cycle",
        vec![step("a", &["b"]), step("b", &["a"])],
    );
    let err = plan.layers().expect_err("cycle must be rejected");
    assert!(format!("{err}").contains("cycle"));
}

#[test]
fn unknown_dependency_is_rejected() {
    let plan = TeamPlan::new(worker(), "bad", vec![step("a", &["ghost"])]);
    let err = plan.layers().expect_err("unknown dep must be rejected");
    assert!(format!("{err}").contains("unknown step"));
}

#[test]
fn duplicate_step_id_is_rejected() {
    let plan = TeamPlan::new(worker(), "dup", vec![step("a", &[]), step("a", &[])]);
    let err = plan.layers().expect_err("dup id must be rejected");
    assert!(format!("{err}").contains("duplicate"));
}

#[test]
fn empty_plan_yields_no_layers() {
    let plan = TeamPlan::new(worker(), "empty", vec![]);
    let layers = plan.layers().expect("empty is acyclic");
    assert!(layers.is_empty());
}
