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

use argo_types::{is_blocked_ip, ArgoError, ArgoResult};
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

// The SSRF address blocklist (`is_blocked_ip`) lives in `argo-types::ssrf` so
// this fetcher and the `web_extract` tool guard share one source of truth and
// can never drift to different blocklists. See [`argo_types::is_blocked_ip`].

#[cfg(test)]
#[path = "safe_fetch_tests.rs"]
mod tests;
