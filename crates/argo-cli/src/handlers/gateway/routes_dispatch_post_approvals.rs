//! JSON approvals API POST dispatch — split from
//! `routes_dispatch_post.rs`. Pure router for the J1 per-id
//! `decide` and `undo` write paths. The list GET arm is wired in
//! `routes_dispatch_get`; the HTML `/inbox` surface in
//! `routes_channels_webinbox` is preserved separately.

/// Dispatch `/api/v1/approvals/<id>/{decide,undo}` POSTs. Returns
/// `None` when no match.
pub(super) async fn dispatch_approvals(req: &str) -> Option<(&'static str, String)> {
    let rest = req.strip_prefix("POST /api/v1/approvals/")?;
    let (id, tail) = rest.split_once('/')?;
    if tail.starts_with("decide ") || tail.starts_with("decide?") {
        return Some(super::routes_approvals_json::decide_response(id, req).await);
    }
    if tail.starts_with("undo ") || tail.starts_with("undo?") {
        return Some(super::routes_approvals_json::undo_response(id, req).await);
    }
    None
}
