//! Mount-point helper for the web dashboard module
//! ([`super::dashboard`]). The dashboard's request matcher is
//! deliberately simple: any `GET /dashboard/...` request is a
//! candidate, and the dashboard module returns `Some((status,
//! body))` when it claims the request or `None` to fall through
//! to the 404 default.
//!
//! Kept separate from `dashboard.rs` so `routes.rs` only learns
//! about a single entry-point and `dashboard.rs` can focus on
//! handlers + layout.
use super::dashboard;
use argo_store::SessionStore;
use std::sync::Arc;

/// Cheap pre-check used by `routes::dispatch_route` before doing
/// the deeper match in [`dashboard::dispatch`]. Mirrors the
/// `is_*`/`*_response` shape used by the other channel/orchestrator
/// route modules in this crate.
pub(super) fn is_dashboard(req: &str) -> bool {
    dashboard::is_dashboard(req)
}

/// Forward a request to the dashboard module. Returns `Some` when
/// the dashboard owns the URL (any of the four tabs, the live-
/// actions partial, or one of the bundled assets); `None` when the
/// request looked like a dashboard URL but matched no handler so
/// the caller can fall through to the 404 default.
pub(super) async fn dispatch(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> Option<(&'static str, String)> {
    dashboard::dispatch(req, store).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_dashboard_passthrough() {
        assert!(is_dashboard("GET /dashboard/ HTTP/1.1\r\n\r\n"));
        assert!(is_dashboard("GET /dashboard/workers HTTP/1.1\r\n\r\n"));
        assert!(!is_dashboard("GET /sessions HTTP/1.1\r\n\r\n"));
        assert!(!is_dashboard("POST /dashboard/workers HTTP/1.1\r\n\r\n"));
    }

    #[tokio::test]
    async fn dispatch_routes_known_path() {
        let res = dispatch("GET /dashboard/workers HTTP/1.1\r\n\r\n", None).await;
        assert!(res.is_some(), "workers tab should be routed");
    }
}
