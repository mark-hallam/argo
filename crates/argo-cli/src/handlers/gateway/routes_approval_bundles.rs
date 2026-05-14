//! `/api/v1/approvals/bundles` — server-defined approval bundles
//! (Stream `mission-approval-bundle-ui`).
//!
//! - `GET  /api/v1/approvals/bundles[?status=pending]` — pending
//!   bundles for the workspace tenant, oldest-first.
//! - `POST /api/v1/approvals/bundles` — worker creates a bundle.
//! - `POST /api/v1/approvals/bundles/<id>/decide` — operator
//!   decides; per-member transitions cascade through the existing
//!   `worker_actions_transitions` state machine. The bundle row is
//!   flipped FIRST so concurrent operators short-circuit; the loop
//!   then drives still-pending members to terminal. Already-decided
//!   members are skipped (surfaced via `members_skipped`).
//!
//! Bearer auth + principal-header parse run upstream in
//! `dispatch_post`. Tenant scoping is workspace-derived (single
//! tenant per workspace).

use super::body_from_request;
use super::routes_auth_principal::principal_id_for_audit;
use crate::handlers::worker_actions_transitions::{
    self, find_action, open_workspace_memory,
};
use argo_memory::MemoryStore;
use argo_store::approval_bundle::{
    create_bundle, decide_bundle, get_bundle, list_pending_bundles, ApprovalBundle, BundleDecision,
    BundleStatus, NewBundle,
};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, TenantId, WorkspaceId};
use serde_json::{json, Value};
use std::sync::Arc;

const DEFAULT_LIMIT: u32 = 50;
const MAX_LIMIT: u32 = 100;

/// `GET /api/v1/approvals/bundles?status=pending&limit=N`.
pub(super) async fn list_response(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let store = match store {
        Some(s) => s,
        None => return super::store_unavailable(),
    };
    let tail = req.strip_prefix("GET ").unwrap_or(req);
    if let Some(status) = parse_query(tail, "status") {
        if status != "pending" {
            return bad_request(&format!(
                "status='{status}' unsupported; only 'pending' is queryable in v1"
            ));
        }
    }
    let limit = match parse_limit(tail) {
        Ok(n) => n,
        Err(resp) => return resp,
    };
    let (tenant, _ws) = match resolve_workspace_tenant(store).await {
        Ok(t) => t,
        Err(e) => return server_error(&format!("resolve tenant: {e}")),
    };
    let bundles = match list_pending_bundles(store.pool(), tenant, limit).await {
        Ok(v) => v,
        Err(e) => return server_error(&format!("list bundles: {e}")),
    };
    let items: Vec<Value> = bundles.iter().map(bundle_to_json).collect();
    ("200 OK", json!({ "items": items }).to_string())
}

/// `POST /api/v1/approvals/bundles` — worker creates a bundle.
/// Body: `{ label: string, members: string[] }`.
pub(super) async fn create_response(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let store = match store {
        Some(s) => s,
        None => return super::store_unavailable(),
    };
    let body = body_from_request(req);
    let parsed: Value = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(e) => return bad_request(&format!("parse body: {e}")),
    };
    let label = match parsed.get("label").and_then(Value::as_str) {
        Some(s) if !s.trim().is_empty() => s.trim().to_string(),
        _ => return bad_request("`label` (non-empty string) is required"),
    };
    let members = match parse_members(parsed.get("members")) {
        Ok(v) => v,
        Err(msg) => return bad_request(&msg),
    };
    let (tenant, _ws) = match resolve_workspace_tenant(store).await {
        Ok(t) => t,
        Err(e) => return server_error(&format!("resolve tenant: {e}")),
    };
    let id = match create_bundle(
        store.pool(),
        &NewBundle {
            tenant_id: tenant,
            label: &label,
            member_approval_ids: members,
        },
    )
    .await
    {
        Ok(id) => id,
        Err(e) => return server_error(&format!("create bundle: {e}")),
    };
    let row = match get_bundle(store.pool(), &id).await {
        Ok(Some(b)) => b,
        Ok(None) => return server_error("bundle vanished post-insert"),
        Err(e) => return server_error(&format!("reload bundle: {e}")),
    };
    ("201 Created", bundle_to_json(&row).to_string())
}

/// `POST /api/v1/approvals/bundles/<id>/decide` — operator decides.
/// Body: `{ decision: "approve" | "refuse" }`.
pub(super) async fn decide_response(
    bundle_id: &str,
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let store = match store {
        Some(s) => s,
        None => return super::store_unavailable(),
    };
    let body = body_from_request(req);
    let parsed: Value = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(e) => return bad_request(&format!("parse body: {e}")),
    };
    let decision = match parsed.get("decision").and_then(Value::as_str) {
        Some("approve") => BundleDecision::Approve,
        Some("refuse") => BundleDecision::Refuse,
        _ => return bad_request("decision must be \"approve\" or \"refuse\""),
    };
    let actor = principal_id_for_audit(req).unwrap_or_else(|| "gateway".to_string());

    // Idempotency: same-decision repeat → 200, different → 409.
    let pre = match get_bundle(store.pool(), bundle_id).await {
        Ok(Some(b)) => b,
        Ok(None) => return not_found(bundle_id),
        Err(e) => return server_error(&format!("load bundle: {e}")),
    };
    if pre.status != BundleStatus::Pending {
        let prior = pre.decision.as_deref().unwrap_or("");
        return if prior == decision.as_str() {
            ok_decide(bundle_id, decision.as_str(), 0, pre.members.len())
        } else {
            conflict(bundle_id, prior)
        };
    }
    // Flip the bundle row first so concurrent operators short-
    // circuit as no-ops; then drive per-member transitions.
    let flipped = match decide_bundle(store.pool(), bundle_id, &actor, decision).await {
        Ok(v) => v,
        Err(e) => return server_error(&format!("decide bundle: {e}")),
    };
    if !flipped {
        // Race: another operator slipped in. Surface the winner.
        let after = match get_bundle(store.pool(), bundle_id).await {
            Ok(Some(b)) => b,
            Ok(None) => return server_error("bundle vanished post-decide"),
            Err(e) => return server_error(&format!("reload bundle: {e}")),
        };
        let prior = after.decision.as_deref().unwrap_or("");
        return if prior == decision.as_str() {
            ok_decide(bundle_id, decision.as_str(), 0, after.members.len())
        } else {
            conflict(bundle_id, prior)
        };
    }
    let outcome = match apply_to_members(&pre.members, decision, &actor).await {
        Ok(o) => o,
        Err(e) => return server_error(&format!("apply to members: {e}")),
    };
    ok_decide(
        bundle_id,
        decision.as_str(),
        outcome.decided,
        outcome.skipped,
    )
}

struct MemberOutcome {
    decided: usize,
    skipped: usize,
}

async fn apply_to_members(
    members: &[String],
    decision: BundleDecision,
    actor: &str,
) -> ArgoResult<MemberOutcome> {
    if members.is_empty() {
        return Ok(MemberOutcome {
            decided: 0,
            skipped: 0,
        });
    }
    let (memory, ws) = open_workspace_memory().await?;
    let mut decided = 0_usize;
    let mut skipped = 0_usize;
    for approval_id in members {
        match transition_member(&memory, ws, approval_id, decision, actor).await {
            Ok(true) => decided += 1,
            Ok(false) => skipped += 1,
            Err(_) => skipped += 1,
        }
    }
    Ok(MemberOutcome { decided, skipped })
}

async fn transition_member(
    memory: &MemoryStore,
    workspace: WorkspaceId,
    approval_id: &str,
    decision: BundleDecision,
    actor: &str,
) -> ArgoResult<bool> {
    let Some(located) = find_action(memory, workspace, approval_id).await? else {
        return Ok(false);
    };
    use crate::handlers::worker_actions::ActionState;
    if located.record.state != ActionState::Pending.as_str() {
        return Ok(false);
    }
    if !located.record.requires_approval {
        return Ok(false);
    }
    let result = match decision {
        BundleDecision::Approve => {
            worker_actions_transitions::approve(memory, workspace, approval_id, actor, None)
                .await
                .map(|_| true)
        }
        BundleDecision::Refuse => worker_actions_transitions::cancel(
            memory,
            workspace,
            approval_id,
            actor,
            Some("decision:refuse:bundle"),
        )
        .await
        .map(|_| true),
    };
    result
}

fn bundle_to_json(b: &ApprovalBundle) -> Value {
    json!({
        "bundle_id": b.id,
        "label": b.label,
        "count": b.members.len(),
        "members": b.members,
        "created_at": b.created_at,
        "status": b.status.as_str(),
        "decision": b.decision,
        "decided_at": b.decided_at,
        "decided_by": b.decided_by,
    })
}

fn parse_members(value: Option<&Value>) -> Result<Vec<String>, String> {
    let arr = value
        .and_then(Value::as_array)
        .ok_or_else(|| "`members` must be an array of approval-id strings".to_string())?;
    if arr.is_empty() {
        return Err("`members` must contain at least one approval id".to_string());
    }
    let mut out = Vec::with_capacity(arr.len());
    for v in arr {
        let s = v
            .as_str()
            .ok_or_else(|| "`members[]` entries must be strings".to_string())?;
        if s.trim().is_empty() {
            return Err("`members[]` entries must be non-empty strings".to_string());
        }
        out.push(s.to_string());
    }
    Ok(out)
}

async fn resolve_workspace_tenant(
    store: &SessionStore,
) -> ArgoResult<(TenantId, WorkspaceId)> {
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))?;
    let name = cwd
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("workspace");
    let root = cwd.to_string_lossy();
    store.ensure_workspace(name, &root).await
}

fn parse_limit(tail: &str) -> Result<u32, (&'static str, String)> {
    let Some(raw) = parse_query(tail, "limit") else {
        return Ok(DEFAULT_LIMIT);
    };
    let Ok(n) = raw.parse::<u32>() else {
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

fn ok_decide(
    bundle_id: &str,
    decision: &str,
    decided: usize,
    skipped: usize,
) -> (&'static str, String) {
    (
        "200 OK",
        json!({
            "ok": true,
            "bundle_id": bundle_id,
            "decision": decision,
            "members_decided": decided,
            "members_skipped": skipped,
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
        "bundle_not_found",
        &format!("no bundle with id '{id}'"),
    )
}

fn conflict(id: &str, current: &str) -> (&'static str, String) {
    error_envelope(
        "409 Conflict",
        "already_decided",
        &format!("bundle '{id}' already decided ({current})"),
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
        json!({"error": {"code": code, "message": message}}).to_string(),
    )
}

#[cfg(test)]
pub(super) fn parse_query_for_test(tail: &str, key: &str) -> Option<String> {
    parse_query(tail, key)
}

#[cfg(test)]
pub(super) fn parse_limit_for_test(tail: &str) -> Result<u32, (&'static str, String)> {
    parse_limit(tail)
}

#[cfg(test)]
pub(super) fn parse_members_for_test(value: Option<&Value>) -> Result<Vec<String>, String> {
    parse_members(value)
}

#[cfg(test)]
pub(super) fn bad_request_for_test(msg: &str) -> (&'static str, String) {
    bad_request(msg)
}

#[cfg(test)]
pub(super) fn not_found_for_test(id: &str) -> (&'static str, String) {
    not_found(id)
}

#[cfg(test)]
pub(super) fn conflict_for_test(id: &str, current: &str) -> (&'static str, String) {
    conflict(id, current)
}

#[cfg(test)]
pub(super) fn ok_decide_for_test(
    id: &str,
    decision: &str,
    decided: usize,
    skipped: usize,
) -> (&'static str, String) {
    ok_decide(id, decision, decided, skipped)
}

#[cfg(test)]
#[path = "routes_approval_bundles_tests.rs"]
mod tests;
