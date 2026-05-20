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

use super::middleware_csrf::{cookie_value, CSRF_COOKIE};

pub(super) async fn dispatch(req: &str) -> Option<(&'static str, String)> {
    if !(req.starts_with("GET /api/v1/auth/csrf ") || req.starts_with("GET /api/v1/auth/csrf?")) {
        return None;
    }
    Some(token_response(req))
}

fn token_response(req: &str) -> (&'static str, String) {
    match cookie_value(req, CSRF_COOKIE) {
        Some(token) => (
            "200 OK",
            serde_json::json!({ "token": token }).to_string(),
        ),
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
}
