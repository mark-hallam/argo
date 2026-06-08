//! Tests for the wiki-URL intake extractor — the SSRF guard fires (pre-connect)
//! and bad input is reported "skipped" rather than panicking.
//!
//! IP-literal hosts resolve locally, so the guard rejects them with no network.

use super::*;

#[tokio::test]
async fn empty_url_is_rejected() {
    let err = fetch_wiki("   ", 0).await.unwrap_err();
    assert!(err.to_lowercase().contains("empty"));
}

#[tokio::test]
async fn loopback_is_refused_pre_connect() {
    let err = fetch_wiki("http://127.0.0.1/internal", 0).await.unwrap_err();
    let msg = err.to_lowercase();
    assert!(msg.contains("skipped"), "should be a friendly skip: {msg}");
    assert!(
        msg.contains("private/loopback/metadata") || msg.contains("refusing"),
        "the SSRF guard should have fired: {msg}"
    );
}

#[tokio::test]
async fn cloud_metadata_endpoint_is_refused() {
    let err = fetch_wiki("http://169.254.169.254/latest/meta-data/", 0)
        .await
        .unwrap_err();
    assert!(err.to_lowercase().contains("skipped"));
}

#[tokio::test]
async fn rfc1918_private_host_is_refused() {
    let err = fetch_wiki("http://10.0.0.1/wiki", 0).await.unwrap_err();
    assert!(err.to_lowercase().contains("skipped"));
}

#[tokio::test]
async fn non_http_scheme_is_refused() {
    let err = fetch_wiki("file:///etc/passwd", 0).await.unwrap_err();
    let msg = err.to_lowercase();
    assert!(msg.contains("skipped") && msg.contains("scheme"), "{msg}");
}
