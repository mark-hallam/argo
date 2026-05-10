# Mission load-bearing wire — Microsoft Graph document creation executor

**Owner**: claude. **Depends on**: `argo-executor-document` crate SHIPPED (verified at `crates/argo-executor-document/src/`: csv, executor, lib, markdown, pdf, slides modules); `argo-mcp-connector-ms365` SHIPPED (auth + token flow).
**Authored against**: origin/main `2a386ed5` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-doc-executor-msgraph`.

> SOURCE: MISSION_BACKLOG.md Section 3 Claim 5 item #103 ("Build document creation executor — Microsoft Graph"). `argo-executor-document` ships CSV / Markdown / PDF / slides executors today, but Microsoft 365 (Word `.docx`, Excel `.xlsx`, OneDrive upload) requires a different API path. Mission Claim 5 ("real-world execution") needs MS Graph parity so verticals like sales-sdr can produce native Office files.

## Scope

Add a NEW `msgraph` module to `argo-executor-document`:

```rust
pub enum MsGraphFormat {
    Docx,                 // .docx via Open XML
    Xlsx { sheet_name: String },  // .xlsx
    UploadOnly,           // bytes uploaded as-is to OneDrive at given path
}

pub struct MsGraphExecutor {
    client: MsGraphClient,        // reuses argo-mcp-connector-ms365::auth + token
    drive_id: Option<String>,     // default = user's default drive
}

impl MsGraphExecutor {
    pub async fn create(&self, fmt: MsGraphFormat, path: &str, bytes: &[u8]) -> ArgoResult<MsGraphFile>;
    pub async fn read(&self, path: &str) -> ArgoResult<Vec<u8>>;
}

pub struct MsGraphFile {
    pub web_url: String,           // https://onedrive.live.com/... shareable link
    pub item_id: String,           // graph item ID
    pub size_bytes: u64,
}
```

The .docx and .xlsx generation paths use minimal Open XML templates (canned single-section docs or single-sheet workbooks) — NOT full document modeling. Operators wanting rich formatting use `UploadOnly` with their own pre-built file.

Wired as an `ActionExecutor` variant `MsGraphExecutor` discoverable via `argo vertical action` declarations (`executor.kind = "msgraph"`).

This is NOT in scope: rich docx layout (paragraphs/tables/styles beyond template substitution); xlsx formula/chart support (separate brief); SharePoint sites (only OneDrive default drive for v1); shared-with-me drives.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-executor-document/src/msgraph.rs` (NEW, ≤ 400 LOC) — `MsGraphFormat` enum, `MsGraphExecutor`, `MsGraphFile`, create + read methods.
- `crates/argo-executor-document/src/msgraph_tests.rs` (NEW, sibling) — 6 unit tests: docx-template-generates-valid-zip; xlsx-template-generates-valid-zip; upload-only-passes-bytes-through; create-returns-web-url; read-returns-bytes; auth-failure-surfaces-clearly.
- `crates/argo-executor-document/src/lib.rs` — narrowly scoped: ONE `pub mod msgraph;` line + re-export `MsGraphFormat`, `MsGraphExecutor`, `MsGraphFile`.
- `crates/argo-executor-document/src/executor.rs` — narrowly scoped: ADD `ActionExecutor::MsGraph(MsGraphExecutor)` variant under HARD RULE 5 exhaustive-match allowance; ADD dispatch arm calling `execute()`.
- `crates/argo-executor-document/tests/msgraph_smoke.rs` (NEW, ≤ 150 LOC) — integration smoke gated by `ARGO_MSGRAPH_TEST_TOKEN` env var; creates a test .docx, reads it back, asserts byte-identical.
- `crates/argo-executor-document/Cargo.toml` — narrowly scoped: ADD `argo-mcp-connector-ms365` dep (reuses auth machinery); verify zip crate dep for Open XML (likely already present).

## Files you must NOT touch

- `crates/argo-mcp-connector-ms365/**` — auth flow + token mgmt is upstream; reuse only
- `crates/argo-executor-document/src/{csv,markdown,pdf,slides}.rs` — sibling executors stable; mirror their shape, don't modify
- Any non-executor crate
- `crates/argo-store/migrations/**` — no schema change

## Acceptance criteria

- [ ] `MsGraphFormat` enum with Docx/Xlsx/UploadOnly variants
- [ ] `MsGraphExecutor::create` writes via Graph API at `https://graph.microsoft.com/v1.0/me/drive/root:/{path}:/content`
- [ ] `MsGraphExecutor::read` reads via Graph API; returns raw bytes
- [ ] `Docx` format generates minimal valid Open XML (header + body + content type)
- [ ] `Xlsx` format generates single-sheet workbook with content type marker
- [ ] `UploadOnly` passes bytes through verbatim (operator-supplied valid Open XML)
- [ ] OneDrive default drive used when `drive_id = None`
- [ ] Auth failure (expired token, 401) surfaces clearly with hint to refresh
- [ ] `ActionExecutor::MsGraph` variant wired in executor.rs
- [ ] Integration smoke gated by env-var; skipped silently when token absent
- [ ] 6 unit tests + 1 gated integration smoke
- [ ] No new public API outside the listed exports
- [ ] `cargo build -p argo-executor-document` clean
- [ ] `cargo clippy -p argo-executor-document --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-executor-document` passes (gated integration test skipped without env var)
- [ ] Final `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] Every modified file < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-doc-executor-msgraph`
- PR title: `executor-document: Microsoft Graph (.docx/.xlsx/upload) executor (mission-doc-executor-msgraph)`

## Notes for the implementer

- The "minimal Open XML template" approach is intentional. Full docx generation (Word's actual feature surface) is a multi-month project. For v1, operators producing real reports use `UploadOnly` with a docx they built elsewhere; the template path covers "write a one-paragraph status note."
- The Graph API path uses simple PUT-to-upload semantics for files under 4MB. Large file upload (chunked session) is out of scope; document the limit in the module doc-comment.
- Auth: reuse `argo-mcp-connector-ms365::auth` for token retrieval. Don't reinvent OAuth here — that connector already handles refresh.
- Gated integration test pattern: `if std::env::var("ARGO_MSGRAPH_TEST_TOKEN").is_err() { return; }` at the top of the test. Don't `#[ignore]` — that hides the test from `cargo test` output entirely.
