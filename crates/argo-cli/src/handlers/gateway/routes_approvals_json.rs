//! `/api/v1/approvals` — JSON wrapper over the worker-action
//! approval state machine. Replaces the HTML `/inbox` page with a
//! cursor-paginated JSON list + per-id decide / undo endpoints
//! that the SPA frontend (E1-a) consumes.
//!
//! The HTML `/inbox` route (`routes_channels_webinbox`) is preserved
//! verbatim for legacy operators — this module is additive.
//!
//! State-machine mapping (sensible deviations from the brief, which
//! assumed a `pending → approved/refused → undone` state-machine
//! that does not exist):
//!
//! - `decide: "approve"` → `worker_actions_transitions::approve()`
//!   (state `pending → approved`).
//! - `decide: "refuse"`  → `worker_actions_transitions::cancel()`
//!   with a `"decision":"refuse"` note (state `pending → cancelled`).
//!   The `cancelled` row is what the API reports back as `"refuse"`.
//! - `POST /undo` within a 30-second window of the decision row's
//!   `created_at` appends a new `pending` memory row directly
//!   (bypassing the state-machine validator, as the existing
//!   `reject_action` helper in `routes_channels.rs` already does for
//!   non-validator append paths). This restores the action to the
//!   pending queue. Outside the window, returns `410 Gone`.
//!
//! Tenant scoping is workspace-derived (single-tenant per
//! workspace). Per-tenant filtering across a multi-tenant gateway
//! is closed by `mission-enterprise-approvals-rbac`.

use super::routes_auth_principal::principal_id_for_audit;
use crate::handlers::worker_actions::{ActionRecord, ActionState};
use crate::handlers::worker_actions_query::collect_latest_actions;
use crate::handlers::worker_actions_transitions::{
    self, find_action, open_workspace_memory,
};
use argo_memory::MemoryFile;
use argo_redact::{
    active_grants_for_principal, audit_payload, redact_approval_row, resolve_policy,
    ViewerPermissions,
};
use serde::Deserialize;
use serde_json::{json, Value};

/// Hard cap on `?limit=`. Brief: default 50, max 100.
const DEFAULT_LIMIT: usize = 50;
const MAX_LIMIT: usize = 100;
/// Window during which a decision is reversible via `POST /undo`.
const UNDO_WINDOW_SECS: i64 = 30;

/// `GET /api/v1/approvals?cursor=<c>&limit=<n>` — paginated list
/// of currently-pending approvals (state `pending` AND
/// `requires_approval = true`). Returns
/// `{ items: ApprovalRow[], next_cursor: <opt> }`.
pub(super) async fn list_response(req: &str) -> (&'static str, String) {
    let tail = req.strip_prefix("GET ").unwrap_or(req);
    let limit = match parse_limit(tail) {
        Ok(n) => n,
        Err(resp) => return resp,
    };
    let cursor = parse_query(tail, "cursor").and_then(|c| decode_cursor(&c));
    let actions = match collect_latest_actions(None).await {
        Ok(v) => v,
        Err(e) => return server_error(&format!("collect actions: {e}")),
    };
    let mut pending: Vec<ActionRecord> = actions
        .into_iter()
        .filter(|a| a.state == ActionState::Pending.as_str() && a.requires_approval)
        .collect();
    // Newest-first by created_at, ties broken by action_id for determinism.
    pending.sort_by(|a, b| {
        b.created_at
            .cmp(&a.created_at)
            .then_with(|| a.action_id.cmp(&b.action_id))
    });
    if let Some((ts, id)) = cursor.as_ref() {
        // Resume strictly after the prior page's last row (timestamp DESC,
        // id ASC) — append `< ts` OR (`== ts` AND `> id`).
        pending.retain(|a| {
            a.created_at.as_str() < ts.as_str()
                || (a.created_at.as_str() == ts.as_str() && a.action_id.as_str() > id.as_str())
        });
    }
    let take = pending.len().min(limit);
    let page: Vec<ActionRecord> = pending.into_iter().take(take).collect();
    let next_cursor = if page.len() == limit {
        page.last().map(|a| encode_cursor(&a.created_at, &a.action_id))
    } else {
        None
    };
    let actor_id = principal_id_for_audit(req).unwrap_or_else(|| "anonymous".to_string());
    // v1 viewer-permissions resolution: the gateway has no
    // principal→permission mapping yet (RBAC permissions row lands
    // with mission-enterprise-approvals-rbac). Until then the fence
    // applies the strict default to every viewer — defence in depth
    // matches what E1-e's client layer already shows users.
    let viewer = ViewerPermissions::strict();
    let grants = active_grants_for_principal(&actor_id);
    let policy = resolve_policy(&viewer, &grants);
    let items: Vec<Value> = page
        .iter()
        .map(|rec| {
            let row = approval_row_json(rec);
            let redaction = redact_approval_row(row, &policy, grants.first_id().map(str::to_string));
            if redaction.was_masked() {
                let payload = audit_payload(&actor_id, &rec.action_id, &redaction);
                tracing::info!(
                    target: "audit::approval_row_redacted",
                    event_type = argo_redact::AUDIT_EVENT_TYPE,
                    actor_id = %actor_id,
                    approval_id = %rec.action_id,
                    payload = %payload,
                    "approval row redacted by server fence"
                );
            }
            redaction.row
        })
        .collect();
    let mut body = json!({ "items": items });
    if let Some(c) = next_cursor {
        body["next_cursor"] = Value::String(c);
    }
    ("200 OK", body.to_string())
}

#[derive(Deserialize)]
struct DecideBody {
    decision: String,
}

/// `POST /api/v1/approvals/<id>/decide` — record approve/refuse.
/// Returns `{ ok, approval_id, decision }`. Idempotent: re-deciding
/// with the *same* decision returns 200; with a *different* decision
/// returns 409.
pub(super) async fn decide_response(approval_id: &str, req: &str) -> (&'static str, String) {
    let body = super::body_from_request(req);
    let Ok(parsed): Result<DecideBody, _> = serde_json::from_str(body) else {
        return bad_request("invalid JSON body");
    };
    let decision = match parsed.decision.as_str() {
        "approve" => "approve",
        "refuse" => "refuse",
        _ => return bad_request("decision must be \"approve\" or \"refuse\""),
    };
    let (memory, ws) = match open_workspace_memory().await {
        Ok(p) => p,
        Err(e) => return server_error(&format!("open workspace: {e}")),
    };
    let Some(located) = (match find_action(&memory, ws, approval_id).await {
        Ok(v) => v,
        Err(e) => return server_error(&format!("find_action: {e}")),
    }) else {
        return not_found(approval_id);
    };
    if !located.record.requires_approval {
        return bad_request("approval does not require approval");
    }
    let actor = principal_id_for_audit(req).unwrap_or_else(|| "gateway".to_string());
    let current_state = located.record.state.as_str();
    let current_decision = decision_from_record(&located.record);
    // Idempotency: same decision → no-op 200. Different decision → 409.
    if current_state != ActionState::Pending.as_str() {
        if current_decision.as_deref() == Some(decision) {
            return ok_decide(approval_id, decision);
        }
        return conflict(approval_id, current_decision.as_deref().unwrap_or(current_state));
    }
    let result = if decision == "approve" {
        worker_actions_transitions::approve(&memory, ws, approval_id, &actor, None).await
    } else {
        worker_actions_transitions::cancel(
            &memory,
            ws,
            approval_id,
            &actor,
            Some("decision:refuse"),
        )
        .await
    };
    match result {
        Ok(_) => ok_decide(approval_id, decision),
        Err(e) => server_error(&format!("decide: {e}")),
    }
}

/// `POST /api/v1/approvals/<id>/undo` — reverse the decision when
/// within the [`UNDO_WINDOW_SECS`] window. Outside the window the
/// transition is terminal, so the API returns `410 Gone`.
pub(super) async fn undo_response(approval_id: &str, _req: &str) -> (&'static str, String) {
    let (memory, ws) = match open_workspace_memory().await {
        Ok(p) => p,
        Err(e) => return server_error(&format!("open workspace: {e}")),
    };
    let Some(located) = (match find_action(&memory, ws, approval_id).await {
        Ok(v) => v,
        Err(e) => return server_error(&format!("find_action: {e}")),
    }) else {
        return not_found(approval_id);
    };
    if located.record.state == ActionState::Pending.as_str() {
        return bad_request("approval has not been decided yet");
    }
    if decision_from_record(&located.record).is_none() {
        return bad_request("approval was not decided via the JSON API");
    }
    let decided_at = chrono::DateTime::parse_from_rfc3339(&located.record.created_at)
        .map(|dt| dt.with_timezone(&chrono::Utc));
    let now = chrono::Utc::now();
    let within_window = matches!(decided_at, Ok(t) if (now - t).num_seconds() <= UNDO_WINDOW_SECS);
    if !within_window {
        return gone(approval_id);
    }
    // Restore to pending. Append a memory row directly — the state-machine
    // validator forbids `approved/cancelled → pending`, but appending a
    // newer row is the audit-clean way to revert (parallels the
    // `reject_action` direct-append in `routes_channels.rs`).
    let mut next = located.record.clone();
    next.state = ActionState::Pending.as_str().to_string();
    next.created_at = now.to_rfc3339();
    next.approved_by = None;
    next.approved_at = None;
    next.result = Some(json!({"undone_at": next.created_at}));
    let body = match serde_json::to_string(&next) {
        Ok(s) => s,
        Err(e) => return server_error(&format!("serialize: {e}")),
    };
    if let Err(e) = memory
        .append_for_vertical(ws, MemoryFile::Memory, body, None, &located.vertical_id)
        .await
    {
        return server_error(&format!("undo append: {e}"));
    }
    (
        "200 OK",
        json!({"ok": true, "approval_id": approval_id}).to_string(),
    )
}

fn approval_row_json(rec: &ActionRecord) -> Value {
    // The frontend `ApprovalItem` shape (web/lib/sample-approvals.ts)
    // carries UX-specific fields (verb / detail / consequences / reason)
    // that don't yet exist on `ActionRecord`. v1 derives placeholders
    // from the record fields available; J3+ ship the structured prompt
    // pack that feeds these directly.
    let action_type = rec.action_type.replace('_', " ");
    let detail = rec
        .payload
        .as_object()
        .and_then(|m| m.get("summary").or_else(|| m.get("text")))
        .and_then(Value::as_str)
        .map(str::to_string)
        .unwrap_or_else(|| short_payload(&rec.payload));
    json!({
        "id": rec.action_id,
        "worker": {
            "id": rec.worker_id.to_string(),
            "name": rec.worker_id.to_string(),
            "avatar_url": Value::Null,
        },
        "verb": format!("wants to {action_type}"),
        "detail": detail,
        "consequence_if_yes": "",
        "consequence_if_no": "",
        "reason": Value::Null,
        "channel": "web",
        "received_at": rec.created_at,
        "decision": Value::Null,
        "risk_level": rec.risk_level,
    })
}

fn short_payload(v: &Value) -> String {
    let s = serde_json::to_string(v).unwrap_or_default();
    if s.len() > 200 {
        format!("{}…", &s[..199])
    } else {
        s
    }
}

fn decision_from_record(rec: &ActionRecord) -> Option<String> {
    match rec.state.as_str() {
        s if s == ActionState::Approved.as_str() => Some("approve".to_string()),
        s if s == ActionState::Cancelled.as_str() => rec
            .result
            .as_ref()
            .and_then(|r| r.pointer("/cancellation/note"))
            .and_then(Value::as_str)
            .filter(|n| n.starts_with("decision:refuse"))
            .map(|_| "refuse".to_string()),
        _ => None,
    }
}

fn parse_limit(tail: &str) -> Result<usize, (&'static str, String)> {
    let Some(raw) = parse_query(tail, "limit") else {
        return Ok(DEFAULT_LIMIT);
    };
    let Ok(n) = raw.parse::<usize>() else {
        return Err(bad_request(&format!("limit '{raw}' not a number")));
    };
    if n == 0 || n > MAX_LIMIT {
        return Err(bad_request(&format!("limit must be 1..={MAX_LIMIT}")));
    }
    Ok(n)
}

fn parse_query(tail: &str, key: &str) -> Option<String> {
    let head = tail.split_once(' ').map(|(p, _)| p).unwrap_or(tail);
    let qs = head.split_once('?').map(|(_, q)| q)?;
    qs.split('&').find_map(|kv| {
        let (k, v) = kv.split_once('=')?;
        (k == key).then(|| v.to_string())
    })
}

fn encode_cursor(ts: &str, id: &str) -> String {
    use base64::{engine::general_purpose, Engine as _};
    general_purpose::URL_SAFE_NO_PAD.encode(format!("{ts}|{id}"))
}

fn decode_cursor(c: &str) -> Option<(String, String)> {
    use base64::{engine::general_purpose, Engine as _};
    let bytes = general_purpose::URL_SAFE_NO_PAD.decode(c).ok()?;
    let s = String::from_utf8(bytes).ok()?;
    let (ts, id) = s.split_once('|')?;
    Some((ts.to_string(), id.to_string()))
}

fn ok_decide(approval_id: &str, decision: &str) -> (&'static str, String) {
    (
        "200 OK",
        json!({
            "ok": true,
            "approval_id": approval_id,
            "decision": decision,
        })
        .to_string(),
    )
}

fn bad_request(message: &str) -> (&'static str, String) {
    error_envelope("400 Bad Request", "bad_request", message)
}

fn not_found(id: &str) -> (&'static str, String) {
    error_envelope(
        "404 Not Found",
        "approval_not_found",
        &format!("no approval with id '{id}'"),
    )
}

fn conflict(id: &str, current: &str) -> (&'static str, String) {
    error_envelope(
        "409 Conflict",
        "already_decided",
        &format!("approval '{id}' already decided ({current})"),
    )
}

fn gone(id: &str) -> (&'static str, String) {
    error_envelope(
        "410 Gone",
        "undo_window_expired",
        &format!("approval '{id}': undo window has expired"),
    )
}

fn server_error(message: &str) -> (&'static str, String) {
    error_envelope("500 Internal Server Error", "internal_error", message)
}

fn error_envelope(
    status: &'static str,
    code: &str,
    message: &str,
) -> (&'static str, String) {
    (
        status,
        json!({
            "error": {
                "code": code,
                "message": message,
            }
        })
        .to_string(),
    )
}

#[cfg(test)]
#[path = "routes_approvals_json_tests.rs"]
mod tests;
