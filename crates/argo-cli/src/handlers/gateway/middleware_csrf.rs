//! Double-submit CSRF gate for cookie-authenticated mutating requests
//! (Stream `claude-rebase-pr-1835-cookies`).
//!
//! Pattern: the gateway mints two browser cookies on a successful
//! magic-link verify — `__Host-session` (HttpOnly, the JWT) and
//! `__Host-csrf` (NOT HttpOnly, the CSRF token = the JWT `jti`).
//! The SPA reads the csrf value out via `document.cookie` (or via
//! `GET /api/v1/auth/csrf` as a fallback) and echoes it in the
//! `X-CSRF-Token` header on mutating requests. The gateway compares
//! the cookie value to the header value (constant-time) and rejects
//! mismatches with `403`.
//!
//! The gate is intentionally narrow:
//! - **No session cookie** → skip. CLI / bearer-token callers do not
//!   carry the browser session cookie, so CSRF cannot apply.
//! - **Safe method** → skip. Only POST / PATCH / DELETE are gated.
//! - **Anonymous auth endpoints** (`/api/v1/auth/signup`,
//!   `/api/v1/auth/verify`, `/api/v1/auth/csrf`) → skip. The signup +
//!   verify flow predates any cookie.
//!
//! Header-only enforcement is the hidden constraint that protects
//! against a malicious page setting `Cookie: csrf=...` cross-site:
//! the attacker cannot forge an `X-CSRF-Token` header because the
//! browser refuses to set custom headers on simple cross-site form
//! POSTs (CORS preflight requirement).

pub(super) const SESSION_COOKIE: &str = "__Host-session";
pub(super) const CSRF_COOKIE: &str = "__Host-csrf";
pub(super) const CSRF_HEADER: &str = "X-CSRF-Token";

/// Non-`__Host-`-prefixed CSRF cookie name, emitted + honoured ONLY in
/// non-production (dev) mode. The `__Host-` prefix requires the `Secure`
/// attribute and an HTTPS origin, so `__Host-csrf` does not round-trip on
/// a plain `http://localhost` dev origin in some browsers — leaving the
/// double-submit compare with no cookie to read. In dev the verify route
/// additionally mints this plain `csrf` cookie (SameSite=Lax, no Secure)
/// so a local-dev hire still has a readable token. Production never emits
/// or accepts it; the strict `__Host-csrf` + Secure posture is unchanged.
pub(super) const DEV_CSRF_COOKIE: &str = "csrf";

const REJECT_BODY_MISSING: &str =
    r#"{"error":"csrf_missing","reason":"X-CSRF-Token header required for cookie-auth requests"}"#;
const REJECT_BODY_MISMATCH: &str =
    r#"{"error":"csrf_mismatch","reason":"X-CSRF-Token header does not match __Host-csrf cookie"}"#;

/// Returns `Some(403)` when the request fails the double-submit
/// check. Returns `None` for the skip-cases listed in the module
/// docs.
pub(super) fn check_csrf(req: &str) -> Option<(&'static str, String)> {
    // Dev-origin fallback to the plain `csrf` cookie is enabled only
    // when the gateway is NOT in production mode. In production the
    // strict `__Host-csrf` cookie is the only one read, so a forged
    // plain `csrf` cookie can never satisfy the compare.
    check_csrf_impl(
        req,
        !super::routes_auth_production_mode::bind_aware_production_mode(),
    )
}

fn check_csrf_impl(req: &str, allow_dev_cookie: bool) -> Option<(&'static str, String)> {
    if !is_mutating_request(req) || is_anonymous_auth(req) {
        return None;
    }
    let session_present = cookie_value(req, SESSION_COOKIE).is_some();
    // Prefer the strict `__Host-csrf` cookie; fall back to the plain
    // dev `csrf` cookie only when allowed AND the strict one is absent.
    let csrf_cookie = cookie_value(req, CSRF_COOKIE).or_else(|| {
        allow_dev_cookie
            .then(|| cookie_value(req, DEV_CSRF_COOKIE))
            .flatten()
    });
    let Some(csrf_cookie) = csrf_cookie else {
        return if session_present {
            Some(("403 Forbidden", REJECT_BODY_MISSING.to_string()))
        } else {
            None
        };
    };
    if !session_present {
        return None;
    }
    let Some(header) = header_value(req, CSRF_HEADER) else {
        return Some(("403 Forbidden", REJECT_BODY_MISSING.to_string()));
    };
    if !ct_eq(csrf_cookie.as_bytes(), header.as_bytes()) {
        return Some(("403 Forbidden", REJECT_BODY_MISMATCH.to_string()));
    }
    None
}

fn is_mutating_request(req: &str) -> bool {
    req.starts_with("POST ") || req.starts_with("PATCH ") || req.starts_with("DELETE ")
}

fn is_anonymous_auth(req: &str) -> bool {
    for prefix in [
        "POST /api/v1/auth/signup",
        "POST /api/v1/auth/verify",
        "GET /api/v1/auth/csrf",
    ] {
        if req.starts_with(&format!("{prefix} ")) || req.starts_with(&format!("{prefix}?")) {
            return true;
        }
    }
    false
}

pub(super) fn cookie_value(req: &str, name: &str) -> Option<String> {
    let headers = req
        .split_once("\r\n\r\n")
        .map_or(req, |(headers, _body)| headers);
    for line in headers.lines().skip(1) {
        let Some((k, v)) = line.split_once(':') else {
            continue;
        };
        if !k.trim().eq_ignore_ascii_case("Cookie") {
            continue;
        }
        for pair in v.split(';') {
            let pair = pair.trim();
            if let Some((cookie_name, cookie_value)) = pair.split_once('=') {
                if cookie_name.trim() == name {
                    return Some(cookie_value.trim().to_string());
                }
            }
        }
    }
    None
}

fn header_value(req: &str, name: &str) -> Option<String> {
    let headers = req
        .split_once("\r\n\r\n")
        .map_or(req, |(headers, _body)| headers);
    headers.lines().skip(1).find_map(|line| {
        let (k, v) = line.split_once(':')?;
        k.trim()
            .eq_ignore_ascii_case(name)
            .then(|| v.trim().to_string())
    })
}

/// Constant-time byte-slice comparison. Returns `false` immediately
/// on length mismatch (length is not a secret, so this leak is
/// intentional + matches `subtle::ConstantTimeEq` semantics).
fn ct_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff: u8 = 0;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn req(method: &str, path: &str, headers: &[(&str, &str)]) -> String {
        let mut s = format!("{method} {path} HTTP/1.1\r\n");
        for (k, v) in headers {
            s.push_str(&format!("{k}: {v}\r\n"));
        }
        s.push_str("\r\n");
        s
    }

    #[test]
    fn safe_method_passes() {
        assert!(check_csrf(&req("GET", "/sessions", &[])).is_none());
    }

    #[test]
    fn anonymous_auth_passes_without_csrf() {
        assert!(check_csrf(&req("POST", "/api/v1/auth/signup", &[])).is_none());
        assert!(check_csrf(&req("POST", "/api/v1/auth/verify", &[])).is_none());
    }

    #[test]
    fn no_session_cookie_skips_check() {
        assert!(check_csrf(&req("POST", "/api/v1/dispatch", &[])).is_none());
    }

    #[test]
    fn session_without_csrf_header_rejected() {
        let r = req(
            "POST",
            "/api/v1/dispatch",
            &[("Cookie", "__Host-session=abc; __Host-csrf=token-value")],
        );
        let resp = check_csrf(&r).expect("rejected");
        assert_eq!(resp.0, "403 Forbidden");
        assert!(resp.1.contains("csrf_missing"));
    }

    #[test]
    fn session_with_matching_header_passes() {
        let r = req(
            "POST",
            "/api/v1/dispatch",
            &[
                ("Cookie", "__Host-session=abc; __Host-csrf=token-value"),
                ("X-CSRF-Token", "token-value"),
            ],
        );
        assert!(check_csrf(&r).is_none());
    }

    #[test]
    fn session_with_mismatch_rejected() {
        let r = req(
            "POST",
            "/api/v1/dispatch",
            &[
                ("Cookie", "__Host-session=abc; __Host-csrf=token-value"),
                ("X-CSRF-Token", "wrong-token"),
            ],
        );
        let resp = check_csrf(&r).expect("rejected");
        assert_eq!(resp.0, "403 Forbidden");
        assert!(resp.1.contains("csrf_mismatch"));
    }

    #[test]
    fn dev_csrf_cookie_accepted_when_allowed() {
        // http://localhost dev path: only the plain `csrf` cookie
        // round-trips. With dev fallback on, the double-submit compare
        // reads it and passes when the header matches.
        let r = req(
            "POST",
            "/api/v1/workers",
            &[
                ("Cookie", "__Host-session=abc; csrf=devtok"),
                ("X-CSRF-Token", "devtok"),
            ],
        );
        assert!(check_csrf_impl(&r, true).is_none());
    }

    #[test]
    fn dev_csrf_cookie_ignored_in_production() {
        // In production the plain `csrf` cookie is NOT honoured; with
        // no `__Host-csrf` present the gate treats the token as missing.
        let r = req(
            "POST",
            "/api/v1/workers",
            &[
                ("Cookie", "__Host-session=abc; csrf=devtok"),
                ("X-CSRF-Token", "devtok"),
            ],
        );
        let resp = check_csrf_impl(&r, false).expect("rejected");
        assert_eq!(resp.0, "403 Forbidden");
        assert!(resp.1.contains("csrf_missing"));
    }

    #[test]
    fn host_csrf_cookie_preferred_over_dev_cookie() {
        // When both cookies are present the strict `__Host-csrf` wins,
        // so a mismatched plain `csrf` cookie cannot weaken the compare.
        let r = req(
            "POST",
            "/api/v1/workers",
            &[
                (
                    "Cookie",
                    "__Host-session=abc; __Host-csrf=strict; csrf=devtok",
                ),
                ("X-CSRF-Token", "strict"),
            ],
        );
        assert!(check_csrf_impl(&r, true).is_none());
        // Header matching only the dev cookie is rejected — the strict
        // cookie is the one compared.
        let r2 = req(
            "POST",
            "/api/v1/workers",
            &[
                (
                    "Cookie",
                    "__Host-session=abc; __Host-csrf=strict; csrf=devtok",
                ),
                ("X-CSRF-Token", "devtok"),
            ],
        );
        assert!(check_csrf_impl(&r2, true).is_some());
    }

    #[test]
    fn ct_eq_matches_equal_and_rejects_mismatches() {
        assert!(ct_eq(b"abc", b"abc"));
        assert!(!ct_eq(b"abc", b"abd"));
        assert!(!ct_eq(b"abc", b"abcd"));
    }
}
