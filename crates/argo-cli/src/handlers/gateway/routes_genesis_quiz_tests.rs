//! Tests for the Genesis sandbox-quiz route — request validation and
//! fail-closed cross-tenant denial. Retrieval honesty + the memory-edit
//! round-trip are proven function-side in `argo_genesis::sandbox_session`.

use super::*;

fn http_request(json_body: &str) -> String {
    format!(
        "POST /api/v1/genesis/quiz HTTP/1.1\r\n\
         Content-Type: application/json\r\n\r\n{json_body}"
    )
}

#[tokio::test]
async fn missing_vertical_id_is_400() {
    let req = http_request(r#"{"question":"what is the refund policy?"}"#);
    let (status, _) = quiz_response(&req, None).await;
    assert_eq!(status, "400 Bad Request");
}

#[tokio::test]
async fn unknown_vertical_is_404_fail_closed() {
    // No teammate by this id exists for the caller's tenant → fail-closed 404,
    // never a silent cross-tenant resolve into another workspace's knowledge.
    let req = http_request(
        r#"{"vertical_id":"genesis-does-not-exist-zzzz","question":"what is the refund policy?"}"#,
    );
    let (status, _) = quiz_response(&req, None).await;
    assert_eq!(status, "404 Not Found");
}

#[tokio::test]
async fn malformed_body_is_400() {
    let req = http_request("not json at all");
    let (status, _) = quiz_response(&req, None).await;
    assert_eq!(status, "400 Bad Request");
}
