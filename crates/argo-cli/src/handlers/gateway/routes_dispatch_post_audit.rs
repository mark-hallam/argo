//! Audit POST/DELETE dispatch — split from `routes_dispatch_post.rs`.
//! Pure router for `/api/v1/audit/elevated-view-*` write paths.
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
    None
}
