//! Inline unit tests for the `/api/v1/pitch/ask` dispatcher.
//!
//! Integration coverage (end-to-end gateway smoke) lives in
//! `crates/argo-cli/tests/pitch_ask_endpoint.rs`. These tests cover
//! the pure shaping helpers — wire-format parsing and the
//! disposition → response-body mapping — without spinning up the
//! gateway.

use super::*;
use argo_capability::ClaimDisposition;

#[test]
fn channel_label_and_capability_are_aligned() {
    for ch in [
        PitchChannel::Rehearsal,
        PitchChannel::WebEmbed,
        PitchChannel::LiveMeeting,
        PitchChannel::Chat,
    ] {
        // Label is non-empty and unique enough to use in audit.
        assert!(!ch.label().is_empty());
        // Capability conversion is total.
        let _ = ch.as_channel_kind();
    }
}

#[test]
fn disposition_for_response_maps_allow() {
    let (label, reason) = disposition_for_response(&ClaimDisposition::Allow);
    assert_eq!(label, "Allow");
    assert!(reason.is_none());
}

#[test]
fn disposition_for_response_maps_forbid_with_reason() {
    let disp = ClaimDisposition::Forbid {
        refusal_template_id: "defer-to-mark-financial".into(),
    };
    let (label, reason) = disposition_for_response(&disp);
    assert_eq!(label, "Block");
    assert_eq!(
        reason.as_deref(),
        Some("forbidden_class: defer-to-mark-financial")
    );
}

#[test]
fn disposition_for_response_maps_escalate() {
    let (label, reason) = disposition_for_response(&ClaimDisposition::Escalate);
    assert_eq!(label, "Escalate");
    assert!(reason.is_none());
}

#[test]
fn render_answer_passes_through_allow_and_forbid() {
    let canonical = "Rust single-binary core.";
    assert_eq!(
        render_answer(canonical, &ClaimDisposition::Allow),
        canonical
    );
    assert_eq!(
        render_answer(
            canonical,
            &ClaimDisposition::Forbid {
                refusal_template_id: "x".into()
            }
        ),
        canonical
    );
}

#[test]
fn render_answer_rewrites_escalate() {
    let rendered = render_answer("private", &ClaimDisposition::Escalate);
    assert!(rendered.to_lowercase().contains("founder"));
}

#[test]
fn miss_fallback_is_not_empty() {
    assert!(!MISS_FALLBACK.trim().is_empty());
    // Spot-check: the operator's words in the brief contained
    // "founder" — make sure we don't drift away from that voice.
    assert!(MISS_FALLBACK.to_lowercase().contains("founder"));
}

#[tokio::test]
async fn bad_request_on_empty_body() {
    // Construct a minimal HTTP/1.1 wire-form with an empty body.
    let req = "POST /api/v1/pitch/ask HTTP/1.1\r\nHost: localhost\r\n\r\n";
    let (status, body) = ask_response(req, None).await;
    assert_eq!(status, "400 Bad Request", "status={status} body={body}");
    assert!(
        body.contains("bad_request"),
        "expected bad_request envelope, got {body}"
    );
}

#[tokio::test]
async fn bad_request_on_blank_question() {
    let payload = serde_json::json!({ "question": "   ", "channel": "Rehearsal" });
    let req = format!(
        "POST /api/v1/pitch/ask HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\n\r\n{payload}"
    );
    let (status, body) = ask_response(&req, None).await;
    assert_eq!(status, "400 Bad Request", "status={status} body={body}");
}

#[tokio::test]
async fn miss_falls_back_to_deterministic_when_llm_disabled_via_env() {
    // Disable LLM path explicitly so the test doesn't need a key.
    std::env::set_var(LLM_FALLBACK_ENV, "0");
    // Use a freshly off-topic question to guarantee a canonical miss
    // even if the operator has authored matching prompts later.
    let payload = serde_json::json!({
        "question": "Can you recite a haiku about arctic terns?",
        "channel": "Rehearsal",
    });
    let req = format!(
        "POST /api/v1/pitch/ask HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\n\r\n{payload}"
    );
    let (status, body) = ask_response(&req, None).await;
    std::env::remove_var(LLM_FALLBACK_ENV);
    assert_eq!(status, "200 OK", "status={status} body={body}");
    let parsed: serde_json::Value = serde_json::from_str(&body).expect("body is JSON");
    assert_eq!(parsed["matched"], false);
    // No LLM path → source stays canonical, deterministic miss text.
    assert_eq!(parsed["source"], "canonical");
    assert_eq!(parsed["disposition"], "Escalate");
    assert_eq!(parsed["answer"], MISS_FALLBACK);
    assert!(parsed.get("model").is_none(), "no model field on canonical path");
}

#[test]
fn llm_provider_env_constant_name_is_documented() {
    // Brief promises: provider routing flips via env without code
    // change once `provider-claude-code-cli-subprocess` lands. Pin
    // the env-var name so the sibling brief picks it up.
    assert_eq!(LLM_PROVIDER_ENV, "ARGO_PITCH_LLM_PROVIDER");
    assert_eq!(LLM_FALLBACK_ENV, "ARGO_PITCH_LLM_FALLBACK");
}
