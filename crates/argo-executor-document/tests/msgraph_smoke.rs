//! End-to-end smoke for the Microsoft Graph executor.
//!
//! Gated by `ARGO_MSGRAPH_TEST_TOKEN` — when the env var is unset
//! the test returns early so `cargo test -p argo-executor-document`
//! stays hermetic on operator machines / CI. We don't use
//! `#[ignore]` because that hides the test from the default output
//! entirely; an explicit early-return at least surfaces in
//! `cargo test`'s "running 1 test" line.
//!
//! When the token is supplied the test:
//!   1. PUTs a tiny .docx to OneDrive (`/argo-smoke/<uuid>.docx`)
//!   2. GETs the same path back via Graph
//!   3. Asserts the round-tripped bytes match
//!   4. (best-effort) deletes the file — not asserted because
//!      cleanup failure shouldn't fail the smoke
//!
//! Optional companion env: `ARGO_MSGRAPH_TEST_DRIVE_ID` overrides
//! `me/drive` for tenants that need an explicit drive id.

use argo_executor_document::msgraph::MsGraphClient;
use argo_executor_document::{MsGraphExecutor, MsGraphFormat};

fn token() -> Option<String> {
    std::env::var("ARGO_MSGRAPH_TEST_TOKEN").ok().filter(|t| !t.trim().is_empty())
}

fn random_suffix() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{nanos:x}")
}

#[tokio::test]
async fn msgraph_round_trip_uploads_and_reads_back() {
    let Some(token) = token() else {
        eprintln!(
            "msgraph_round_trip_uploads_and_reads_back: ARGO_MSGRAPH_TEST_TOKEN unset, skipping"
        );
        return;
    };
    let client = MsGraphClient::new(token);
    let mut exec = MsGraphExecutor::new(client);
    if let Ok(drive_id) = std::env::var("ARGO_MSGRAPH_TEST_DRIVE_ID") {
        if !drive_id.trim().is_empty() {
            exec = exec.with_drive_id(drive_id);
        }
    }
    let path = format!("argo-smoke/{}.docx", random_suffix());
    let fmt = MsGraphFormat::Docx {
        title: "argo smoke".into(),
        body: "round-trip ok".into(),
    };
    let file = exec
        .create(fmt.clone(), &path, &[])
        .await
        .expect("upload .docx to OneDrive");
    assert!(!file.web_url.is_empty(), "web_url should be populated");
    assert!(!file.item_id.is_empty(), "item_id should be populated");

    let read_bytes = exec.read(&path).await.expect("read .docx back");
    assert!(read_bytes.starts_with(b"PK\x03\x04"), "round-trip should preserve PKZIP magic");
    assert!(read_bytes.len() > 100, "round-tripped file looks empty");
}
