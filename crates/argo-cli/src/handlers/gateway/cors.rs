//! CORS preflight handler + per-response header enhancement for
//! `/api/v1/*` browser callers (mission-gateway-cors-preflight).
//!
//! The gateway's dispatcher returns `(status_line: &'static str,
//! body: String)`. The on-wire formatter splices any extra `\r\n`-
//! separated headers found in `status_line` between the HTTP/1.1
//! status line and the body (see
//! `boot_router::format_response`). This module pre-builds the
//! CORS-enhanced variant of every `(status_code, allowed_origin)`
//! pair at gateway boot, `Box::leak`-ing each formatted string so
//! lookups at request time return `&'static str` with zero per-
//! request allocation.
//!
//! Origins not in the configured allowlist receive their original
//! status line unchanged — the browser then refuses the response,
//! which is the intentional fail-closed behaviour
//! (`Access-Control-Allow-Origin: *` + `Allow-Credentials: true`
//! is forbidden by the CORS spec).

use argo_config::GatewayCorsConfig;
use std::collections::HashMap;
use std::sync::OnceLock;

/// Methods advertised to browsers in preflight responses. Superset
/// of every verb the `/api/v1/*` dispatcher accepts today; extra
/// verbs in the list are harmless (browsers only act on the ones
/// they actually issue).
const ALLOWED_METHODS: &str = "GET, POST, PUT, PATCH, DELETE, OPTIONS";

/// Request headers the SPA is permitted to send.
/// `X-Argo-Principal` + `X-Idempotency-Key` are SKYCOT-specific;
/// `X-Argo-Tenant-Id` is the existing dev/test tenant key the
/// wizard uses; `X-CSRF-Token` is the double-submit value the
/// `web/lib/api/client.ts` request wrapper stamps on every mutating
/// request when a `__Host-csrf` cookie is present. The magic-link
/// and demo-mint cookie pipeline mints both cookies together, so
/// without `X-CSRF-Token` in this allow-list the browser fail-closes
/// every signed-in SPA POST to the gateway from a different origin
/// even when the preflight succeeds. Adding more here is harmless.
const ALLOWED_HEADERS: &str =
    "Content-Type, Authorization, X-Argo-Principal, X-Idempotency-Key, X-Argo-Tenant-Id, X-CSRF-Token";

/// Preflight cache lifetime advertised to browsers (24h). Browsers
/// cap this at their own ceilings (Chromium 2h, Firefox 24h), but
/// requesting 24h means the browser uses its own ceiling rather
/// than a shorter default.
const MAX_AGE: &str = "86400";

/// Status lines the `/api/v1/*` dispatcher emits. Each is pre-
/// formatted with full CORS headers for every allowed origin at
/// install time. Lines emitted by other code paths (e.g. the wizard
/// in `routes_wizard.rs`, which embeds its own `with_cors` headers
/// into the status line) miss the registry lookup and pass through
/// unchanged — no double-stamping.
const CORS_STATUSES: &[&str] = &[
    "200 OK",
    "201 Created",
    "202 Accepted",
    "204 No Content",
    "400 Bad Request",
    "401 Unauthorized",
    "403 Forbidden",
    "404 Not Found",
    "405 Method Not Allowed",
    "408 Request Timeout",
    "409 Conflict",
    "410 Gone",
    "412 Precondition Failed",
    "422 Unprocessable Entity",
    "429 Too Many Requests",
    "500 Internal Server Error",
    "502 Bad Gateway",
    "503 Service Unavailable",
];

/// Pre-built CORS-enhanced status lines keyed by `(base_status,
/// origin)`. Populated by [`install`], immutable thereafter.
pub(super) struct CorsRegistry {
    table: HashMap<(&'static str, String), &'static str>,
}

impl CorsRegistry {
    pub(super) fn build(allowed_origins: &[String]) -> Self {
        let mut table = HashMap::new();
        for origin in allowed_origins {
            for &status in CORS_STATUSES {
                let enhanced = format!(
                    "{status}\r\n\
                     Access-Control-Allow-Origin: {origin}\r\n\
                     Access-Control-Allow-Methods: {ALLOWED_METHODS}\r\n\
                     Access-Control-Allow-Headers: {ALLOWED_HEADERS}\r\n\
                     Access-Control-Allow-Credentials: true\r\n\
                     Access-Control-Max-Age: {MAX_AGE}\r\n\
                     Vary: Origin"
                );
                let leaked: &'static str = Box::leak(enhanced.into_boxed_str());
                table.insert((status, origin.clone()), leaked);
            }
        }
        Self { table }
    }

    fn lookup(&self, status: &'static str, origin: &str) -> Option<&'static str> {
        self.table.get(&(status, origin.to_string())).copied()
    }
}

static REGISTRY: OnceLock<CorsRegistry> = OnceLock::new();

/// Initialise the global CORS registry from gateway config.
/// Idempotent — only the first caller actually populates state,
/// matching the pattern used by `rate_limit::init_with_config`. The
/// allowlist comes from `[gateway.cors].allowed_origins`; with an
/// empty list, no Origin is ever allowlisted and every request
/// receives its base status line unchanged.
pub(super) fn install(cfg: &GatewayCorsConfig) {
    let _ = REGISTRY.set(CorsRegistry::build(&cfg.allowed_origins));
}

/// Extract the `Origin` request header value, case-insensitively.
/// Returns `None` when the header is absent or empty — typical for
/// CLI / curl / server-to-server callers, which then receive their
/// base status line unchanged (no CORS headers, no preflight reply
/// required).
pub(super) fn extract_origin(req: &str) -> Option<&str> {
    let headers = req
        .split_once("\r\n\r\n")
        .map_or(req, |(headers, _body)| headers);
    headers
        .lines()
        .skip(1)
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.trim()
                .eq_ignore_ascii_case("Origin")
                .then(|| value.trim())
        })
        .filter(|s| !s.is_empty())
}

/// True when the request path is under `/api/v1`. Matches both
/// `/api/v1/foo` and a bare `/api/v1` (with or without a query
/// string).
pub(super) fn is_api_v1(req: &str) -> bool {
    let line = req.lines().next().unwrap_or("");
    let path = line.split_whitespace().nth(1).unwrap_or("");
    path == "/api/v1"
        || path.starts_with("/api/v1/")
        || path.starts_with("/api/v1?")
}

/// True when the request is an OPTIONS preflight against
/// `/api/v1/*`.
pub(super) fn is_preflight(req: &str) -> bool {
    req.starts_with("OPTIONS ") && is_api_v1(req)
}

/// Build the 204 preflight response for `/api/v1/*` OPTIONS
/// requests. Always returns `204 No Content`; only the headers
/// differ. If the request's `Origin` is not allowlisted (or no
/// registry is installed), the response carries no
/// `Access-Control-Allow-*` headers — browsers refuse the actual
/// request as a result, the intentional fail-closed behaviour.
pub(super) fn preflight_response(req: &str) -> (&'static str, String) {
    (enhance_status(req, "204 No Content"), String::new())
}

/// If the request's `Origin` is allowlisted and the registry has a
/// CORS-enhanced variant of `base_status`, return it. Otherwise
/// return `base_status` unchanged. Status lines already carrying
/// `\r\n`-separated headers (e.g. those embedded by
/// `routes_wizard::with_cors`) miss the registry lookup and pass
/// through — no double-stamping.
pub(super) fn enhance_status(req: &str, base_status: &'static str) -> &'static str {
    let Some(registry) = REGISTRY.get() else {
        return base_status;
    };
    let Some(origin) = extract_origin(req) else {
        return base_status;
    };
    registry.lookup(base_status, origin).unwrap_or(base_status)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_origin_finds_header_case_insensitive() {
        let req = "GET /api/v1/health HTTP/1.1\r\nHost: localhost\r\norigin: http://localhost:3000\r\n\r\n";
        assert_eq!(extract_origin(req), Some("http://localhost:3000"));
    }

    #[test]
    fn extract_origin_returns_none_when_absent() {
        let req = "GET /api/v1/health HTTP/1.1\r\nHost: localhost\r\n\r\n";
        assert_eq!(extract_origin(req), None);
    }

    #[test]
    fn extract_origin_returns_none_when_empty() {
        let req = "GET /api/v1/health HTTP/1.1\r\nHost: localhost\r\nOrigin:   \r\n\r\n";
        assert_eq!(extract_origin(req), None);
    }

    #[test]
    fn is_api_v1_matches_prefixed_paths() {
        assert!(is_api_v1("GET /api/v1/auth/signup HTTP/1.1\r\n\r\n"));
        assert!(is_api_v1("POST /api/v1?tenant=x HTTP/1.1\r\n\r\n"));
        assert!(is_api_v1("OPTIONS /api/v1 HTTP/1.1\r\n\r\n"));
        assert!(!is_api_v1("GET /sessions HTTP/1.1\r\n\r\n"));
        assert!(!is_api_v1("GET /api/v2/health HTTP/1.1\r\n\r\n"));
        assert!(!is_api_v1("GET /api/v10/health HTTP/1.1\r\n\r\n"));
    }

    #[test]
    fn is_preflight_requires_options_and_api_v1() {
        assert!(is_preflight(
            "OPTIONS /api/v1/auth/signup HTTP/1.1\r\n\r\n"
        ));
        assert!(!is_preflight("OPTIONS / HTTP/1.1\r\n\r\n"));
        assert!(!is_preflight(
            "POST /api/v1/auth/signup HTTP/1.1\r\n\r\n"
        ));
    }

    #[test]
    fn registry_returns_cors_status_for_allowlisted_origin() {
        let registry = CorsRegistry::build(&["http://localhost:3000".to_string()]);
        let enhanced = registry
            .lookup("200 OK", "http://localhost:3000")
            .expect("registry has 200 OK / localhost:3000");
        assert!(enhanced.starts_with("200 OK\r\n"));
        assert!(enhanced.contains("Access-Control-Allow-Origin: http://localhost:3000"));
        assert!(enhanced.contains("Access-Control-Allow-Credentials: true"));
        assert!(enhanced.contains("Access-Control-Allow-Methods: GET, POST"));
        assert!(enhanced.contains("Access-Control-Max-Age: 86400"));
        assert!(enhanced.contains("Vary: Origin"));
    }

    #[test]
    fn registry_misses_for_non_allowlisted_origin() {
        let registry = CorsRegistry::build(&["http://localhost:3000".to_string()]);
        assert!(registry
            .lookup("200 OK", "https://evil.example.com")
            .is_none());
    }

    #[test]
    fn registry_covers_every_status_for_each_origin() {
        let registry = CorsRegistry::build(&[
            "http://localhost:3000".to_string(),
            "https://app.skycot.com".to_string(),
        ]);
        for origin in ["http://localhost:3000", "https://app.skycot.com"] {
            for status in CORS_STATUSES {
                let enhanced = registry
                    .lookup(status, origin)
                    .expect("every status/origin pair pre-built");
                assert!(enhanced.starts_with(status));
                assert!(enhanced.contains(&format!(
                    "Access-Control-Allow-Origin: {origin}"
                )));
            }
        }
    }

    #[test]
    fn enhance_status_falls_through_when_registry_not_installed() {
        // The global REGISTRY may already be set by a parallel test;
        // either way, an unmatched origin returns the base status.
        let req = "GET /api/v1/health HTTP/1.1\r\nOrigin: https://nowhere.example\r\n\r\n";
        assert_eq!(enhance_status(req, "200 OK"), "200 OK");
    }
}
