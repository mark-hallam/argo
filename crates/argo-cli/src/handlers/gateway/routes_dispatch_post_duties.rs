//! Worker duty POST/PATCH/DELETE dispatch — split from
//! `routes_dispatch_post.rs`. Covers the J13 paths under
//! `/api/v1/workers/<id>/duties/...`:
//!   - `POST   …/duties/{classify,commit,preview}`
//!   - `PATCH  …/duties/<duty_id>`
//!   - `DELETE …/duties/<duty_id>`
//!
//! Returns `Some(resp)` on a match, `None` otherwise so the caller
//! falls through.
use argo_store::SessionStore;
use std::sync::Arc;

/// Match the five J13 duty paths under
/// `/api/v1/workers/<id>/duties/...`. Returns `None` when the
/// request doesn't match so the caller falls through.
pub(super) async fn dispatch_duties(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> Option<(&'static str, String)> {
    for verb in ["POST", "PATCH", "DELETE"] {
        let prefix = format!("{verb} /api/v1/workers/");
        let Some(rest) = req.strip_prefix(&prefix) else {
            continue;
        };
        let (head, _) = rest.split_once(' ')?;
        let parts: Vec<&str> = head.split('/').collect();
        // [worker_id, "duties", "classify"|"commit"|"preview"] → POST
        if verb == "POST" && parts.len() == 3 && parts[1] == "duties" {
            let worker_id = parts[0];
            return Some(match parts[2].split('?').next().unwrap_or(parts[2]) {
                "classify" => {
                    super::routes_duties::classify_response(worker_id, req, store).await
                }
                "commit" => super::routes_duties::commit_response(worker_id, req, store).await,
                "preview" => {
                    super::routes_duties::preview_response(worker_id, req, store).await
                }
                _ => return None,
            });
        }
        // [worker_id, "duties", duty_id] → PATCH / DELETE
        if parts.len() == 3 && parts[1] == "duties" {
            let worker_id = parts[0];
            let duty_id = parts[2];
            return Some(match verb {
                "PATCH" => {
                    super::routes_duties::patch_response(worker_id, duty_id, req, store).await
                }
                "DELETE" => {
                    super::routes_duties::delete_response(worker_id, duty_id, req, store).await
                }
                _ => return None,
            });
        }
    }
    None
}
