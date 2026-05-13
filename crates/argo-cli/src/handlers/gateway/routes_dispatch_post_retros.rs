//! Worker retrospective POST/PATCH dispatch — split from
//! `routes_dispatch_post.rs`. Covers the
//! `mission-retro-review-ui` write paths:
//!   - `POST  /api/v1/workers/<id>/retros`                  → create
//!   - `PATCH /api/v1/workers/<id>/retros/<retro_id>`        → update
//!   - `POST  /api/v1/workers/<id>/retros/<retro_id>/finalize`
//!
//! Returns `Some(resp)` on a match, `None` otherwise so the caller
//! falls through.
use argo_store::SessionStore;
use std::sync::Arc;

/// Match the three retro write paths anchored under
/// `/api/v1/workers/<id>/retros`. Returns `None` when the request
/// doesn't match so the caller falls through.
pub(super) async fn dispatch_retros(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> Option<(&'static str, String)> {
    if let Some(rest) = req.strip_prefix("POST /api/v1/workers/") {
        let (head, _) = rest.split_once(' ')?;
        let parts: Vec<&str> = head.split('/').collect();
        // [worker_id, "retros"] → create
        if parts.len() == 2 && parts[1] == "retros" {
            return Some(super::routes_retros::create_response(parts[0], req, store).await);
        }
        // [worker_id, "retros", retro_id, "finalize"] → finalize
        if parts.len() == 4 && parts[1] == "retros" && parts[3] == "finalize" {
            return Some(
                super::routes_retros::finalize_response(parts[0], parts[2], req, store).await,
            );
        }
    }
    if let Some(rest) = req.strip_prefix("PATCH /api/v1/workers/") {
        let (head, _) = rest.split_once(' ')?;
        let parts: Vec<&str> = head.split('/').collect();
        if parts.len() == 3 && parts[1] == "retros" {
            return Some(
                super::routes_retros::update_response(parts[0], parts[2], req, store).await,
            );
        }
    }
    None
}
