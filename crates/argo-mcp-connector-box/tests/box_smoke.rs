//! End-to-end smoke test against the real Box.com `api.box.com` /
//! `upload.box.com` surface.
//!
//! Gated by `ARGO_BOX_TEST_TOKEN` and `ARGO_BOX_TEST_FOLDER_ID`; without
//! either, the test is a no-op so the suite stays safe on CI without
//! credentials. To run locally:
//!
//! ```bash
//! export ARGO_BOX_TEST_TOKEN=<box oauth2 access token>
//! export ARGO_BOX_TEST_FOLDER_ID=<numeric folder id with write access>
//! cargo test -p argo-mcp-connector-box --test box_smoke -- --ignored
//! ```
//!
//! The test uploads a 1 KiB payload under `ARGO_BOX_TEST_FOLDER_ID`,
//! downloads it back, asserts byte-identical round-trip, then enables a
//! `company`-level shared link. The uploaded file is intentionally left
//! behind (no `delete_file` on the connector v1 surface — the brief excludes
//! it). Run periodically against a scratch folder.

use argo_mcp_connector_box::{BoxConnector, SharedLinkAccess};

const TOKEN_ENV: &str = "ARGO_BOX_TEST_TOKEN";
const FOLDER_ENV: &str = "ARGO_BOX_TEST_FOLDER_ID";

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
#[ignore = "requires ARGO_BOX_TEST_TOKEN + ARGO_BOX_TEST_FOLDER_ID"]
async fn round_trip_1kb_payload_against_live_box() {
    let token = match std::env::var(TOKEN_ENV) {
        Ok(v) if !v.trim().is_empty() => v,
        _ => return,
    };
    let folder_id = match std::env::var(FOLDER_ENV) {
        Ok(v) if !v.trim().is_empty() => v,
        _ => return,
    };

    let bx = BoxConnector::new(token);

    // Use a per-run filename so concurrent runs do not collide.
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let name = format!("argo-smoke-{nonce}.bin");

    // 1 KiB deterministic payload — byte at offset i is `(i % 251)`, which
    // exercises most byte values. Useful for catching transport-layer
    // mangling (e.g. accidental UTF-8 coercion).
    let payload: Vec<u8> = (0..1024_usize).map(|i| (i % 251) as u8).collect();

    let uploaded = bx
        .upload_file(&folder_id, &name, &payload)
        .await
        .expect("upload should succeed");
    assert_eq!(uploaded.name, name);
    assert_eq!(uploaded.size_bytes, payload.len() as u64);

    let downloaded = bx
        .download_file(&uploaded.id)
        .await
        .expect("download should succeed");
    assert_eq!(
        downloaded, payload,
        "downloaded body must be byte-identical to upload"
    );

    let url = bx
        .create_shared_link(&uploaded.id, SharedLinkAccess::Company)
        .await
        .expect("create_shared_link should succeed");
    assert!(
        url.starts_with("https://"),
        "shared link should be https, got {url:?}"
    );
}
