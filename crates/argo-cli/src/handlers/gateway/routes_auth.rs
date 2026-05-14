//! Shared bearer-token gate for gateway dispatch.
//!
//! `check_bearer_auth` is wired by `routes_dispatch_post::dispatch_post`
//! (every POST) and `routes::dispatch_route` (every non-allowlist GET).
//! `is_public_get` carries the curated read-only allowlist that stays
//! anonymous so liveness probes, load balancers, and catalog-discovery
//! flows keep working without an auth token.
//!
//! Mirrors `ARGO_GATEWAY_BEARER_TOKEN` precedent landed by #946.

/// Bearer-token gate applied before dispatch.
///
/// Returns `None` (auth passes) when any of:
/// - `ARGO_GATEWAY_AUTH_BYPASS=1` is set (integration-test escape hatch),
/// - `ARGO_GATEWAY_BEARER_TOKEN` is unset/empty (dev default), or
/// - the request carries a matching `Authorization: Bearer <token>` header.
///
/// Returns `Some(401, body)` otherwise. Body is a small JSON envelope so
/// CLI clients can parse a stable error shape.
///
/// # Examples
///
/// ```ignore
/// // (Helper is `pub(super)`; example shown for documentation only.)
/// // No token set → auth passes for every caller.
/// std::env::remove_var("ARGO_GATEWAY_BEARER_TOKEN");
/// assert!(check_bearer_auth("GET /sessions HTTP/1.1\r\n\r\n").is_none());
/// ```
pub(super) fn check_bearer_auth(req: &str) -> Option<(&'static str, String)> {
    if std::env::var("ARGO_GATEWAY_AUTH_BYPASS")
        .ok()
        .as_deref()
        == Some("1")
    {
        return None;
    }
    let expected = match argo_config::cred::get("ARGO_GATEWAY_BEARER_TOKEN") {
        Ok(t) if !t.is_empty() => t,
        _ => return None,
    };
    if presented_bearer(req).as_deref() == Some(expected.as_str()) {
        None
    } else {
        Some((
            "401 Unauthorized",
            r#"{"error":"unauthorized","reason":"missing or invalid bearer token"}"#
                .to_string(),
        ))
    }
}

/// True for the curated read-only allowlist that bypasses bearer auth
/// even when `ARGO_GATEWAY_BEARER_TOKEN` is configured.
///
/// Members:
/// - `GET /` — root index
/// - `GET /health`, `/health/live`, `/health/ready`, `/health/full`
/// - `GET /livez`, `/readyz` — k8s-convention probes (mission-health-check-endpoint)
/// - `GET /version`
/// - `GET /routes`
///
/// Anything else (including `/doctor`, `/db/*`, `/audit/*`, `/workspace/*`,
/// `/search*`, and every per-resource GET) requires auth when configured.
///
/// # Examples
///
/// ```ignore
/// assert!(is_public_get("GET /health/ready HTTP/1.1\r\n\r\n"));
/// assert!(!is_public_get("GET /sessions HTTP/1.1\r\n\r\n"));
/// ```
pub(super) fn is_public_get(req: &str) -> bool {
    // We only allow GET; POST always goes through `dispatch_post`'s gate.
    if !req.starts_with("GET ") {
        return false;
    }
    // Strip "GET " then peel off whichever terminator follows the path
    // (a trailing space marks "no query", '?' marks the start of one).
    let rest = req.strip_prefix("GET ").unwrap_or(req);
    let path_end = rest.find([' ', '?']).unwrap_or(rest.len());
    let path = &rest[..path_end];
    let exact = matches!(
        path,
        "/" | "/health"
            | "/health/live"
            | "/health/ready"
            | "/health/full"
            | "/livez"
            | "/readyz"
            | "/version"
            | "/routes"
            | "/openmetrics"
    );
    // SCIM 2.0 endpoints (`mission-scim-provisioning`) carry their own
    // bearer credential (`ARGO_SCIM_BEARER_TOKEN`); the dedicated
    // `scim_auth` gate inside the SCIM dispatcher enforces it. Treat
    // the path as "public" here so the operator-facing gate skips it.
    exact || path.starts_with("/scim/v2/")
}

fn presented_bearer(req: &str) -> Option<String> {
    req.lines()
        .find_map(|l| {
            l.split_once(':').and_then(|(k, v)| {
                k.eq_ignore_ascii_case("Authorization").then(|| v.trim())
            })
        })
        .and_then(|h| h.strip_prefix("Bearer "))
        .map(str::to_string)
}

#[cfg(test)]
#[path = "routes_auth_tests.rs"]
mod tests;
