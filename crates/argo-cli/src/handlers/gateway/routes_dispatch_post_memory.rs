//! Memory POST/PUT/DELETE dispatch — split from
//! `routes_dispatch_post.rs`. Covers:
//!   - `POST /memory` — global memory append
//!   - `POST /memory/<id>/{lock,unlock,forget,edit}` — per-id lifecycle
//!   - J10 worker memory facts CRUD under
//!     `/api/v1/workers/<id>/memory/facts[/<fact_id>]`
//!
//! Returns `Some(resp)` on a match, `None` otherwise so the caller
//! falls through.
use super::routes_post_admin::{
    memory_edit_response, memory_forget_response, memory_lock_response, memory_unlock_response,
};
use super::routes_post_writes::append_memory_response;
use argo_store::SessionStore;
use std::sync::Arc;

/// Dispatch all memory-domain write verbs. Returns `None` when no
/// match.
pub(super) async fn dispatch_memory(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> Option<(&'static str, String)> {
    if req.starts_with("POST /memory ") {
        return Some(append_memory_response(req, store).await);
    }
    // POST /memory/<id>/{lock,unlock,forget,edit}
    if let Some(rest) = req.strip_prefix("POST /memory/") {
        if let Some((id, tail)) = rest.split_once('/') {
            if tail.starts_with("lock ") {
                return Some(memory_lock_response(id, req, store).await);
            }
            if tail.starts_with("unlock ") {
                return Some(memory_unlock_response(id, store).await);
            }
            if tail.starts_with("forget ") {
                return Some(memory_forget_response(id, store).await);
            }
            if tail.starts_with("edit ") {
                return Some(memory_edit_response(id, req, store).await);
            }
        }
    }
    // J10 — worker memory facts CRUD.
    if let Some(resp) = dispatch_worker_memory_facts(req, store).await {
        return Some(resp);
    }
    None
}

/// J10 — match `POST /api/v1/workers/<id>/memory/facts`,
/// `PUT /api/v1/workers/<id>/memory/facts/<fact_id>`, and
/// `DELETE /api/v1/workers/<id>/memory/facts/<fact_id>`. Returns
/// `None` when no arm matches so the caller falls through.
async fn dispatch_worker_memory_facts(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> Option<(&'static str, String)> {
    if let Some(rest) = req.strip_prefix("POST /api/v1/workers/") {
        let (head, _) = rest.split_once(' ')?;
        let parts: Vec<&str> = head.split('/').collect();
        // [worker_id, "memory", "facts"] → create
        if parts.len() == 3 && parts[1] == "memory" && parts[2] == "facts" {
            return Some(
                super::routes_worker_memory::fact_create_response(parts[0], req, store).await,
            );
        }
    }
    if let Some(rest) = req.strip_prefix("PUT /api/v1/workers/") {
        let (head, _) = rest.split_once(' ')?;
        let parts: Vec<&str> = head.split('/').collect();
        if parts.len() == 4 && parts[1] == "memory" && parts[2] == "facts" {
            return Some(
                super::routes_worker_memory::fact_update_response(parts[0], parts[3], req, store)
                    .await,
            );
        }
    }
    if let Some(rest) = req.strip_prefix("DELETE /api/v1/workers/") {
        let (head, _) = rest.split_once(' ')?;
        let parts: Vec<&str> = head.split('/').collect();
        if parts.len() == 4 && parts[1] == "memory" && parts[2] == "facts" {
            return Some(
                super::routes_worker_memory::fact_delete_response(parts[0], parts[3], req, store)
                    .await,
            );
        }
    }
    None
}
