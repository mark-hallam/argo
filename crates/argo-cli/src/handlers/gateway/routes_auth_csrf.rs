//! `GET /api/v1/auth/csrf` — token-fetch surface for the SPA's
//! double-submit CSRF helper
//! (Stream `claude-rebase-pr-1835-cookies`).
//!
//! The `__Host-csrf` cookie value is HTTPOnly-bit-clear so JavaScript
//! can read it directly. This endpoint is the same-origin
//! alternative for SPA paths that want to fetch the token via XHR
//! rather than `document.cookie` parsing — it simply mirrors the
//! cookie value back as JSON. A request without the cookie returns
//! `401`.

use super::middleware_csrf::{cookie_value, CSRF_COOKIE, DEV_CSRF_COOKIE};

pub(super) async fn dispatch(req: &str) -> Option<(&'static str, String)> {
    if !(req.starts_with("GET /api/v1/auth/csrf ") || req.starts_with("GET /api/v1/auth/csrf?")) {
        return None;
    }
    Some(token_response(req))
}

fn token_response(req: &str) -> (&'static str, String) {
    // Dev-origin fallback to the plain `csrf` cookie matches the gate in
    // `middleware_csrf`: enabled only outside production so the SPA's XHR
    // fallback can read a token on a plain `http://localhost` origin where
    // `__Host-csrf` does not round-trip. Production mirrors `__Host-csrf`
    // only.
    token_response_impl(
        req,
        !super::routes_auth_production_mode::bind_aware_production_mode(),
    )
}

fn token_response_impl(req: &str, allow_dev_cookie: bool) -> (&'static str, String) {
    let token = cookie_value(req, CSRF_COOKIE).or_else(|| {
        allow_dev_cookie
            .then(|| cookie_value(req, DEV_CSRF_COOKIE))
            .flatten()
    });
    match token {
        Some(token) => ("200 OK", serde_json::json!({ "token": token }).to_string()),
        None => (
            "401 Unauthorized",
            r#"{"error":"no_csrf_cookie"}"#.to_string(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn req_with_cookie(cookie: Option<&str>) -> String {
        let mut s = String::from("GET /api/v1/auth/csrf HTTP/1.1\r\n");
        if let Some(c) = cookie {
            s.push_str(&format!("Cookie: {c}\r\n"));
        }
        s.push_str("\r\n");
        s
    }

    #[tokio::test]
    async fn returns_token_when_cookie_present() {
        let r = req_with_cookie(Some("__Host-csrf=abc123"));
        let (status, body) = dispatch(&r).await.expect("dispatched");
        assert_eq!(status, "200 OK");
        assert!(body.contains("abc123"));
    }

    #[tokio::test]
    async fn unauthorized_without_cookie() {
        let r = req_with_cookie(None);
        let (status, _body) = dispatch(&r).await.expect("dispatched");
        assert_eq!(status, "401 Unauthorized");
    }

    #[tokio::test]
    async fn other_paths_skipped() {
        assert!(dispatch("GET /version HTTP/1.1\r\n\r\n").await.is_none());
    }

    #[test]
    fn mirrors_dev_csrf_cookie_when_host_absent() {
        let r = req_with_cookie(Some("csrf=devtok"));
        let (status, body) = token_response_impl(&r, true);
        assert_eq!(status, "200 OK");
        assert!(body.contains("devtok"));
    }

    #[test]
    fn does_not_mirror_dev_cookie_in_production() {
        let r = req_with_cookie(Some("csrf=devtok"));
        let (status, _body) = token_response_impl(&r, false);
        assert_eq!(status, "401 Unauthorized");
    }

    #[test]
    fn host_cookie_preferred_over_dev_cookie_in_mirror() {
        let r = req_with_cookie(Some("__Host-csrf=strict; csrf=devtok"));
        let (status, body) = token_response_impl(&r, true);
        assert_eq!(status, "200 OK");
        assert!(body.contains("strict"));
    }
}
