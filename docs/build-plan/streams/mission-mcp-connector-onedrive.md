# Mission load-bearing wire — OneDrive MCP connector

**Owner**: claude. **Depends on**: existing connector scaffold (mirror `argo-mcp-connector-dropbox/`).
**Authored against**: origin/main `1cca6a69` on `2026-05-12`.

> SOURCE: MISSION_BACKLOG.md Section 3 Claim 5 #107. Adds Microsoft OneDrive connector. Uses Microsoft Graph API. Overlaps with the Microsoft 365 ecosystem (Outlook, SharePoint, Teams) but stays scoped to file ops.

## Scope

NEW crate `crates/argo-mcp-connector-onedrive/` via Microsoft Graph:

```rust
pub struct OneDriveConnector { client: Client, oauth_token: String }
impl OneDriveConnector {
    pub fn new(oauth_token: String) -> Self;
    pub async fn list_folder(&self, item_id: Option<&str>) -> ArgoResult<Vec<DriveItem>>;
    pub async fn upload_small_file(&self, parent: &str, name: &str, content: &[u8]) -> ArgoResult<DriveItem>; // <4MB simple upload
    pub async fn upload_large_file(&self, parent: &str, name: &str, content: &[u8]) -> ArgoResult<DriveItem>; // resumable session
    pub async fn download_file(&self, item_id: &str) -> ArgoResult<Vec<u8>>;
    pub async fn create_share_link(&self, item_id: &str, link_type: LinkType, scope: LinkScope) -> ArgoResult<String>;
    pub async fn search(&self, query: &str) -> ArgoResult<Vec<DriveItem>>;
}
```

This is NOT in scope: SharePoint (different drive resource); Microsoft Lists; Outlook attachment scrape; admin tenant APIs.

## Files you may touch

- `crates/argo-mcp-connector-onedrive/` (NEW crate, ≤ 1000 LOC)
  - `Cargo.toml` (≤ 30 LOC)
  - `src/lib.rs` (≤ 100 LOC), `src/client.rs` (≤ 420 LOC; resumable-upload session needs its own state machine), `src/types.rs` (≤ 280 LOC)
  - `src/client_tests.rs` (≤ 320 LOC sibling) — 8 unit tests with wiremock
  - `tests/onedrive_smoke.rs` (≤ 180 LOC) — env-gated
- `Cargo.toml` workspace

## Files you must NOT touch

- Other connector crates; the SharePoint surface (different scope)

## Acceptance criteria

- [ ] All 6 methods implemented + tested
- [ ] Small-vs-large upload split at 4MB boundary (Graph API requirement)
- [ ] Resumable upload session handles connection drops (resumes from last-byte-acked)
- [ ] LinkType (view / edit / embed) + LinkScope (anonymous / organization / users) preserved as enums
- [ ] Auth via Bearer; 401 returns typed error with refresh hint
- [ ] `cargo clippy -p argo-mcp-connector-onedrive --all-targets -- -D warnings` clean
- [ ] Every file < 500 LOC; BUILD_STATUS.md updated

## Branch + PR

- Branch: `claude-mission-mcp-connector-onedrive`
- PR title: `mcp-connector: OneDrive via Microsoft Graph (mission-mcp-connector-onedrive)`

## Notes

- The resumable-upload session is the trickiest piece — it's a multi-roundtrip protocol. Reference: Microsoft Graph docs `/me/drive/items/{parent}/createUploadSession`.
- Auth scopes needed: `Files.ReadWrite` (and `Files.ReadWrite.All` for shared drives). Document in `# Notes` so operators know what to request in their OAuth app.
- The drive-item ID format includes a `driveId!itemId` style for shared drives — handle gracefully.
