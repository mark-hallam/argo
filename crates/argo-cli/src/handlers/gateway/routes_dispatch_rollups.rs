//! `/sessions/by-day`, `/messages/by-day`, `/audit/by-day`,
//! `/audit/by-kind` — analytics rollup dispatchers lifted out
//! of `routes.rs` so the main dispatcher stays under the
//! 500-LOC budget. Pure router — every match arm forwards to
//! the corresponding sibling-module handler. Returns
//! `Some(resp)` on a match, `None` otherwise so `routes.rs`
//! falls through to other branches.
//!
//! Domain-grouped sub-dispatchers (sessions / messages / audit /
//! forge) live in
//! [`super::routes_dispatch_rollups_groups`]; `dispatch_rollups`
//! is now a thin fan-out that walks each group in turn.
use argo_store::SessionStore;
use std::sync::Arc;

/// Dispatch the analytics rollup endpoints. Caller should
/// short-circuit on `Some(resp)`.
pub(super) async fn dispatch_rollups(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> Option<(&'static str, String)> {
    use super::routes_dispatch_rollups_groups as g;
    if let Some(resp) = g::dispatch_sessions_rollups(req, store).await {
        return Some(resp);
    }
    if let Some(resp) = g::dispatch_messages_rollups(req, store).await {
        return Some(resp);
    }
    if let Some(resp) = g::dispatch_audit_rollups(req, store).await {
        return Some(resp);
    }
    if let Some(resp) = g::dispatch_forge_rollups(req, store).await {
        return Some(resp);
    }
    None
}
