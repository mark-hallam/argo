//! Browser-side security headers spliced into every gateway response.
//!
//! Adds `X-Content-Type-Options`, `X-Frame-Options`, `Referrer-Policy`,
//! and `Permissions-Policy` to all responses; adds `Strict-Transport-Security`
//! only when the request was HTTPS-derived (read from `X-Forwarded-Proto`);
//! and adds `Content-Security-Policy` only on `text/html` responses.
//!
//! The middleware-style abstraction the brief envisaged (`axum::middleware::from_fn`)
//! does not fit this gateway — `boot.rs` hand-rolls `format!`-based HTTP/1.1
//! responses on a `TcpStream` rather than running an axum router. Instead,
//! `header_lines()` returns a ready-to-splice header block (each line
//! terminated with `\r\n`) that callers paste between `Content-Length`
//! and the blank-line separator.
//!
//! `ARGO_CSP_REPORT_ONLY=1` swaps the `Content-Security-Policy` header
//! name for `Content-Security-Policy-Report-Only` for staging rollout
//! (browsers report violations without blocking).

const CSP_VALUE: &str = "default-src 'self'; \
script-src 'self'; \
style-src 'self' 'unsafe-inline'; \
img-src 'self' data:; \
connect-src 'self'; \
frame-ancestors 'none'; \
base-uri 'self'; \
form-action 'self'";

const PERMISSIONS_POLICY: &str =
    "geolocation=(), camera=(), microphone=(), payment=()";
const HSTS_VALUE: &str = "max-age=31536000; includeSubDomains";

/// Build the security-header block for a response.
///
/// Returns lines of the form `"Header: Value\r\n"` concatenated. Splice
/// the block between an existing `Content-Length: N\r\n` line and the
/// blank-line separator: the trailing `\r\n` of the last header pairs
/// with the existing `\r\n` to form the `\r\n\r\n` body separator.
///
/// - `content_type` — the response Content-Type (e.g. `"application/json"`).
///   CSP is added only when this starts with `text/html`.
/// - `request_https` — whether the request was HTTPS-derived (caller
///   uses [`request_is_https`] to decide). HSTS is only sent when true.
///
/// # Examples
///
/// ```ignore
/// let block = header_lines("application/json", false);
/// assert!(block.contains("X-Content-Type-Options: nosniff"));
/// assert!(!block.contains("Content-Security-Policy"));
/// assert!(!block.contains("Strict-Transport-Security"));
/// ```
pub(super) fn header_lines(content_type: &str, request_https: bool) -> String {
    let mut out = String::with_capacity(512);
    out.push_str("X-Content-Type-Options: nosniff\r\n");
    out.push_str("X-Frame-Options: DENY\r\n");
    out.push_str("Referrer-Policy: strict-origin-when-cross-origin\r\n");
    out.push_str("Permissions-Policy: ");
    out.push_str(PERMISSIONS_POLICY);
    out.push_str("\r\n");
    if request_https {
        out.push_str("Strict-Transport-Security: ");
        out.push_str(HSTS_VALUE);
        out.push_str("\r\n");
    }
    if content_type.starts_with("text/html") {
        out.push_str(csp_header_name());
        out.push_str(": ");
        out.push_str(CSP_VALUE);
        out.push_str("\r\n");
    }
    out
}

fn csp_header_name() -> &'static str {
    if std::env::var("ARGO_CSP_REPORT_ONLY").ok().as_deref() == Some("1") {
        "Content-Security-Policy-Report-Only"
    } else {
        "Content-Security-Policy"
    }
}

/// True when the request was HTTPS-derived. Reads `X-Forwarded-Proto`
/// (set by load balancers) — there is no other reliable signal in the
/// hand-rolled HTTP/1.1 listener, so absent header → assume HTTP.
///
/// Matching is case-insensitive on both header name and value.
///
/// # Examples
///
/// ```ignore
/// assert!(request_is_https("GET / HTTP/1.1\r\nX-Forwarded-Proto: https\r\n\r\n"));
/// assert!(!request_is_https("GET / HTTP/1.1\r\n\r\n"));
/// ```
pub(super) fn request_is_https(req: &str) -> bool {
    req.lines()
        .find_map(|l| {
            l.split_once(':').and_then(|(k, v)| {
                k.eq_ignore_ascii_case("X-Forwarded-Proto")
                    .then(|| v.trim())
            })
        })
        .is_some_and(|v| v.eq_ignore_ascii_case("https"))
}

#[cfg(test)]
#[path = "security_headers_tests.rs"]
mod tests;
