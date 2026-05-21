//! Audit POST/DELETE dispatch — split from `routes_dispatch_post.rs`.
//! Pure router for `/api/v1/audit/*` write paths.
//! Returns `Some(resp)` on a match, `None` otherwise so the caller
//! falls through.
use argo_store::SessionStore;
use std::sync::Arc;

/// Dispatch audit-domain write verbs (POST + DELETE). Returns
/// `None` when no match.
pub(super) async fn dispatch_audit(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> Option<(&'static str, String)> {
    // J16 — elevated-view request (auto-grants in v1). Bearer auth
    // applied upstream; the handler validates body shape and inserts a
    // row into `elevated_view_grant`.
    if req.starts_with("POST /api/v1/audit/elevated-view-request ") {
        return Some(super::routes_elevated_view::request_response(req, store).await);
    }
    // J16 — revoke a grant. DELETE shares the upstream bearer +
    // principal-header gate; the gateway has no separate DELETE
    // dispatcher in v1.
    if let Some(rest) = req.strip_prefix("DELETE /api/v1/audit/elevated-view-grants/") {
        let id = rest.split_whitespace().next().unwrap_or("");
        if !id.is_empty() {
            return Some(super::routes_elevated_view::revoke_response(id, req, store).await);
        }
    }
    // web-gateway-endpoints-audit-mutations-batch — evidence-bundle
    // export + preview. POST /api/v1/audit/export[/preview].
    if let Some(resp) = super::routes_audit_export::try_dispatch_post(req, store).await {
        return Some(resp);
    }
    // POST /api/v1/audit/<id>/replay — sandboxed audit-row replay.
    // Match BEFORE the bare `POST /api/v1/audit` annotation route so
    // the longer-path match wins.
    if let Some(rest) = req.strip_prefix("POST /api/v1/audit/") {
        let head = rest.split_whitespace().next().unwrap_or("");
        let head_no_q = head.split_once('?').map_or(head, |(p, _)| p);
        if let Some(id) = head_no_q.strip_suffix("/replay") {
            if !id.is_empty() && !id.contains('/') {
                return Some(super::routes_audit_replay::replay_response(id, req, store).await);
            }
        }
    }
    // POST /api/v1/audit — record an out-of-band ("audit-the-audit")
    // annotation row into the chain.
    if req.starts_with("POST /api/v1/audit ") || req.starts_with("POST /api/v1/audit?") {
        return Some(super::routes_audit_record::record_response(req, store).await);
    }
    None
}
