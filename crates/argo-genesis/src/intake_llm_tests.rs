//! Tests for the LLM enrichment layer + its degrade-fail-safe contract.

use super::*;
use crate::intake_machine::{IntakeSlot, IntakeState};
use argo_types::{ArgoError, ArgoResult};
use async_trait::async_trait;

/// A mock that returns a fixed string, or errors when `fail` is set.
struct MockClient {
    reply: String,
    fail: bool,
}

#[async_trait]
impl IntakeClient for MockClient {
    async fn complete(&self, _prompt: &str) -> ArgoResult<String> {
        if self.fail {
            Err(ArgoError::Provider("transport down".to_string()))
        } else {
            Ok(self.reply.clone())
        }
    }
}

fn ok(reply: &str) -> MockClient {
    MockClient {
        reply: reply.to_string(),
        fail: false,
    }
}

fn down() -> MockClient {
    MockClient {
        reply: String::new(),
        fail: true,
    }
}

#[tokio::test]
async fn propose_question_returns_cleaned_single_line() {
    let client = ok("\"What should we call your new teammate?\"\nextra chatter");
    let q = propose_question(&IntakeState::default(), IntakeSlot::Persona, &client)
        .await
        .expect("a question");
    assert_eq!(q, "What should we call your new teammate?");
}

#[tokio::test]
async fn propose_question_degrades_to_none_on_transport_error() {
    let q = propose_question(&IntakeState::default(), IntakeSlot::Charter, &down()).await;
    assert!(q.is_none(), "transport failure must yield None → wizard fallback");
}

#[tokio::test]
async fn propose_question_degrades_to_none_on_empty_reply() {
    let q = propose_question(&IntakeState::default(), IntakeSlot::Charter, &ok("   \n  ")).await;
    assert!(q.is_none());
}

#[tokio::test]
async fn extract_delta_parses_embedded_json() {
    let client = ok(
        "sure: {\"cadence\":\"every weekday at 9am\",\"responsibilities\":[\"run the morning report\"]} done",
    );
    let delta = extract_delta(IntakeSlot::Cadence, "every weekday at 9", &client)
        .await
        .expect("a delta");
    assert_eq!(delta.cadence.as_deref(), Some("every weekday at 9am"));
    assert_eq!(delta.responsibilities.len(), 1);
}

#[tokio::test]
async fn extract_delta_parses_typed_success_criteria() {
    let client = ok(
        "{\"success_criteria\":[{\"name\":\"first response time\",\"kind\":\"duration_ms\",\"target\":300000,\"comparator\":\"lte\"}]}",
    );
    let delta = extract_delta(IntakeSlot::SuccessCriteria, "respond within 5 min", &client)
        .await
        .expect("a delta");
    assert_eq!(delta.success_criteria.len(), 1);
    assert_eq!(delta.success_criteria[0].name, "first response time");
}

#[tokio::test]
async fn extract_delta_degrades_to_none_on_transport_error() {
    let delta = extract_delta(IntakeSlot::Cadence, "weekdays", &down()).await;
    assert!(delta.is_none(), "transport failure must yield None → wizard fallback");
}

#[tokio::test]
async fn extract_delta_degrades_to_none_on_garbage_or_empty() {
    assert!(extract_delta(IntakeSlot::Cadence, "x", &ok("no json here")).await.is_none());
    assert!(extract_delta(IntakeSlot::Cadence, "x", &ok("{}")).await.is_none());
}

#[test]
fn prompts_mention_the_slot_topic() {
    let qp = build_question_prompt(&IntakeState::default(), IntakeSlot::Exceptions);
    assert!(qp.contains("escalate"));
    let ep = build_extract_prompt(IntakeSlot::SuccessCriteria, "csat 90");
    assert!(ep.contains("success_criteria"));
    assert!(ep.contains("csat 90"));
}

#[tokio::test]
async fn extract_delta_parses_access_and_escalation_keys() {
    let client = ok(
        "{\"access_integrations\":[\"Xero\",\"Gmail\"],\"escalation_failures_threshold\":2,\"escalation_to\":\"my ops lead\"}",
    );
    let delta = extract_delta(IntakeSlot::Escalation, "after 2 tries, to my ops lead", &client)
        .await
        .expect("a delta");
    assert_eq!(delta.access_integrations, vec!["Xero", "Gmail"]);
    assert_eq!(delta.escalation_failures_threshold, Some(2));
    assert_eq!(delta.escalation_to.as_deref(), Some("my ops lead"));
}

#[test]
fn extract_prompt_advertises_dec048_keys() {
    let prompt = build_extract_prompt(IntakeSlot::Access, "Xero and Gmail");
    assert!(prompt.contains("access_integrations"));
    assert!(prompt.contains("escalation_failures_threshold"));
    assert!(prompt.contains("escalation_to"));
}
