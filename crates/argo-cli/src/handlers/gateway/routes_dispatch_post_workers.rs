//! Workers POST dispatch — split from `routes_dispatch_post.rs`.
//! Pure router for `/api/v1/workers` create. Returns `Some(resp)`
//! on a match, `None` otherwise so the caller falls through.
//!
//! Worker sub-resources live in dedicated siblings:
//!   - `/memory/facts` → `routes_dispatch_post_memory`
//!   - `/retros`       → `routes_dispatch_post_retros`
//!   - `/duties`       → `routes_dispatch_post_duties`
use argo_store::SessionStore;
use std::sync::Arc;

/// Dispatch worker-collection POSTs. Returns `None` when no match.
pub(super) async fn dispatch_workers(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> Option<(&'static str, String)> {
    // J5 — hire-commit endpoint. Bearer auth applied upstream; body
    // shape is `{worker_draft, problem, answers}` per the brief.
    if req.starts_with("POST /api/v1/workers ") {
        return Some(super::routes_workers_hire::hire_response(req, store).await);
    }
    None
}
