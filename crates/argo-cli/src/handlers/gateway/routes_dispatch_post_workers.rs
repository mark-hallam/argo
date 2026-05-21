//! Workers POST dispatch — split from `routes_dispatch_post.rs`.
//! Pure router for `/api/v1/workers` create and the per-worker POST
//! verbs that don't have their own dedicated sibling. Returns
//! `Some(resp)` on a match, `None` otherwise so the caller falls
//! through.
//!
//! Worker sub-resources live in dedicated siblings:
//!   - `/memory/facts` → `routes_dispatch_post_memory`
//!   - `/retros`       → `routes_dispatch_post_retros`
//!   - `/duties`       → `routes_dispatch_post_duties`
//!   - `/trust-grade/recompute` → handled here
//!     (`web-worker-trust-promotion-engine`).
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
    // web-worker-trust-promotion-engine — on-demand trust-grade
    // recompute. Path: `POST /api/v1/workers/<id>/trust-grade/recompute`.
    if let Some(rest) = req.strip_prefix("POST /api/v1/workers/") {
        let (head, _) = rest.split_once(' ')?;
        let parts: Vec<&str> = head.split('/').collect();
        if parts.len() == 3 && parts[1] == "trust-grade" && parts[2] == "recompute" {
            return Some(
                super::routes_worker_trust_grade::recompute_response(parts[0], req, store).await,
            );
        }
    }
    None
}
