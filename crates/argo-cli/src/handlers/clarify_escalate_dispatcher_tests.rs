//! Tests for the headless `clarify` escalation dispatcher.

use super::*;
use argo_tool::ClarifyDispatcher;
use std::sync::{Arc, Mutex};

fn cell() -> ClarifyCell {
    Arc::new(Mutex::new(None))
}

#[tokio::test]
async fn ask_records_the_question_and_returns_escalated_success() {
    let c = cell();
    let d = ClarifyEscalateDispatcher::new(c.clone());

    let answer = d
        .ask("What is the seller's reserve price?", None)
        .await
        .expect("clarify ask must succeed (escalate), never error");

    // The success result tells the agent it was escalated + to stop — NOT a
    // fabricated answer and NOT the "dispatcher not wired" error.
    assert!(answer.answer.to_lowercase().contains("escalated"));
    assert!(answer.answer.to_lowercase().contains("stop"));
    assert_eq!(answer.option_index, None);
    // The question is captured in the shared cell for the NEED_INFO stop.
    assert_eq!(
        c.lock().unwrap().as_deref(),
        Some("What is the seller's reserve price?")
    );
}

#[tokio::test]
async fn ask_with_options_records_them_alongside_the_question() {
    let c = cell();
    let d = ClarifyEscalateDispatcher::new(c.clone());

    d.ask(
        "Ship by air or sea?",
        Some(&["air".to_string(), "sea".to_string()]),
    )
    .await
    .unwrap();

    let recorded = c.lock().unwrap().clone().unwrap();
    assert!(recorded.contains("Ship by air or sea?"));
    assert!(recorded.contains("air | sea"));
}

#[tokio::test]
async fn only_the_first_question_is_recorded() {
    // A NEED_INFO stop ends the run, so the FIRST question is the one the agent
    // paused on; a later ask in the same turn must not clobber it.
    let c = cell();
    let d = ClarifyEscalateDispatcher::new(c.clone());

    d.ask("first question", None).await.unwrap();
    d.ask("second question", None).await.unwrap();

    assert_eq!(c.lock().unwrap().as_deref(), Some("first question"));
}

#[test]
fn surface_id_is_stable() {
    let d = ClarifyEscalateDispatcher::new(cell());
    assert_eq!(d.surface_id(), "work-serve-escalate");
}
