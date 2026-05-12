# Mission load-bearing wire — Box MCP connector

**Owner**: claude. **Depends on**: existing connector scaffold (`crates/argo-mcp-connector-dropbox/` as canonical document-storage pattern).
**Authored against**: origin/main `1cca6a69` on `2026-05-12`.

> SOURCE: MISSION_BACKLOG.md Section 3 Claim 5 #107. Adds Box.com connector to the document-storage MCP family (Dropbox shipped; OneDrive pending in sibling brief). Box uses OAuth 2.0 + REST API.

## Scope

NEW crate `crates/argo-mcp-connector-box/` mirroring `argo-mcp-connector-dropbox/` shape:

```rust
pub struct BoxConnector { client: Client, oauth_token: String }
impl BoxConnector {
    pub fn new(oauth_token: String) -> Self;
    pub async fn list_folder(&self, folder_id: &str) -> ArgoResult<Vec<BoxItem>>;
    pub async fn upload_file(&self, parent_folder_id: &str, name: &str, content: &[u8]) -> ArgoResult<BoxFile>;
    pub async fn download_file(&self, file_id: &str) -> ArgoResult<Vec<u8>>;
    pub async fn create_shared_link(&self, file_id: &str, access: SharedLinkAccess) -> ArgoResult<String>;
    pub async fn search(&self, query: &str, limit: u32) -> ArgoResult<Vec<BoxItem>>;
}
```

This is NOT in scope: Box Sign (digital signatures — separate brief); Box Skills framework; Box Webhooks; admin/governance APIs.

## Files you may touch

- `crates/argo-mcp-connector-box/` (NEW crate, ≤ 900 LOC)
  - `Cargo.toml` (≤ 30 LOC)
  - `src/lib.rs` (≤ 100 LOC) — re-exports
  - `src/client.rs` (≤ 380 LOC) — REST client + OAuth token refresh
  - `src/types.rs` (≤ 250 LOC) — DTOs + serde derives
  - `src/client_tests.rs` (≤ 280 LOC, sibling) — 7 unit tests with wiremock
  - `tests/box_smoke.rs` (≤ 150 LOC) — env-gated by `ARGO_BOX_TEST_TOKEN` + `ARGO_BOX_TEST_FOLDER_ID`
- `Cargo.toml` workspace — add `crates/argo-mcp-connector-box` to members

## Files you must NOT touch

- Sibling connector crates (`argo-mcp-connector-dropbox` etc.); the shared retry primitive in `argo-mcp-connector-shared`

## Acceptance criteria

- [ ] NEW crate compiles standalone
- [ ] Auth: `Authorization: Bearer <oauth_token>`; expired-token returns typed `ArgoError::Provider` with refresh hint
- [ ] All 5 methods implemented + tested with wiremock
- [ ] Shared-link access enum covers Box's `open` / `company` / `collaborators` levels
- [ ] Integration smoke env-gated
- [ ] `cargo clippy -p argo-mcp-connector-box --all-targets -- -D warnings` clean
- [ ] Every file < 500 LOC; BUILD_STATUS.md updated

## Branch + PR

- Branch: `claude-mission-mcp-connector-box`
- PR title: `mcp-connector: Box document storage (mission-mcp-connector-box)`

## Notes

- Box's API is REST/JSON; auth via OAuth 2.0 bearer tokens with refresh. Token refresh is the caller's responsibility — this connector takes a valid token; refresh is the operator's wrapper.
- The shared-link access levels are Box-specific terminology — preserve them in the enum (don't normalize to "public/restricted").
- Pair with `argo-mcp-connector-dropbox` + the OneDrive sibling brief for full document-storage coverage in the gtm-engineer + executive-assistant verticals.
