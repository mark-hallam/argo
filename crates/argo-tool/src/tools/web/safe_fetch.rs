//! SSRF guard for `web_extract` (and any web tool that fetches a
//! **model-supplied** URL).
//!
//! `web_extract`'s `url` argument comes straight from the model — i.e. from a
//! (potentially prompt-injected) worker teammate. Without a guard,
//! `client.get(url)` would happily fetch `http://169.254.169.254/…` (cloud-IAM
//! credentials), `http://127.0.0.1:7400/` (the gateway's own API), or any
//! RFC-1918 internal host, and return the body to the model — a classic
//! **Server-Side Request Forgery** hole. cu1d (#2819) grants `NetEgress{**}` to
//! fenced workers that may call `web_extract`, making this hole reachable by the
//! exact least-privilege adversary the capability fence exists to contain.
//!
//! This guard enforces, on the raw model URL **and every redirect hop**:
//! 1. **Scheme allow-list** — only `http` / `https`.
//! 2. **Pre-connect address block** — the host is resolved and every resolved
//!    address checked against [`argo_types::is_blocked_ip`] (the single,
//!    shared SSRF blocklist, also used by `argo-genesis`'s wiki-URL fetcher);
//!    if any is internal the fetch is refused before a socket opens.
//! 3. **Manual redirect re-validation** — reqwest auto-redirect is disabled so
//!    a `302` to an internal host can't smuggle past the first check. Capped at
//!    [`MAX_REDIRECTS`].
//! 4. **Body cap + timeout** — [`BODY_CAP_BYTES`] / [`REQ_TIMEOUT`].
//!
//! Residual risk (documented, not hidden): a TOCTOU DNS-rebind
//! (resolve-good-then-connect-to-bad) is still theoretically possible because
//! reqwest re-resolves on connect. Closing that fully needs a pinned-IP
//! connector — a deliberate follow-up. The pre-connect check blocks the
//! overwhelmingly common static-internal-target attack this brief calls out.

use super::UA;
use std::time::Duration;

/// Per-request timeout for a guarded web fetch.
const REQ_TIMEOUT: Duration = Duration::from_secs(20);

/// Maximum number of redirect hops to follow (each re-validated).
const MAX_REDIRECTS: usize = 5;

/// Cap on the fetched body size (bytes). A malicious internal/external target
/// can't OOM the worker by streaming an unbounded body.
const BODY_CAP_BYTES: usize = 5 * 1024 * 1024;

/// Fetch `url` as HTML text through the SSRF guard.
///
/// Returns the decoded body on success, or a human-readable error string (the
/// web tools' error channel) on any guard violation or transport failure.
pub(super) async fn safe_fetch_html(url: &str) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .connect_timeout(REQ_TIMEOUT)
        .timeout(REQ_TIMEOUT)
        // Follow redirects ourselves so every hop is re-validated.
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|e| format!("reqwest builder: {e}"))?;

    let mut current = url.to_string();
    for _hop in 0..=MAX_REDIRECTS {
        let parsed = validate_url(&current).await?;
        let resp = client
            .get(parsed.clone())
            .header(reqwest::header::USER_AGENT, UA)
            .header(reqwest::header::ACCEPT, "text/html,application/xhtml+xml")
            .send()
            .await
            .map_err(|e| format!("fetch {current}: {e}"))?;
        let status = resp.status();

        if status.is_redirection() {
            let location = resp
                .headers()
                .get(reqwest::header::LOCATION)
                .and_then(|v| v.to_str().ok())
                .ok_or_else(|| format!("redirect without Location for {current}"))?;
            // Resolve relative redirects against the current absolute URL; the
            // next loop iteration re-validates the resolved target.
            current = parsed
                .join(location)
                .map_err(|e| format!("bad redirect target from {current}: {e}"))?
                .to_string();
            continue;
        }

        if !status.is_success() {
            return Err(format!("status {} for {current}", status.as_u16()));
        }
        return read_capped_body(resp, &current).await;
    }

    Err(format!("{url}: exceeded {MAX_REDIRECTS} redirects"))
}

/// Parse + scheme-check `url`, resolve its host, and reject if **any** resolved
/// address is blocked. Returns the parsed [`reqwest::Url`] on success.
async fn validate_url(url: &str) -> Result<reqwest::Url, String> {
    let parsed =
        reqwest::Url::parse(url).map_err(|e| format!("unparseable URL '{url}': {e}"))?;

    let scheme = parsed.scheme();
    if scheme != "http" && scheme != "https" {
        return Err(format!("refusing non-http(s) scheme '{scheme}' for {url}"));
    }

    let raw_host = parsed
        .host_str()
        .ok_or_else(|| format!("URL '{url}' has no host"))?;
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
        .map_err(|e| format!("cannot resolve '{host}': {e}"))?;
    let mut any = false;
    for addr in addrs {
        any = true;
        if argo_types::is_blocked_ip(addr.ip()) {
            return Err(format!(
                "refusing to fetch private/loopback/metadata address {} (host '{host}')",
                addr.ip()
            ));
        }
    }
    if !any {
        return Err(format!("host '{host}' resolved to no addresses"));
    }
    Ok(parsed)
}

/// Read a response body, enforcing [`BODY_CAP_BYTES`] (both the advertised
/// `Content-Length` and the actual decoded length) and UTF-8.
async fn read_capped_body(resp: reqwest::Response, url: &str) -> Result<String, String> {
    if let Some(len) = resp.content_length() {
        if len > BODY_CAP_BYTES as u64 {
            return Err(format!(
                "body {len} bytes exceeds {BODY_CAP_BYTES}-byte cap for {url}"
            ));
        }
    }
    let bytes = resp
        .bytes()
        .await
        .map_err(|e| format!("body {url}: {e}"))?;
    if bytes.len() > BODY_CAP_BYTES {
        return Err(format!(
            "body {} bytes exceeds {BODY_CAP_BYTES}-byte cap for {url}",
            bytes.len()
        ));
    }
    String::from_utf8(bytes.to_vec()).map_err(|e| format!("body {url} not utf-8: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every internal-address literal must be refused **before connect**.
    /// IP-literal hosts resolve locally (no DNS, no network), so the guard
    /// fires deterministically.
    #[tokio::test]
    async fn refuses_internal_addresses_before_connect() {
        for url in [
            "http://169.254.169.254/latest/meta-data/iam/security-credentials/", // cloud IAM
            "http://127.0.0.1:7400/",  // the gateway's own API
            "http://localhost/",       // loopback by name (resolves to 127.0.0.1/::1)
            "http://10.0.0.5/internal", // RFC-1918
            "http://192.168.1.1/",     // RFC-1918
            "http://[::1]:8080/",      // IPv6 loopback
        ] {
            let err = safe_fetch_html(url).await.unwrap_err().to_lowercase();
            assert!(
                err.contains("refusing") || err.contains("private/loopback/metadata"),
                "{url} must be refused, got: {err}"
            );
        }
    }

    #[tokio::test]
    async fn refuses_non_http_schemes() {
        for url in ["file:///etc/passwd", "ftp://example.com/x", "gopher://x"] {
            let err = safe_fetch_html(url).await.unwrap_err().to_lowercase();
            assert!(
                err.contains("scheme"),
                "{url} must be refused for its scheme, got: {err}"
            );
        }
    }

    #[tokio::test]
    async fn refuses_unparseable_url() {
        let err = safe_fetch_html("not a url at all")
            .await
            .unwrap_err()
            .to_lowercase();
        assert!(err.contains("unparseable"), "got: {err}");
    }
}
