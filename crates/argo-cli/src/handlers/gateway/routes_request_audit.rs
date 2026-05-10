//! Per-request audit-chain wrapper (`mvp-gateway-request-audit`).
//!
//! Sits one layer above route dispatch in [`super::routes::dispatch_route`].
//! After every non-skip-list HTTP request, appends one
//! [`argo_audit::AuditKind::HttpRequest`] row capturing method, path
//! (query string stripped — may carry secrets), tenant id, response
//! status code, and wall-clock duration. The resolved principal —
//! same `X-Argo-Principal` parse the bearer-auth gate uses — lands
//! in the `audit_record.principal_id` column via the audit sink.
//!
//! Liveness probes (`/health*`) and Prometheus scrape (`/metrics*`)
//! are skipped via [`SKIP_PATH_PREFIXES`] so the chain doesn't fill
//! with probe noise. Authenticated mutations on every other route
//! are auditable for the operator and the v1 production gateway.
use super::rate_limit;
use super::routes_auth_principal::principal_id_for_audit;
use argo_audit::{AuditKind, AuditSink};
use argo_store::SessionStore;
use std::sync::Arc;
use std::time::Duration;

/// Path prefixes that bypass the per-request audit write. Members
/// are matched via `path.starts_with(prefix)`, so `/health` covers
/// `/health`, `/health/live`, `/health/ready`, `/health/full`, and
/// any future variant. Same shape for `/metrics`.
pub(super) const SKIP_PATH_PREFIXES: &[&str] = &["/health", "/metrics"];

/// `true` if `path` matches any prefix in [`SKIP_PATH_PREFIXES`].
pub(super) fn should_skip(path: &str) -> bool {
    SKIP_PATH_PREFIXES.iter().any(|p| path.starts_with(p))
}

/// Pull `(method, path)` from the raw HTTP request line. Query
/// string is stripped — query parameters may carry secrets and
/// should not land in the audit chain. Returns `None` for a
/// malformed first line so the caller short-circuits cleanly.
pub(super) fn extract_method_and_path(req: &str) -> Option<(&str, &str)> {
    let line = req.split("\r\n").next()?;
    let (method, rest) = line.split_once(' ')?;
    let target = rest.split_once(' ').map(|(t, _)| t).unwrap_or(rest);
    let path = target.split_once('?').map(|(p, _)| p).unwrap_or(target);
    if method.is_empty() || path.is_empty() {
        return None;
    }
    Some((method, path))
}

/// Parse the leading 3-digit status code out of an HTTP status line
/// (`"200 OK"` → `200`). Falls back to `0` when the line is malformed
/// or doesn't start with a parseable status — the audit row still
/// writes so the chain stays complete even on garbage input.
pub(super) fn parse_status_code(status_line: &str) -> u16 {
    status_line
        .split_whitespace()
        .next()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0)
}

/// Record one audit-chain entry for an HTTP request.
///
/// No-ops when the request line can't be parsed, the path matches
/// [`SKIP_PATH_PREFIXES`], or the gateway booted without a session
/// store (e.g. `/health`-only smoke). Audit-write failures log at
/// `warn` and are swallowed so a transient store hiccup never
/// rejects a successfully-served request.
pub(super) async fn record(
    req: &str,
    store: Option<&Arc<SessionStore>>,
    result: &(&'static str, String),
    elapsed: Duration,
) {
    let Some((method, path)) = extract_method_and_path(req) else {
        return;
    };
    if should_skip(path) {
        return;
    }
    let Some(store) = store else {
        return;
    };
    let status = parse_status_code(result.0);
    let duration_ms = u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX);
    let tenant = rate_limit::extract_tenant(req);
    let principal_id = principal_id_for_audit(req);
    let payload = serde_json::json!({
        "method": method,
        "path": path,
        "status": status,
        "duration_ms": duration_ms,
        "tenant_id": tenant,
    });
    let sink = AuditSink::new(Arc::clone(store));
    if let Err(e) = sink
        .write(None, AuditKind::HttpRequest, payload, principal_id)
        .await
    {
        tracing::warn!(
            target: "gateway::request_audit",
            method = %method,
            path = %path,
            error = %e,
            "request audit write failed"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skip_list_covers_health_and_metrics_prefixes() {
        for path in [
            "/health",
            "/health/live",
            "/health/ready",
            "/health/full",
            "/metrics",
            "/metrics/json",
        ] {
            assert!(should_skip(path), "{path} should skip");
        }
    }

    #[test]
    fn skip_list_does_not_match_unrelated_paths() {
        for path in ["/", "/sessions", "/version", "/audit/recent"] {
            assert!(!should_skip(path), "{path} should not skip");
        }
    }

    #[test]
    fn extract_method_and_path_handles_query_post_and_malformed() {
        let get_q = "GET /sessions?tenant=acme&limit=10 HTTP/1.1\r\nHost: x\r\n\r\n";
        assert_eq!(extract_method_and_path(get_q), Some(("GET", "/sessions")));
        let post = "POST /sessions HTTP/1.1\r\nHost: x\r\n\r\n{}";
        assert_eq!(extract_method_and_path(post), Some(("POST", "/sessions")));
        assert_eq!(extract_method_and_path(""), None);
        assert_eq!(extract_method_and_path("\r\n"), None);
    }

    #[test]
    fn parse_status_code_extracts_number_or_zero() {
        assert_eq!(parse_status_code("200 OK"), 200);
        assert_eq!(parse_status_code("404 Not Found"), 404);
        assert_eq!(parse_status_code("500 Internal Server Error"), 500);
        assert_eq!(parse_status_code(""), 0);
        assert_eq!(parse_status_code("not-a-code"), 0);
    }

    #[tokio::test]
    async fn record_writes_one_http_request_row() {
        let store = Arc::new(argo_store::SessionStore::open(":memory:").await.unwrap());
        store.migrate().await.unwrap();
        let req = "GET /sessions HTTP/1.1\r\nHost: x\r\n\r\n";
        let result = ("200 OK", "[]".to_string());
        record(req, Some(&store), &result, Duration::from_millis(7)).await;

        let rows: Vec<(String, String)> =
            sqlx::query_as("SELECT kind, payload_json FROM audit_record")
                .fetch_all(store.pool())
                .await
                .unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].0, "http.request");
        let payload: serde_json::Value = serde_json::from_str(&rows[0].1).unwrap();
        assert_eq!(payload["method"], "GET");
        assert_eq!(payload["path"], "/sessions");
        assert_eq!(payload["status"], 200);
        assert!(payload["duration_ms"].as_u64().is_some());
    }

    #[tokio::test]
    async fn record_writes_audit_on_5xx_response() {
        let store = Arc::new(argo_store::SessionStore::open(":memory:").await.unwrap());
        store.migrate().await.unwrap();
        let req = "POST /sessions HTTP/1.1\r\nHost: x\r\n\r\n{}";
        let result = ("500 Internal Server Error", "{\"error\":\"x\"}".to_string());
        record(req, Some(&store), &result, Duration::from_millis(1)).await;

        let payload: String =
            sqlx::query_scalar("SELECT payload_json FROM audit_record WHERE kind = 'http.request'")
                .fetch_one(store.pool())
                .await
                .unwrap();
        let payload: serde_json::Value = serde_json::from_str(&payload).unwrap();
        assert_eq!(payload["status"], 500);
        assert_eq!(payload["method"], "POST");
    }

    #[tokio::test]
    async fn record_skips_health_path() {
        let store = Arc::new(argo_store::SessionStore::open(":memory:").await.unwrap());
        store.migrate().await.unwrap();
        let req = "GET /health HTTP/1.1\r\nHost: x\r\n\r\n";
        let result = ("200 OK", "{}".to_string());
        record(req, Some(&store), &result, Duration::from_millis(1)).await;

        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM audit_record")
            .fetch_one(store.pool())
            .await
            .unwrap();
        assert_eq!(count, 0);
    }
}
