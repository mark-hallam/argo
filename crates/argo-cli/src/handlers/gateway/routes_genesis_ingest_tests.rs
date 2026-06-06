//! Tests for the onboarding-document ingest route — size caps, parse-failure
//! handling, and request-validation branches.

use super::*;

fn http_request(json_body: &str) -> String {
    format!(
        "POST /api/v1/genesis/ingest-document HTTP/1.1\r\n\
         Content-Type: application/json\r\n\r\n{json_body}"
    )
}

#[test]
fn decode_text_ok_reports_byte_size() {
    let (text, bytes) = decode_one("Hello onboarding world", "text", 0).expect("ok");
    assert_eq!(text, "Hello onboarding world");
    assert_eq!(bytes, "Hello onboarding world".len());
}

#[test]
fn decode_empty_is_rejected() {
    let err = decode_one("   ", "text", 0).unwrap_err();
    assert!(err.to_lowercase().contains("empty"));
}

#[test]
fn decode_bad_base64_pdf_is_rejected_not_panic() {
    // A corrupted PDF upload must produce a friendly per-file reason, never a
    // panic — the remaining files keep ingesting.
    let err = decode_one("!!!not base64!!!", "pdf_base64", 0).unwrap_err();
    assert!(err.to_lowercase().contains("skipped"));
}

#[test]
fn decode_enforces_total_cap() {
    // A small file that would tip the running total over the 200-MB session cap
    // is rejected with a friendly reason (size-cap coverage without allocating
    // hundreds of MB).
    let err = decode_one("a short doc body", "text", MAX_TOTAL_BYTES).unwrap_err();
    assert!(err.to_lowercase().contains("total"));
}

#[tokio::test]
async fn missing_vertical_id_is_400() {
    let req = http_request(r#"{"documents":[{"filename":"x.md","content":"hi"}]}"#);
    let (status, _) = ingest_response(&req, None).await;
    assert_eq!(status, "400 Bad Request");
}

#[tokio::test]
async fn empty_documents_is_400() {
    let req = http_request(r#"{"vertical_id":"genesis-sam","documents":[]}"#);
    let (status, _) = ingest_response(&req, None).await;
    assert_eq!(status, "400 Bad Request");
}

#[tokio::test]
async fn unknown_vertical_is_404_fail_closed() {
    // No teammate by this id exists for the caller's tenant → fail-closed 404,
    // never a silent cross-tenant resolve.
    let req = http_request(
        r#"{"vertical_id":"genesis-does-not-exist-zzzz","documents":[{"filename":"x.md","format":"text","content":"some onboarding body text here"}]}"#,
    );
    let (status, _) = ingest_response(&req, None).await;
    assert_eq!(status, "404 Not Found");
}
