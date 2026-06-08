//! Tests for the SSRF-safe fetcher — the IP-range blocklist and the
//! pre-connect rejection of blocked / non-http(s) URLs.
//!
//! The URL-level tests use **IP-literal hosts** so resolution is local (no DNS,
//! no network): the guard fires deterministically before any socket is opened.

use super::*;

#[test]
fn blocks_ipv4_internal_ranges() {
    for ip in [
        "127.0.0.1",       // loopback
        "10.0.0.1",        // RFC-1918
        "172.16.5.4",      // RFC-1918
        "192.168.1.1",     // RFC-1918
        "169.254.169.254", // link-local cloud metadata endpoint
        "0.0.0.0",         // unspecified
        "0.1.2.3",         // 0.0.0.0/8 — routes to localhost on Linux (SSRF bypass guard)
        "255.255.255.255", // broadcast
        "100.64.0.1",      // CGNAT
        "240.0.0.1",       // reserved / Class E (240.0.0.0/4)
    ] {
        let parsed: IpAddr = ip.parse().expect("parse v4");
        assert!(is_blocked_ip(parsed), "{ip} must be blocked");
    }
}

#[test]
fn allows_public_ipv4() {
    for ip in ["8.8.8.8", "1.1.1.1", "93.184.216.34"] {
        let parsed: IpAddr = ip.parse().expect("parse v4");
        assert!(!is_blocked_ip(parsed), "{ip} must be allowed");
    }
}

#[test]
fn blocks_ipv6_internal_ranges() {
    for ip in [
        "::1",                       // loopback
        "::",                        // unspecified
        "fc00::1",                   // unique-local
        "fe80::1",                   // link-local
        "::ffff:10.0.0.1",           // ipv4-mapped private
        "::ffff:169.254.169.254",    // ipv4-mapped metadata
    ] {
        let parsed: IpAddr = ip.parse().expect("parse v6");
        assert!(is_blocked_ip(parsed), "{ip} must be blocked");
    }
}

#[test]
fn allows_public_ipv6() {
    let parsed: IpAddr = "2606:4700:4700::1111".parse().expect("parse v6");
    assert!(!is_blocked_ip(parsed), "public v6 must be allowed");
}

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
