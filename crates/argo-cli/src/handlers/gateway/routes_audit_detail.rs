//! `GET /api/v1/audit/<actionId>` — full AuditDetail (Stream J15).
//! Read-only over `audit_record`; the 8-step decision chain,
//! provenance, output, and external-ref pre-scan are all composed
//! from `payload_json` (v1 audit schema doesn't break the fields
//! out as columns). Single tenant per workspace today, so
//! "cross-tenant" 404 is structurally guaranteed by the unknown-id
//! path. J2 redaction is applied to summary, inputs, decision-chain
//! bodies, and the output body before serialisation.

use super::routes_audit_screenshot::{sign_screenshot_url, SCREENSHOT_URL_TTL_SECS};
use super::routes_auth_principal::principal_id_for_audit;
use super::store_unavailable;
use argo_audit::{AuditKind, AuditSink};
use argo_redact::{redact_text, RedactionPolicy};
use argo_store::SessionStore;
use serde_json::{json, Value};
use std::sync::Arc;

/// Per-output cap on the number of external references emitted by
/// the pre-scan; bounds compute and response size when an
/// adversarial / unusually-large output body trips many matches.
const MAX_EXTERNAL_REFS: usize = 30;

/// `GET /api/v1/audit/<actionId>` — see module docs.
pub(super) async fn detail_response(
    req: &str,
    action_id: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return store_unavailable();
    };
    let id_bytes = match hex::decode(action_id) {
        Ok(b) if b.len() == 16 => b,
        _ => return not_found(),
    };
    let row = match fetch_row(store, &id_bytes).await {
        Ok(Some(r)) => r,
        Ok(None) => return not_found(),
        Err(e) => return server_error(&format!("audit detail fetch: {e}")),
    };
    let payload: Value = serde_json::from_str(&row.payload_json).unwrap_or(Value::Null);
    let policy = RedactionPolicy::default();
    let body = json!({
        "detail": build_detail(action_id, &row, &payload, &policy),
    });
    emit_view_audit(store, req, action_id).await;
    ("200 OK", body.to_string())
}

#[derive(sqlx::FromRow)]
struct DetailRow {
    id: Vec<u8>,
    session_id: Option<Vec<u8>>,
    kind: String,
    payload_json: String,
    self_hash: Vec<u8>,
    created_at: i64,
}

async fn fetch_row(
    store: &Arc<SessionStore>,
    id_bytes: &[u8],
) -> Result<Option<DetailRow>, sqlx::Error> {
    sqlx::query_as::<_, DetailRow>(
        "SELECT id, session_id, kind, payload_json, self_hash, created_at \
         FROM audit_record WHERE id = ? LIMIT 1",
    )
    .bind(id_bytes)
    .fetch_optional(store.pool())
    .await
}

fn build_detail(
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
    let worker_name = payload_str(payload, "worker_name")
        .unwrap_or_else(|| if worker_id.is_empty() { "unknown".into() } else { worker_id.clone() });
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
        (1, "Memory loaded", "memory_loaded_summary",
            json!({ "snapshot_id": g("memory_loaded_snapshot_id") })),
        (2, "Skill chosen", "skill_summary",
            json!({ "skill_id": g("skill_id"), "skill_version": g("skill_version"), "skill_trust": g("skill_trust") })),
        (3, "Plan candidates", "plan_candidates_summary",
            json!({ "candidates": g("plan_candidates_json") })),
        (4, "Plan selected", "plan_selected_summary",
            json!({ "selected_idx": g("plan_selected_idx") })),
        (5, "Policy evaluated", "policy_summary",
            json!({ "policy_id": g("policy_id"), "outcome": g("policy_outcome") })),
        (6, "Approval", "approval_summary",
            json!({ "approver_id": g("approver_id"), "channel": g("approval_channel") })),
        (7, "Execution", "executor_summary",
            json!({ "kind": g("executor_kind"), "meta": g("executor_meta") })),
        (8, "Outcome", "outcome_summary",
            json!({ "status": g("outcome_status"), "downstream": g("downstream_meta") })),
    ];
    let out: Vec<Value> = steps
        .into_iter()
        .map(|(n, title, body_key, details)| decision_step(n, title, body_key, policy, payload, details))
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
    let screenshots = payload.get("screenshots").and_then(Value::as_array).map(|arr| {
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
    let raw = shot.get("description").and_then(Value::as_str).unwrap_or("");
    let (description, _) = redact_text(raw, policy);
    json!({
        "url": sign_screenshot_url(action_id, idx, exp),
        "timestamp": timestamp,
        "description": description,
    })
}

fn build_external_refs(payload: &Value) -> Value {
    let body_text = match payload.get("output_body") {
        Some(Value::String(s)) => s.clone(),
        Some(other) => other.to_string(),
        None => return Value::Array(Vec::new()),
    };
    let urls = iter_urls(&body_text);
    let mut hits: Vec<Value> = Vec::new();
    scan_stripe(&body_text, &mut hits);
    scan_slack(&urls, &mut hits);
    scan_github(&urls, &mut hits);
    for token in iter_uppercase_dashed_ids(&body_text) {
        hits.push(ref_entry("linear", &token, &format!("https://linear.app/issue/{token}")));
    }
    for (system, marker) in URL_SCANNERS {
        scan_url_substring(&urls, system, marker, &mut hits);
    }
    hits.truncate(MAX_EXTERNAL_REFS);
    Value::Array(hits)
}

const URL_SCANNERS: &[(&str, &str)] = &[
    ("salesforce", "salesforce.com/"),
    ("hubspot", "hubspot.com/"),
    ("zendesk", ".zendesk.com/agent/tickets/"),
    ("jira", ".atlassian.net/browse/"),
];

fn scan_stripe(text: &str, out: &mut Vec<Value>) {
    static PREFIXES: &[(&str, &str)] = &[
        ("re_", "refunds"),
        ("ch_", "payments"),
        ("cus_", "customers"),
        ("pi_", "payments"),
        ("in_", "invoices"),
        ("sub_", "subscriptions"),
    ];
    for (prefix, path) in PREFIXES {
        for token in matches_with_prefix(text, prefix) {
            let url = format!("https://dashboard.stripe.com/{path}/{token}");
            out.push(ref_entry("stripe", &token, &url));
        }
    }
}

fn scan_slack(urls: &[String], out: &mut Vec<Value>) {
    for url in urls {
        if let Some(rest) = url.strip_prefix("https://") {
            if let Some(tail) = rest.split_once(".slack.com/archives/").map(|(_, t)| t) {
                let id: String = tail
                    .chars()
                    .take_while(|c| c.is_ascii_alphanumeric() || matches!(c, '/' | '-'))
                    .collect();
                if !id.is_empty() {
                    out.push(ref_entry("slack", &id, url));
                }
            }
        }
    }
}

fn scan_github(urls: &[String], out: &mut Vec<Value>) {
    for url in urls {
        let Some(rest) = url.strip_prefix("https://github.com/") else { continue };
        let parts: Vec<&str> = rest.split('/').collect();
        if parts.len() >= 4 && (parts[2] == "issues" || parts[2] == "pull") {
            let id = format!("{}/{}#{}", parts[0], parts[1], parts[3]);
            out.push(ref_entry("github", &id, url));
        }
    }
}

fn scan_url_substring(urls: &[String], system: &str, marker: &str, out: &mut Vec<Value>) {
    for url in urls {
        if url.contains(marker) {
            let id = url.rsplit('/').next().unwrap_or(url).to_string();
            out.push(ref_entry(system, &id, url));
        }
    }
}

fn ref_entry(system: &str, id: &str, deep_link: &str) -> Value {
    json!({ "system": system, "id": id, "deep_link": deep_link })
}

/// Iterate ASCII tokens that begin with `prefix` and run until the
/// next non-`[A-Za-z0-9_]` byte. Used by the Stripe scanner.
fn matches_with_prefix(text: &str, prefix: &str) -> Vec<String> {
    let mut out = Vec::new();
    let bytes = text.as_bytes();
    let prefix_bytes = prefix.as_bytes();
    let mut i = 0;
    while i + prefix_bytes.len() <= bytes.len() {
        let starts = bytes[i..i + prefix_bytes.len()] == *prefix_bytes;
        let boundary = i == 0 || !is_id_byte(bytes[i - 1]);
        if starts && boundary {
            let mut j = i + prefix_bytes.len();
            while j < bytes.len() && is_id_byte(bytes[j]) {
                j += 1;
            }
            if j > i + prefix_bytes.len() {
                out.push(std::str::from_utf8(&bytes[i..j]).unwrap_or("").to_string());
            }
            i = j.max(i + 1);
        } else {
            i += 1;
        }
    }
    out
}

fn is_id_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

fn iter_urls(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for token in text.split_ascii_whitespace() {
        let cleaned = token.trim_end_matches([',', '.', ')', ']', '"', '\'']);
        if cleaned.starts_with("https://") || cleaned.starts_with("http://") {
            out.push(cleaned.to_string());
        }
    }
    out
}

fn iter_uppercase_dashed_ids(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if !bytes[i].is_ascii_uppercase() {
            i += 1;
            continue;
        }
        let start = i;
        while i < bytes.len() && bytes[i].is_ascii_uppercase() { i += 1; }
        if i < bytes.len() && bytes[i] == b'-' && start + 2 <= i {
            let dash = i;
            i += 1;
            let num_start = i;
            while i < bytes.len() && bytes[i].is_ascii_digit() { i += 1; }
            if i > num_start {
                out.push(std::str::from_utf8(&bytes[start..i]).unwrap_or("").to_string());
            } else {
                i = dash;
            }
        }
    }
    out
}

fn payload_str(p: &Value, key: &str) -> Option<String> {
    p.get(key).and_then(Value::as_str).map(str::to_string)
}

async fn emit_view_audit(store: &Arc<SessionStore>, req: &str, action_id: &str) {
    let payload = json!({
        "endpoint": "audit.detail.viewed",
        "viewed_action_id": action_id,
    });
    let sink = AuditSink::new(Arc::clone(store));
    if let Err(e) = sink
        .write(None, AuditKind::HttpRequest, payload, principal_id_for_audit(req))
        .await
    {
        tracing::warn!(
            target: "gateway::audit_detail",
            error = %e,
            "audit.detail.viewed write failed",
        );
    }
}

fn not_found() -> (&'static str, String) {
    (
        "404 Not Found",
        json!({"error": {"code": "not_found", "message": "audit row not found"}}).to_string(),
    )
}

fn server_error(message: &str) -> (&'static str, String) {
    (
        "500 Internal Server Error",
        json!({"error": {"code": "internal_error", "message": message}}).to_string(),
    )
}

#[cfg(test)]
#[path = "routes_audit_detail_tests.rs"]
mod tests;
