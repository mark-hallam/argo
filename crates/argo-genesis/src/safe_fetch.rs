//! SSRF-guarded HTTP(S) fetch for Genesis Phase-2 wiki-URL onboarding intake.
//!
//! ## Why this exists (and why [`crate::job_description`]'s `fetch_url` is NOT
//! reused for wiki-URL ingest)
//!
//! The job-description fetcher has only a timeout + a body-size cap. It will
//! happily `GET http://169.254.169.254/…` (the cloud-metadata endpoint),
//! `http://localhost`, or any RFC-1918 private address — a classic
//! **Server-Side Request Forgery** hole: an operator (or anything that can
//! reach the gateway) could make the *server* fetch internal-only URLs and read
//! back the response. Wiki-URL onboarding accepts an operator-supplied URL, so
//! it must go through this guarded path instead.
//!
//! ## What the guard enforces
//!
//! 1. **Scheme allow-list** — only `http` / `https`. `file://`, `ftp://`,
//!    `gopher://`, `data:`, etc. are rejected before any I/O.
//! 2. **Address blocking** — the host is resolved to IP addresses and *every*
//!    resolved address is checked; if any is loopback / private / link-local
//!    (incl. `169.254.169.254`) / unique-local / unspecified / multicast /
//!    broadcast / documentation, the fetch is refused **before connecting**.
//! 3. **Redirect cap + per-hop re-validation** — redirects are followed
//!    manually (reqwest auto-redirect is disabled) so each hop's URL is
//!    re-checked against rules 1 + 2; a redirect to an internal address can't
//!    smuggle past the first check. Capped at [`MAX_REDIRECTS`].
//! 4. **Body cap + timeout** — [`SAFE_FETCH_BODY_CAP_BYTES`] / [`TIMEOUT`].
//!
//! ## Residual risk (documented, not hidden)
//!
//! A TOCTOU DNS-rebind (resolve-good-then-connect-to-bad) is still
//! theoretically possible because reqwest re-resolves the host when it
//! connects. Fully closing that needs a custom connector that pins the
//! validated IP; that is a deliberate follow-up. The pre-connect resolution
//! check blocks the overwhelmingly common static-internal-target attack, which
//! is the acceptance gate this brief calls out.

use argo_types::{ArgoError, ArgoResult};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::time::Duration;

/// Max decoded body size for a fetched wiki page (bytes). Matches the
/// job-description fetcher's cap so the two paths agree on resource limits.
pub const SAFE_FETCH_BODY_CAP_BYTES: usize = 5 * 1024 * 1024;

/// Per-request timeout for the guarded fetch.
const TIMEOUT: Duration = Duration::from_secs(30);

/// Maximum number of redirect hops to follow (each re-validated).
const MAX_REDIRECTS: usize = 5;

/// Fetch `url` as UTF-8 text through the SSRF guard.
///
/// Returns the decoded body on success. Every guard violation maps to
/// [`ArgoError::Schema`] (a caller fault — a bad/blocked URL), while genuine
/// network/transport failures map to [`ArgoError::Provider`].
///
/// # Errors
///
/// - [`ArgoError::Schema`] — non-`http(s)` scheme, unparseable URL, a host that
///   resolves to a blocked address, redirect limit exceeded, over-cap body, or
///   a non-UTF-8 body.
/// - [`ArgoError::Provider`] — DNS resolution failure, transport error, or a
///   non-success HTTP status.
pub async fn safe_fetch_text(url: &str) -> ArgoResult<String> {
    let client = reqwest::Client::builder()
        .timeout(TIMEOUT)
        // We follow redirects ourselves so every hop is re-validated.
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|e| ArgoError::Provider(format!("safe-fetch client: {e}")))?;

    let mut current = url.to_string();
    for _hop in 0..=MAX_REDIRECTS {
        let parsed = validate_url(&current).await?;
        let resp = client
            .get(parsed.clone())
            .send()
            .await
            .map_err(|e| ArgoError::Provider(format!("safe-fetch {current}: {e}")))?;
        let status = resp.status();

        if status.is_redirection() {
            let location = resp
                .headers()
                .get(reqwest::header::LOCATION)
                .and_then(|v| v.to_str().ok())
                .ok_or_else(|| {
                    ArgoError::Schema(format!("safe-fetch {current}: redirect without Location"))
                })?;
            // Resolve relative redirects against the current absolute URL.
            current = parsed
                .join(location)
                .map_err(|e| ArgoError::Schema(format!("safe-fetch bad redirect target: {e}")))?
                .to_string();
            continue;
        }

        if !status.is_success() {
            return Err(ArgoError::Provider(format!(
                "safe-fetch {current}: HTTP {}",
                status.as_u16()
            )));
        }
        return read_capped_body(resp, &current).await;
    }

    Err(ArgoError::Schema(format!(
        "safe-fetch {url}: exceeded {MAX_REDIRECTS} redirects"
    )))
}

/// Parse + scheme-check `url`, resolve its host, and reject if any resolved
/// address is blocked. Returns the parsed [`reqwest::Url`] on success.
async fn validate_url(url: &str) -> ArgoResult<reqwest::Url> {
    let parsed = reqwest::Url::parse(url)
        .map_err(|e| ArgoError::Schema(format!("safe-fetch: unparseable URL '{url}': {e}")))?;

    let scheme = parsed.scheme();
    if scheme != "http" && scheme != "https" {
        return Err(ArgoError::Schema(format!(
            "safe-fetch: refusing non-http(s) scheme '{scheme}'"
        )));
    }

    let raw_host = parsed
        .host_str()
        .ok_or_else(|| ArgoError::Schema(format!("safe-fetch: URL '{url}' has no host")))?;
    // An IPv6 literal host comes back bracketed (`[::1]`); strip the brackets so
    // the numeric address resolves locally (no DNS) instead of failing to parse.
    let host = raw_host
        .strip_prefix('[')
        .and_then(|h| h.strip_suffix(']'))
        .unwrap_or(raw_host)
        .to_string();
    let port = parsed.port_or_known_default().unwrap_or(80);

    // Resolve every address the host maps to and reject if ANY is blocked
    // (conservative: a single internal answer fails the whole fetch).
    let addrs = tokio::net::lookup_host((host.as_str(), port))
        .await
        .map_err(|e| ArgoError::Provider(format!("safe-fetch: cannot resolve '{host}': {e}")))?;
    let mut any = false;
    for addr in addrs {
        any = true;
        if is_blocked_ip(addr.ip()) {
            return Err(ArgoError::Schema(format!(
                "safe-fetch: refusing to fetch private/loopback/metadata address {} (host '{host}')",
                addr.ip()
            )));
        }
    }
    if !any {
        return Err(ArgoError::Provider(format!(
            "safe-fetch: host '{host}' resolved to no addresses"
        )));
    }
    Ok(parsed)
}

/// Read a response body, enforcing [`SAFE_FETCH_BODY_CAP_BYTES`] (both the
/// advertised `Content-Length` and the actual decoded length) and UTF-8.
async fn read_capped_body(resp: reqwest::Response, url: &str) -> ArgoResult<String> {
    if let Some(len) = resp.content_length() {
        if len > SAFE_FETCH_BODY_CAP_BYTES as u64 {
            return Err(ArgoError::Schema(format!(
                "safe-fetch {url}: body {len} bytes exceeds {SAFE_FETCH_BODY_CAP_BYTES}-byte cap"
            )));
        }
    }
    let bytes = resp
        .bytes()
        .await
        .map_err(|e| ArgoError::Provider(format!("safe-fetch body {url}: {e}")))?;
    if bytes.len() > SAFE_FETCH_BODY_CAP_BYTES {
        return Err(ArgoError::Schema(format!(
            "safe-fetch {url}: body {} bytes exceeds {SAFE_FETCH_BODY_CAP_BYTES}-byte cap",
            bytes.len()
        )));
    }
    String::from_utf8(bytes.to_vec())
        .map_err(|e| ArgoError::Schema(format!("safe-fetch {url}: body not utf-8: {e}")))
}

/// `true` when `ip` falls in a range the gateway must never fetch on an
/// operator-supplied URL's behalf (SSRF defence).
#[must_use]
pub(crate) fn is_blocked_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => is_blocked_v4(v4),
        IpAddr::V6(v6) => is_blocked_v6(v6),
    }
}

/// Blocked IPv4 ranges: loopback, RFC-1918 private, link-local (incl. the
/// `169.254.169.254` metadata endpoint), the whole `0.0.0.0/8`
/// "this host"/"this network" block, broadcast, CGNAT, documentation,
/// reserved/Class-E (`240.0.0.0/4`), and multicast.
fn is_blocked_v4(v4: Ipv4Addr) -> bool {
    let [a, b, _, _] = v4.octets();
    v4.is_loopback()
        || v4.is_private()
        || v4.is_link_local()
        // 0.0.0.0/8 — "this host"/"this network". `is_unspecified()` only
        // matches 0.0.0.0, but on Linux any 0.x.x.x routes to localhost, so
        // `http://0.1.2.3/` would otherwise reach loopback-bound services
        // (the gateway binds loopback). Block the whole /8.
        || a == 0
        || v4.is_broadcast()
        || v4.is_multicast()
        || v4.is_documentation()
        // 100.64.0.0/10 — Carrier-Grade NAT shared address space.
        || (a == 100 && (64..=127).contains(&b))
        // 192.0.0.0/24 — IETF protocol assignments.
        || (a == 192 && b == 0 && v4.octets()[2] == 0)
        // 240.0.0.0/4 — reserved / Class E (never a legitimate fetch target).
        || a >= 240
}

/// Blocked IPv6 ranges: loopback, unspecified, multicast, unique-local
/// (`fc00::/7`), link-local (`fe80::/10`), and IPv4-mapped addresses whose
/// embedded v4 is itself blocked.
fn is_blocked_v6(v6: Ipv6Addr) -> bool {
    if v6.is_loopback() || v6.is_unspecified() || v6.is_multicast() {
        return true;
    }
    // IPv4-mapped (::ffff:a.b.c.d) — re-check the embedded v4 so a mapped
    // private address can't slip through.
    if let Some(mapped) = v6.to_ipv4_mapped() {
        return is_blocked_v4(mapped);
    }
    let first = v6.segments()[0];
    // fc00::/7 unique-local.
    let unique_local = (first & 0xfe00) == 0xfc00;
    // fe80::/10 link-local.
    let link_local = (first & 0xffc0) == 0xfe80;
    unique_local || link_local
}

#[cfg(test)]
#[path = "safe_fetch_tests.rs"]
mod tests;
