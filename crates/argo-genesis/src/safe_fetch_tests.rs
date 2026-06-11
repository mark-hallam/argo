//! Tests for the SSRF-safe fetcher — the pre-connect rejection of blocked /
//! non-http(s) URLs. The IP-range blocklist itself ([`argo_types::is_blocked_ip`])
//! is unit-tested in `argo-types::ssrf`; here we test that the *fetcher* routes
//! every hop through it.
//!
//! The URL-level tests use **IP-literal hosts** so resolution is local (no DNS,
//! no network): the guard fires deterministically before any socket is opened.

use super::*;

#[tokio::test]
async fn rejects_non_http_schemes() {
    for url in [
        "file:///etc/passwd",
        "ftp://example.com/x",
        "gopher://example.com",
    ] {
        let err = safe_fetch_text(url).await.unwrap_err();
        let msg = err.to_string().to_lowercase();
        assert!(
            msg.contains("scheme"),
            "{url} should be rejected for its scheme, got: {msg}"
        );
    }
}

#[tokio::test]
async fn rejects_loopback_and_metadata_before_connect() {
    // IP-literal hosts → resolved locally, so the guard fires with no network.
    for url in [
        "http://127.0.0.1/",
        "http://169.254.169.254/latest/meta-data/",
        "http://10.0.0.1/internal",
        "http://[::1]:8080/",
    ] {
        let err = safe_fetch_text(url).await.unwrap_err();
        let msg = err.to_string().to_lowercase();
        assert!(
            msg.contains("private/loopback/metadata") || msg.contains("refusing"),
            "{url} should be refused pre-connect, got: {msg}"
        );
    }
}

#[tokio::test]
async fn rejects_unparseable_url() {
    let err = safe_fetch_text("not a url at all").await.unwrap_err();
    assert!(err.to_string().to_lowercase().contains("unparseable"));
}
