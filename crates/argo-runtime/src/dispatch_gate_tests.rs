//! Unit tests for [`crate::dispatch_gate::HeuristicGate`].

use super::dispatch_gate::*;
use crate::PlannerContext;

fn ctx() -> PlannerContext {
    PlannerContext::for_worker("wkr-test")
}

#[tokio::test]
async fn short_simple_goal_does_not_decompose() {
    let gate = HeuristicGate::new();
    let decide = gate
        .should_decompose("write a haiku about ferris", &ctx())
        .await
        .unwrap();
    assert!(
        !decide,
        "single-sentence prose under the length threshold should stay as a single task",
    );
}

#[tokio::test]
async fn goal_with_and_then_decomposes() {
    let gate = HeuristicGate::new();
    let decide = gate
        .should_decompose(
            "draft the launch tweet and then schedule it for noon",
            &ctx(),
        )
        .await
        .unwrap();
    assert!(decide, "explicit ' and then ' marker should trigger decomposition");
}

#[tokio::test]
async fn goal_with_bullets_decomposes() {
    let gate = HeuristicGate::new();
    let goal = "Promote the launch:\n- write tweet\n- write blog\n- schedule both";
    let decide = gate.should_decompose(goal, &ctx()).await.unwrap();
    assert!(decide, "newline-bullet markers should trigger decomposition");
}

#[tokio::test]
async fn long_prose_decomposes() {
    let gate = HeuristicGate::new();
    let mut goal = String::new();
    while goal.len() <= HEURISTIC_LENGTH_THRESHOLD {
        goal.push_str("explore the competitive landscape carefully ");
    }
    let decide = gate.should_decompose(&goal, &ctx()).await.unwrap();
    assert!(
        decide,
        "goal exceeding {} chars should trigger decomposition by length alone",
        HEURISTIC_LENGTH_THRESHOLD,
    );
}

#[tokio::test]
async fn empty_goal_does_not_decompose() {
    let gate = HeuristicGate::new();
    let empty = gate.should_decompose("", &ctx()).await.unwrap();
    let whitespace = gate
        .should_decompose("   \n\t  ", &ctx())
        .await
        .unwrap();
    assert!(!empty, "empty goal must short-circuit to no-decompose");
    assert!(
        !whitespace,
        "whitespace-only goal must short-circuit to no-decompose",
    );
}

#[tokio::test]
async fn semicolon_list_decomposes() {
    let gate = HeuristicGate::new();
    let decide = gate
        .should_decompose("research; draft; review; ship", &ctx())
        .await
        .unwrap();
    assert!(decide, "semicolon-separated list should trigger decomposition");
}
