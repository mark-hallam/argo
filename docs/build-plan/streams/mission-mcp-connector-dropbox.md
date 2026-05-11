# Mission load-bearing wire — Dropbox MCP connector

**Owner**: claude. **Depends on**: existing MCP connector scaffold (`crates/argo-mcp-connector-gmail/` as canonical OAuth2-bearer file-API pattern; `crates/argo-mcp-connector-shared/` retry primitive).
**Authored against**: origin/main `d7c20e4d` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-mcp-connector-dropbox`.

> SOURCE: MISSION_BACKLOG.md Section 3 Claim 5 #107 ("Add MCP connector Dropbox/Box/OneDrive"). Operators on the bookkeeping-assistant / data-entry-clerk verticals need a place to read source documents and write outputs that isn't tied to MS Graph (covered by `mission-doc-executor-msgraph` already). Dropbox is the smallest viable file-storage connector. Box and OneDrive are separate follow-up briefs.

## Scope

NEW crate `crates/argo-mcp-connector-dropbox/` mirroring `argo-mcp-connector-gmail` shape (OAuth2-bearer auth, file CRUD + shared-link generation):

```rust
// crates/argo-mcp-connector-dropbox/src/lib.rs
pub struct DropboxConnector { client: Client, access_token: String }
impl DropboxConnector {
    pub fn new(access_token: String) -> Self;
    pub async fn list_folder(&self, path: &str, recursive: bool) -> ArgoResult<Vec<DbxEntry>>;
    pub async fn upload(&self, path: &str, bytes: &[u8], mode: DbxWriteMode) -> ArgoResult<DbxFile>;
    pub async fn download(&self, path: &str) -> ArgoResult<Vec<u8>>;
    pub async fn delete(&self, path: &str) -> ArgoResult<()>;
    pub async fn create_shared_link(&self, path: &str, requires_password: bool) -> ArgoResult<DbxSharedLink>;
    pub async fn search(&self, query: &str, max_results: u32) -> ArgoResult<Vec<DbxEntry>>;
}
pub enum DbxEntry { File(DbxFile), Folder(DbxFolder) }
pub struct DbxFile { pub id: String, pub name: String, pub path_display: String, pub size_bytes: u64, pub content_hash: String, pub server_modified: DateTime<Utc> }
pub struct DbxFolder { pub id: String, pub name: String, pub path_display: String }
pub enum DbxWriteMode { Add, Overwrite, Update { rev: String } }
pub struct DbxSharedLink { pub url: String, pub path: String, pub expires: Option<DateTime<Utc>> }
```

Upload size limit: 150MB single-request (`/files/upload`); larger files go through `/files/upload_session/*` chunked endpoints — chunked upload is OUT for v1 (document the cap in the module doc-comment, surface as `ArgoError::Provider` with hint to use the executor-document `UploadOnly` path through MS Graph for now).

This is NOT in scope: chunked upload sessions; team admin endpoints (`/team/*`); Dropbox Paper docs; OneDrive (separate brief — different OAuth scope + Microsoft Graph base); Box (separate brief — different API entirely); webhook-event ingest from Dropbox.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-mcp-connector-dropbox/` (NEW crate, ≤ 900 LOC total)
  - `Cargo.toml` (NEW, ≤ 30 LOC)
  - `src/lib.rs` (NEW, ≤ 120 LOC) — re-exports
  - `src/client.rs` (NEW, ≤ 400 LOC) — `DropboxConnector` + REST calls to `https://api.dropboxapi.com/2/` (metadata) and `https://content.dropboxapi.com/2/` (file content)
  - `src/types.rs` (NEW, ≤ 200 LOC) — DTOs + serde derives
  - `src/client_tests.rs` (NEW, sibling, ≤ 300 LOC) — 6 unit tests w/ wiremock: list-folder, upload-overwrite, download-roundtrip, delete-returns-empty-body, shared-link-public, oversized-rejection
  - `tests/dropbox_smoke.rs` (NEW, ≤ 200 LOC) — gated by `ARGO_DBX_TEST_TOKEN`; uploads a 1KB test file under `/skycot-test/`, downloads, asserts byte-identical, deletes
- `Cargo.toml` (workspace) — narrowly scoped: add `crates/argo-mcp-connector-dropbox` to `members`

## Files you must NOT touch

- `crates/argo-mcp-connector-gmail/**` — sibling stable, mirror its OAuth2-bearer pattern, don't modify
- `crates/argo-executor-document/**` — file-content executors live on the document side; this connector is metadata/CRUD only
- `crates/argo-store/migrations/**` — no schema change

## Acceptance criteria

- [ ] NEW `crates/argo-mcp-connector-dropbox/` crate compiles standalone
- [ ] `DropboxConnector::new` constructs with access_token
- [ ] All metadata calls POST to `https://api.dropboxapi.com/2/files/<action>` with `Authorization: Bearer <token>` and JSON body
- [ ] All content calls POST to `https://content.dropboxapi.com/2/files/<action>` with `Dropbox-API-Arg: <json>` header and binary body
- [ ] `upload` enforces 150MB cap with typed error; chunked-upload-required hint in error message
- [ ] `download` returns the raw bytes; metadata is in the `Dropbox-API-Result` response header
- [ ] `create_shared_link` returns `DbxSharedLink`; `requires_password = true` sets visibility to password-protected
- [ ] 6 unit tests with wiremock; integration smoke gated by env-var
- [ ] `cargo clippy -p argo-mcp-connector-dropbox --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-mcp-connector-dropbox` passes
- [ ] Final `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] Every new file < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-mcp-connector-dropbox`
- PR title: `mcp-connector: Dropbox file storage (list / upload / download / shared-link) (mission-mcp-connector-dropbox)`

## Notes for the implementer

- Dropbox is RPC-over-HTTP-POST (all endpoints are POST even for "reads") — not REST. Document this in module docs.
- Two host bases: metadata operations on `api.dropboxapi.com`, content operations on `content.dropboxapi.com`. Treat the host as a function of the operation, not a per-connector setting.
- Paths are always absolute and start with `/`. An empty string `""` means the user's root. Validate that path starts with `/` (or is empty) before any network call.
- Token refresh is OUT of scope for this brief — the connector takes a long-lived access token. Refresh-via-`refresh_token` is the next-brief follow-up once this lands.
- Retriable errors: 429 (with `Retry-After`), 5xx, network timeout. Reuse `argo-mcp-connector-shared::retry` if it exists.
- The integration smoke test should clean up after itself (delete the test file even on assertion failure) — wrap the test body in a guard.
