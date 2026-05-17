//! Row-to-JSON render helpers for the audit-detail endpoint — split from
//! `routes_audit_detail.rs` to keep that file under the 500-LOC budget.
//! All helpers take an already-fetched `DetailRow` + parsed payload and emit
//! the redaction-aware JSON shape consumed by the frontend.

use super::routes_audit_detail::DetailRow;
use super::routes_audit_detail_refs::build_external_refs;
use super::routes_audit_screenshot::{sign_screenshot_url, SCREENSHOT_URL_TTL_SECS};
use argo_redact::{redact_text, RedactionPolicy};
use serde_json::{json, Value};

pub(super) fn build_detail(
    action_id: &str,
    row: &DetailRow,
    payload: &Value,
    policy: &RedactionPolicy,
) -> Value {
    json!({
        "summary": build_summary(row, payload, policy),
        "inputs": build_inputs(row, payload, policy),
        "decision_chain": build_decision_chain(payload, policy),
        "provenance": build_provenance(row, payload),
        "output": build_output(action_id, payload, policy),
        "external_refs": build_external_refs(payload),
    })
}

fn build_summary(row: &DetailRow, payload: &Value, policy: &RedactionPolicy) -> Value {
    let worker_id = payload_str(payload, "worker_id").unwrap_or_default();
    let worker_name = payload_str(payload, "worker_name").unwrap_or_else(|| {
        if worker_id.is_empty() {
            "unknown".into()
        } else {
            worker_id.clone()
        }
    });
    let (verb, _) = redact_text(
        &payload_str(payload, "verb").unwrap_or_else(|| row.kind.clone()),
        policy,
    );
    let timestamp = chrono::DateTime::<chrono::Utc>::from_timestamp_micros(row.created_at)
        .map_or_else(|| row.created_at.to_string(), |dt| dt.to_rfc3339());
    let approver_json = payload_str(payload, "approver_id").map_or(Value::Null, |id| {
        let raw = payload_str(payload, "approver_name").unwrap_or_else(|| id.clone());
        let (name, _) = redact_text(&raw, policy);
        json!({"id": id, "name": name})
    });
    json!({
        "id": hex::encode(&row.id),
        "timestamp": timestamp,
        "worker": { "id": worker_id, "name": worker_name },
        "action_type": payload_str(payload, "action_type").unwrap_or_else(|| row.kind.clone()),
        "verb": verb,
        "status": payload_str(payload, "outcome_status")
            .or_else(|| payload_str(payload, "status"))
            .unwrap_or_else(|| "succeeded".into()),
        "approver": approver_json,
        "channel": payload_str(payload, "channel").unwrap_or_else(|| "web".into()),
        "cost": payload.get("cost_usd").and_then(Value::as_f64)
            .map_or(Value::Null, |usd| json!({"usd": usd})),
        "data_classification": payload_str(payload, "data_classification")
            .or_else(|| payload_str(payload, "classification"))
            .unwrap_or_else(|| "internal".into()),
        "session_id": row.session_id.as_ref().map(hex::encode).map_or(Value::Null, Value::String),
    })
}

fn build_inputs(row: &DetailRow, payload: &Value, policy: &RedactionPolicy) -> Value {
    let trigger_kind = payload_str(payload, "trigger_kind")
        .unwrap_or_else(|| infer_trigger_kind(&row.kind).into());
    let trigger_raw = payload_str(payload, "trigger_summary")
        .or_else(|| payload_str(payload, "verb"))
        .unwrap_or_else(|| row.kind.clone());
    let (trigger_summary, _) = redact_text(&trigger_raw, policy);
    let predecessor = payload.get("predecessor").and_then(|p| {
        let id = p.get("id").and_then(Value::as_str)?;
        let verb_raw = p.get("verb").and_then(Value::as_str).unwrap_or("");
        let (verb, _) = redact_text(verb_raw, policy);
        Some(json!({ "id": id, "verb": verb }))
    });
    json!({
        "trigger_kind": trigger_kind,
        "trigger_summary": trigger_summary,
        "predecessor": predecessor,
    })
}

fn infer_trigger_kind(kind: &str) -> &'static str {
    match kind {
        "tool_call" | "tool_result" => "dependent_action",
        "http.request" => "message",
        _ => "message",
    }
}

fn build_decision_chain(payload: &Value, policy: &RedactionPolicy) -> Value {
    let g = |k: &str| payload.get(k).cloned().unwrap_or(Value::Null);
    let steps = [
        (
            1,
            "Memory loaded",
            "memory_loaded_summary",
            json!({ "snapshot_id": g("memory_loaded_snapshot_id") }),
        ),
        (
            2,
            "Skill chosen",
            "skill_summary",
            json!({ "skill_id": g("skill_id"), "skill_version": g("skill_version"), "skill_trust": g("skill_trust") }),
        ),
        (
            3,
            "Plan candidates",
            "plan_candidates_summary",
            json!({ "candidates": g("plan_candidates_json") }),
        ),
        (
            4,
            "Plan selected",
            "plan_selected_summary",
            json!({ "selected_idx": g("plan_selected_idx") }),
        ),
        (
            5,
            "Policy evaluated",
            "policy_summary",
            json!({ "policy_id": g("policy_id"), "outcome": g("policy_outcome") }),
        ),
        (
            6,
            "Approval",
            "approval_summary",
            json!({ "approver_id": g("approver_id"), "channel": g("approval_channel") }),
        ),
        (
            7,
            "Execution",
            "executor_summary",
            json!({ "kind": g("executor_kind"), "meta": g("executor_meta") }),
        ),
        (
            8,
            "Outcome",
            "outcome_summary",
            json!({ "status": g("outcome_status"), "downstream": g("downstream_meta") }),
        ),
    ];
    let out: Vec<Value> = steps
        .into_iter()
        .map(|(n, title, body_key, details)| {
            decision_step(n, title, body_key, policy, payload, details)
        })
        .collect();
    Value::Array(out)
}

fn decision_step(
    n: u64,
    title: &str,
    body_key: &str,
    policy: &RedactionPolicy,
    payload: &Value,
    details: Value,
) -> Value {
    let raw_body = payload_str(payload, body_key).unwrap_or_default();
    let (body, _) = redact_text(&raw_body, policy);
    json!({
        "n": n,
        "title": title,
        "body": body,
        "details": details,
    })
}

fn build_provenance(row: &DetailRow, payload: &Value) -> Value {
    let s = |k: &str| payload_str(payload, k).unwrap_or_default();
    json!({
        "model": { "id": s("model_id"), "version": s("model_version") },
        "prompt_template": { "id": s("prompt_template_id"), "hash": s("prompt_template_hash") },
        "skill": { "id": s("skill_id"), "version": s("skill_version") },
        "memory_snapshot": { "id": s("memory_loaded_snapshot_id") },
        "policy": { "id": s("policy_id"), "version": s("policy_version"), "hash": s("policy_hash") },
        "hash_chain_anchor": hex::encode(&row.self_hash),
        "signature": s("signature"),
    })
}

fn build_output(action_id: &str, payload: &Value, policy: &RedactionPolicy) -> Value {
    let body = match payload.get("output_body") {
        Some(Value::String(s)) => Value::String(redact_text(s, policy).0),
        Some(other) => other.clone(),
        None => Value::Null,
    };
    let screenshots = payload
        .get("screenshots")
        .and_then(Value::as_array)
        .map(|arr| {
            let exp = chrono::Utc::now().timestamp() + SCREENSHOT_URL_TTL_SECS;
            let entries: Vec<Value> = arr
                .iter()
                .enumerate()
                .map(|(idx, shot)| build_screenshot_entry(action_id, idx, exp, shot, policy))
                .collect();
            Value::Array(entries)
        });
    json!({
        "kind": payload_str(payload, "output_kind").unwrap_or_else(|| "text".into()),
        "body": body,
        "screenshots": screenshots.unwrap_or(Value::Null),
        "delivery": payload.get("delivery").cloned().unwrap_or(Value::Null),
        "follow_up": payload.get("follow_up").cloned().unwrap_or(Value::Null),
    })
}

fn build_screenshot_entry(
    action_id: &str,
    idx: usize,
    exp: i64,
    shot: &Value,
    policy: &RedactionPolicy,
) -> Value {
    let timestamp = shot
        .get("timestamp")
        .and_then(Value::as_str)
        .map_or(Value::Null, |s| Value::String(s.into()));
    let raw = shot
        .get("description")
        .and_then(Value::as_str)
        .unwrap_or("");
    let (description, _) = redact_text(raw, policy);
    json!({
        "url": sign_screenshot_url(action_id, idx, exp),
        "timestamp": timestamp,
        "description": description,
    })
}

pub(super) fn payload_str(p: &Value, key: &str) -> Option<String> {
    p.get(key).and_then(Value::as_str).map(str::to_string)
}
