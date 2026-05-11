//! End-to-end smoke test against the real Dropbox `api.dropboxapi.com` /
//! `content.dropboxapi.com` surface.
//!
//! Gated by `ARGO_DBX_TEST_TOKEN`; without it the test is a no-op so the
//! suite stays safe on CI without credentials. To run locally:
//!
//! ```bash
//! export ARGO_DBX_TEST_TOKEN=<dropbox oauth2 access token>
//! cargo test -p argo-mcp-connector-dropbox --test dropbox_smoke -- --ignored
//! ```
//!
//! The test uploads a 1 KiB payload under `/skycot-test/`, downloads it back,
//! asserts byte-identical round-trip, then deletes it. A cleanup guard runs
//! the delete even if an assertion in the middle panics, so a flaky network
//! does not leave debris in the test Dropbox account.

use argo_mcp_connector_dropbox::{DbxWriteMode, DropboxConnector};

const TOKEN_ENV: &str = "ARGO_DBX_TEST_TOKEN";
const TEST_FOLDER: &str = "/skycot-test";

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
#[ignore = "requires ARGO_DBX_TEST_TOKEN"]
async fn round_trip_1kb_payload_then_clean_up() {
    let token = match std::env::var(TOKEN_ENV) {
        Ok(v) if !v.trim().is_empty() => v,
        _ => return,
    };

    let dbx = DropboxConnector::new(token);

    // Use a per-run path so concurrent runs do not collide.
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let test_path = format!("{TEST_FOLDER}/argo-smoke-{nonce}.bin");

    // Build a 1 KiB deterministic payload — the byte at offset i is (i % 251),
    // which exercises every byte value except a tail few. Useful for catching
    // any transport-layer mangling (e.g. accidental UTF-8 coercion of bytes).
    let payload: Vec<u8> = (0..1024_usize).map(|i| (i % 251) as u8).collect();

    // Cleanup guard — runs delete on drop. Captures references to the
    // connector and the path so the guard's Drop impl can still issue the
    // call even if a later assertion panics. Uses a blocking-runtime escape
    // hatch (`tokio::task::block_in_place`) to await the delete inside Drop.
    struct CleanupGuard<'a> {
        dbx: &'a DropboxConnector,
        path: String,
    }
    impl Drop for CleanupGuard<'_> {
        fn drop(&mut self) {
            // Best-effort cleanup; ignore errors so test failures still
            // surface as the *original* assertion, not the cleanup outcome.
            let path = self.path.clone();
            let dbx = self.dbx;
            tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(async move {
                    let _ = dbx.delete(&path).await;
                });
            });
        }
    }
    let _guard = CleanupGuard {
        dbx: &dbx,
        path: test_path.clone(),
    };

    let uploaded = dbx
        .upload(&test_path, &payload, DbxWriteMode::Overwrite)
        .await
        .expect("upload should succeed");
    assert_eq!(uploaded.path_display, test_path);
    assert_eq!(uploaded.size_bytes, payload.len() as u64);

    let downloaded = dbx
        .download(&test_path)
        .await
        .expect("download should succeed");
    assert_eq!(
        downloaded, payload,
        "downloaded body must be byte-identical to upload"
    );

    // Guard handles delete on scope exit.
}
