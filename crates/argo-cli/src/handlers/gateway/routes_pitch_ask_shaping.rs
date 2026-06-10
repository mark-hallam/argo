//! Response-shaping helpers for `routes_pitch_ask`. Mechanical extraction
//! (no behaviour change) to keep the endpoint file under the per-file LOC
//! budget — these build the JSON wire body + audit payloads for the
//! canonical-hit and LLM-fallback paths, and the small error responders.

use super::{disposition_for_response, render_answer, PitchChannel};
use crate::handlers::gateway::routes_pitch_ask_llm::LlmAskOutcome;
use argo_capability::{evaluate_claim, ClaimCandidate, TruthBoundaryPolicy};
use serde_json::{json, Value};

pub(super) fn build_canonical_hit_payload(
    channel: PitchChannel,
    policy: &TruthBoundaryPolicy,
    matched: &argo_skill::canonical_qa::CanonicalMatch,
    latency_ms: u64,
) -> (Value, Value) {
    let evaluation = evaluate_claim(
        policy,
        channel.as_channel_kind(),
        &ClaimCandidate {
            text: matched.answer.clone(),
            declared_classes: matched.declared_claim_classes.clone(),
        },
    );
    let (disposition_label, block_reason) = disposition_for_response(&evaluation.disposition);
    let answer = render_answer(&matched.answer, &evaluation.disposition);
    let body = json!({
        "matched": true,
        "source": "canonical",
        "entry_id": matched.entry_id,
        "answer": answer,
        "declared_claim_classes": matched.declared_claim_classes,
        "disposition": disposition_label,
        "block_reason": block_reason,
        "channel": channel.label(),
        "latency_ms": latency_ms,
    });
    let audit = json!({
        "entry_id": matched.entry_id,
        "channel": channel.label(),
        "disposition": disposition_label,
        "declared_claim_classes": matched.declared_claim_classes,
        "latency_ms": latency_ms,
    });
    (body, audit)
}

pub(super) fn build_llm_payload(
    channel: PitchChannel,
    outcome: &LlmAskOutcome,
    latency_ms: u64,
) -> (Value, Value, Value) {
    let (disposition_label, block_reason) = disposition_for_response(&outcome.disposition);
    let source = if outcome.swapped_response {
        "llm_blocked_canonical_refused"
    } else {
        "llm"
    };
    let declared = outcome
        .matched_class
        .as_ref()
        .map(|c| vec![c.clone()])
        .unwrap_or_default();
    let mut body = json!({
        "matched": false,
        "source": source,
        "answer": outcome.answer,
        "declared_claim_classes": declared,
        "disposition": disposition_label,
        "block_reason": block_reason,
        "channel": channel.label(),
        "latency_ms": latency_ms,
        "model": outcome.model,
        "tokens": {
            "input": outcome.tokens_in,
            "output": outcome.tokens_out,
        },
    });
    if outcome.swapped_response {
        // The draft is the LLM's actual (unsafe) attempt. Surface it
        // for the operator's rehearsal surface so they can see what
        // the model wanted to say before the swap. Production
        // wire-formats should redact this in transit — out of scope
        // for v1.
        if let Some(map) = body.as_object_mut() {
            map.insert("draft".to_string(), Value::String(outcome.draft.clone()));
        }
    }
    let llm_audit = json!({
        "channel": channel.label(),
        "model": outcome.model,
        "latency_ms": latency_ms,
        "tokens_in": outcome.tokens_in,
        "tokens_out": outcome.tokens_out,
        "response_hash": outcome.response_hash,
    });
    let eval_audit = json!({
        "channel": channel.label(),
        "disposition": disposition_label,
        "block_reason": block_reason,
        "matched_class": outcome.matched_class,
        "swapped_response": outcome.swapped_response,
    });
    (body, llm_audit, eval_audit)
}

pub(super) fn bad_request(msg: &str) -> (&'static str, String) {
    (
        "400 Bad Request",
        json!({ "error": { "code": "bad_request", "message": msg } }).to_string(),
    )
}

pub(super) fn service_unavailable(msg: &str) -> (&'static str, String) {
    (
        "503 Service Unavailable",
        json!({ "error": { "code": "pitch_unavailable", "message": msg } }).to_string(),
    )
}
