//! Unit tests for [`crate::synthesizer`].
//!
//! Three brief-mandated cases:
//!   1. stub synthesizer round-trips raw → charter with identity mission
//!   2. LLM synthesizer with a `MockProvider` returning a scripted
//!      JSON response produces the expected `WorkerCharter`
//!   3. Malformed provider response returns a clear `ArgoError::Provider`
//!      (not a panic)
//!
//! Plus a small set of helpers covering pre-processing, code-fence
//! stripping, the fail-soft wrapper, and field-validation propagation
//! through the LLM path — keeps coverage above 80% without
//! ballooning the file past the 500-LOC budget.

use super::*;
use crate::archetype::ProviderClient;
use crate::charter::WorkerCharter;
use argo_types::{ArgoError, ArgoResult};
use async_trait::async_trait;
use std::sync::Arc;
use std::sync::Mutex;

/// Scriptable provider — pops the next response (or error) on each
/// call. Mirrors `archetype_tests::MockProviderClient`.
struct MockProvider {
    script: Mutex<Vec<MockResponse>>,
}

enum MockResponse {
    Ok(String),
    Err(ArgoError),
}

impl MockProvider {
    fn ok(body: &str) -> Arc<Self> {
        Arc::new(Self {
            script: Mutex::new(vec![MockResponse::Ok(body.to_string())]),
        })
    }

    fn err(e: ArgoError) -> Arc<Self> {
        Arc::new(Self {
            script: Mutex::new(vec![MockResponse::Err(e)]),
        })
    }
}

#[async_trait]
impl ProviderClient for MockProvider {
    async fn classify(&self, _prompt: &str) -> ArgoResult<String> {
        let mut guard = self.script.lock().expect("mock script mutex poisoned");
        match guard.pop() {
            Some(MockResponse::Ok(s)) => Ok(s),
            Some(MockResponse::Err(e)) => Err(e),
            None => panic!("MockProvider: script exhausted"),
        }
    }
}

#[tokio::test]
async fn stub_synthesizer_round_trips_identity_charter() {
    let raw = "Be my recruiter screener. Screen inbound CVs.";
    let synth = StubSynthesizer::new();
    let charter = synth.synthesize(raw).await.expect("stub never errors");
    assert_eq!(charter.mission, raw);
    assert!(charter.responsibilities.is_empty());
    assert!(charter.tools_hint.is_empty());
    assert!(charter.escalation_contact.is_none());
    assert!(charter.vertical_hint.is_none());
    assert_eq!(charter.source_jd_hash, WorkerCharter::sha256_jd(raw));
    // Round-trip: SHA-256 is deterministic for the same input.
    let again = synth.synthesize(raw).await.unwrap();
    assert_eq!(charter, again);
}

#[tokio::test]
async fn llm_synthesizer_parses_scripted_json_into_worker_charter() {
    let raw = "We are hiring a Sales SDR. Outbound prospecting on Slack and email. Report to VP Sales.";
    let scripted = r#"{
        "mission": "Run outbound prospecting and qualify inbound leads.",
        "responsibilities": [
            "Send 30 cold outreach emails per day.",
            "Qualify inbound leads on Slack.",
            "Hand off SQLs to AEs."
        ],
        "tools_hint": ["Slack", "Salesforce", "Apollo"],
        "escalation_contact": "VP Sales",
        "vertical_hint": "sales_sdr"
    }"#;
    let synth = LlmSynthesizer::new(MockProvider::ok(scripted));
    let charter = synth.synthesize(raw).await.expect("scripted JSON parses");

    assert_eq!(
        charter.mission,
        "Run outbound prospecting and qualify inbound leads."
    );
    assert_eq!(charter.responsibilities.len(), 3);
    assert_eq!(charter.responsibilities[0], "Send 30 cold outreach emails per day.");
    assert_eq!(charter.tools_hint, vec!["Slack", "Salesforce", "Apollo"]);
    assert_eq!(charter.escalation_contact.as_deref(), Some("VP Sales"));
    assert_eq!(charter.vertical_hint.as_deref(), Some("sales_sdr"));
    // Hash is always over the original raw text, not the cleaned form.
    assert_eq!(charter.source_jd_hash, WorkerCharter::sha256_jd(raw));
}

#[tokio::test]
async fn llm_synthesizer_malformed_response_returns_provider_error_not_panic() {
    // Not JSON at all — the model went off-script.
    let synth = LlmSynthesizer::new(MockProvider::ok(
        "I'm afraid I can't help with that, but here is a poem...",
    ));
    let err = synth
        .synthesize("Be my SDR.")
        .await
        .expect_err("malformed response must error, not panic");
    match err {
        ArgoError::Provider(msg) => {
            assert!(
                msg.contains("not valid JSON"),
                "expected JSON-parse error message, got: {msg}"
            );
        }
        other => panic!("expected ArgoError::Provider, got: {other:?}"),
    }
}

// ─── extra coverage (small, focused) ──────────────────────────────

#[tokio::test]
async fn llm_synthesizer_tolerates_json_code_fence() {
    let raw = "Be my marketing intern.";
    let fenced = "```json\n{\"mission\":\"Run marketing campaigns.\",\
                  \"responsibilities\":[\"Draft newsletters.\"],\
                  \"tools_hint\":[],\"escalation_contact\":null,\
                  \"vertical_hint\":\"marketing_intern\"}\n```";
    let synth = LlmSynthesizer::new(MockProvider::ok(fenced));
    let charter = synth.synthesize(raw).await.expect("fenced JSON parses");
    assert_eq!(charter.mission, "Run marketing campaigns.");
}

#[tokio::test]
async fn llm_synthesizer_rejects_charter_with_empty_mission() {
    let raw = "Be my ops on-call.";
    let scripted = r#"{
        "mission": "   ",
        "responsibilities": ["page me"],
        "tools_hint": [],
        "escalation_contact": null,
        "vertical_hint": "ops_on_call"
    }"#;
    let synth = LlmSynthesizer::new(MockProvider::ok(scripted));
    let err = synth.synthesize(raw).await.expect_err("empty mission rejects");
    match err {
        ArgoError::Schema(msg) => assert!(msg.contains("mission is empty"), "got: {msg}"),
        other => panic!("expected ArgoError::Schema, got: {other:?}"),
    }
}

#[tokio::test]
async fn llm_synthesizer_propagates_provider_transport_error() {
    let synth = LlmSynthesizer::new(MockProvider::err(ArgoError::Provider(
        "simulated network timeout".into(),
    )));
    let err = synth.synthesize("Be my SDR.").await.expect_err("transport error");
    match err {
        ArgoError::Provider(msg) => assert!(msg.contains("simulated network timeout")),
        other => panic!("expected ArgoError::Provider, got: {other:?}"),
    }
}

#[tokio::test]
async fn synthesize_or_stub_falls_back_when_llm_errors() {
    let synth = LlmSynthesizer::new(MockProvider::err(ArgoError::Provider("nope".into())));
    let raw = "Be my marketing intern.";
    let charter = synthesize_or_stub(&synth, raw).await;
    assert_eq!(charter.mission, raw);
    assert!(charter.responsibilities.is_empty());
}

#[test]
fn preprocess_collapses_runs_of_whitespace_and_blank_lines() {
    let raw = "Role:    Sales   SDR\n\n\n\nDetails:    outbound\n   prospecting.\n";
    let out = preprocess(raw);
    // No double-spaces.
    assert!(!out.contains("  "), "found double-space in: {out:?}");
    // No triple-newline runs.
    assert!(!out.contains("\n\n\n"), "found triple-newline in: {out:?}");
}

#[test]
fn preprocess_truncates_at_byte_cap_on_safe_boundary() {
    let raw = "a".repeat(MAX_PROMPT_INPUT_BYTES * 2);
    let out = preprocess(&raw);
    assert!(out.len() <= MAX_PROMPT_INPUT_BYTES);
}
