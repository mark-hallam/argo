//! Dedicated bearer-token gate for the SCIM 2.0 endpoints
//! (mission-scim-provisioning).
//!
//! SCIM auth is intentionally distinct from the operator-facing
//! gateway bearer (`ARGO_GATEWAY_BEARER_TOKEN`). The IdP gets its
//! own credential (`ARGO_SCIM_BEARER_TOKEN`) so that if the IdP's
//! token leaks, the blast radius is provisioning only — not the
//! full gateway surface.
//!
//! Wired by the `routes_scim_users` dispatcher: every SCIM verb
//! (`GET`/`POST`/`PATCH`/`DELETE`) calls [`check_scim_bearer`]
//! before issuing any read or write.
//!
//! Behaviour:
//!
//! - `ARGO_SCIM_BEARER_TOKEN` unset/empty → return 503. SCIM is
//!   opt-in; an operator must provision a token before the
//!   provisioning endpoints become reachable. This avoids a silent
//!   "anyone with network access provisions users" foot-gun.
//! - `ARGO_GATEWAY_AUTH_BYPASS=1` → bypass, for integration tests.
//!   Same escape hatch the operator-facing bearer honors.
//! - Otherwise: require an exact `Authorization: Bearer <token>`
//!   match. Mismatch → 401.

/// Run the SCIM bearer-token check.
///
/// Returns `None` on auth-pass; returns `Some((status, body))` for
/// the dispatcher to short-circuit on auth-fail.
///
/// # Examples
///
/// ```ignore
/// // (Helper is `pub(super)`; example illustrates usage shape only.)
/// // Token unset → SCIM endpoints return 503.
/// std::env::remove_var("ARGO_SCIM_BEARER_TOKEN");
/// assert!(check_scim_bearer("GET /scim/v2/Users HTTP/1.1\r\n\r\n").is_some());
/// ```
pub(super) fn check_scim_bearer(req: &str) -> Option<(&'static str, String)> {
    if std::env::var("ARGO_GATEWAY_AUTH_BYPASS")
        .ok()
        .as_deref()
        == Some("1")
    {
        return None;
    }
    let expected = match argo_config::cred::get("ARGO_SCIM_BEARER_TOKEN") {
        Ok(t) if !t.is_empty() => t,
        _ => {
            return Some((
                "503 Service Unavailable",
                r#"{"schemas":["urn:ietf:params:scim:api:messages:2.0:Error"],"status":"503","detail":"SCIM provisioning disabled: ARGO_SCIM_BEARER_TOKEN is not configured"}"#
                    .to_string(),
            ));
        }
    };
    if presented_bearer(req).as_deref() == Some(expected.as_str()) {
        None
    } else {
        Some((
            "401 Unauthorized",
            r#"{"schemas":["urn:ietf:params:scim:api:messages:2.0:Error"],"status":"401","detail":"missing or invalid SCIM bearer token"}"#
                .to_string(),
        ))
    }
}

/// True when the request path falls under `/scim/v2/`. The gateway
/// dispatcher uses this to short-circuit the operator-facing bearer
/// gate (SCIM has its own credential and its own gate).
pub(super) fn is_scim_path(req: &str) -> bool {
    let head = req.lines().next().unwrap_or("");
    let mut tokens = head.split_whitespace();
    let _verb = tokens.next();
    let path = tokens.next().unwrap_or("");
    let path = path.split('?').next().unwrap_or(path);
    path.starts_with("/scim/v2/")
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
mod tests {
    use super::is_scim_path;

    #[test]
    fn matches_users_collection() {
        assert!(is_scim_path("GET /scim/v2/Users HTTP/1.1\r\n\r\n"));
    }

    #[test]
    fn matches_user_with_id() {
        assert!(is_scim_path("PATCH /scim/v2/Users/abc-123 HTTP/1.1\r\n\r\n"));
    }

    #[test]
    fn matches_with_query() {
        assert!(is_scim_path(
            "GET /scim/v2/Users?filter=userName+eq+%22a%22 HTTP/1.1\r\n\r\n"
        ));
    }

    #[test]
    fn rejects_non_scim_paths() {
        assert!(!is_scim_path("GET /sessions HTTP/1.1\r\n\r\n"));
        assert!(!is_scim_path("GET /scimX/v2/Users HTTP/1.1\r\n\r\n"));
        assert!(!is_scim_path("GET /scim/v1/Users HTTP/1.1\r\n\r\n"));
    }
}

