//! End-to-end smoke test against the real Microsoft Graph
//! (`graph.microsoft.com/v1.0`) surface.
//!
//! Gated by `ARGO_ONEDRIVE_TEST_TOKEN`; without it the test is a no-op so
//! the suite stays safe on CI without credentials. To run locally:
//!
//! ```bash
//! export ARGO_ONEDRIVE_TEST_TOKEN=<microsoft graph oauth2 access token>
//! cargo test -p argo-mcp-connector-onedrive --test onedrive_smoke -- --ignored
//! ```
//!
//! The token must carry the `Files.ReadWrite` scope (and `Files.ReadWrite.All`
//! if the test account targets a shared drive). The test uploads a 1 KiB
//! payload to the user's drive root, lists the root to confirm the upload is
//! visible, downloads the bytes back, asserts byte-identical round-trip, and
//! deletes the uploaded item via a Drop-guarded cleanup. The cleanup runs
//! even if a middle assertion panics so a flaky network does not leave
//! debris in the test account.

use argo_mcp_connector_onedrive::{DriveItem, OneDriveConnector};

const TOKEN_ENV: &str = "ARGO_ONEDRIVE_TEST_TOKEN";

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
#[ignore = "requires ARGO_ONEDRIVE_TEST_TOKEN"]
async fn round_trip_1kb_payload_then_clean_up() {
    let token = match std::env::var(TOKEN_ENV) {
        Ok(v) if !v.trim().is_empty() => v,
        _ => return,
    };

    let od = OneDriveConnector::new(token);
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let name = format!("argo-smoke-{nonce}.bin");

    // 1 KiB deterministic payload — byte at offset i is (i % 251). Exercises
    // every value except a tail few; useful for catching transport-layer
    // mangling (e.g. accidental UTF-8 coercion).
    let payload: Vec<u8> = (0..1024_usize).map(|i| (i % 251) as u8).collect();

    let uploaded = od
        .upload_small_file("root", &name, &payload)
        .await
        .expect("upload should succeed");
    let uploaded_id = match &uploaded {
        DriveItem::File(f) => f.id.clone(),
        DriveItem::Folder(_) => panic!("upload returned a Folder, expected File"),
    };

    // Cleanup guard — issues a best-effort DELETE-via-Graph on Drop so the
    // test account stays clean even if a middle assertion panics. Uses
    // reqwest directly because the connector does not currently expose a
    // `delete` method (out of scope for v1 — files persist until cleaned up
    // out-of-band).
    struct CleanupGuard<'a> {
        token: &'a str,
        item_id: String,
    }
    impl Drop for CleanupGuard<'_> {
        fn drop(&mut self) {
            let token = self.token.to_string();
            let item_id = self.item_id.clone();
            tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(async move {
                    let _ = reqwest::Client::new()
                        .delete(format!(
                            "https://graph.microsoft.com/v1.0/me/drive/items/{item_id}"
                        ))
                        .bearer_auth(token)
                        .send()
                        .await;
                });
            });
        }
    }
    let _guard = CleanupGuard {
        token: &od_token(&od),
        item_id: uploaded_id.clone(),
    };

    let entries = od.list_folder(None).await.expect("list_folder should succeed");
    let found = entries.iter().any(|e| match e {
        DriveItem::File(f) => f.id == uploaded_id,
        _ => false,
    });
    assert!(found, "uploaded item must appear in root listing");

    let downloaded = od
        .download_file(&uploaded_id)
        .await
        .expect("download should succeed");
    assert_eq!(
        downloaded, payload,
        "downloaded body must be byte-identical to upload"
    );
}

/// Recover the access token from the connector so the cleanup guard can issue
/// the DELETE call without holding a second copy in scope. The connector
/// doesn't expose the token publicly, so we read it back from the env var
/// the test already consulted.
fn od_token(_od: &OneDriveConnector) -> String {
    std::env::var(TOKEN_ENV).unwrap_or_default()
}
