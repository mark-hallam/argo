# V1.2 — Salesforce MCP connector

**Owner**: codex. **Depends on**: C-mcp-wire (#806) merged.

## Scope

Salesforce connector. Auth via Connected App + OAuth client
credentials flow (`SF_CLIENT_ID`, `SF_CLIENT_SECRET`,
`SF_INSTANCE_URL`). Token cache under
`<cwd>/.argo/connectors/salesforce/`.

- Methods: `query` (SOQL passthrough), `get_object`,
  `create_object`, `update_object`, `list_accounts`,
  `list_opportunities`, `add_note`
- Read methods autonomous-safe; create/update/note
  approval-gated

## Files you may touch

NEW crate at `crates/argo-mcp-connector-salesforce/`
mirroring the file layout of `argo-mcp-connector-gmail`
(includes OAuth refresh + access-token cache):
- `crates/argo-mcp-connector-salesforce/Cargo.toml` (NEW)
- `crates/argo-mcp-connector-salesforce/src/lib.rs` (NEW)
- `crates/argo-mcp-connector-salesforce/src/main.rs` (NEW)
- `crates/argo-mcp-connector-salesforce/src/auth.rs` (NEW)
- `crates/argo-mcp-connector-salesforce/src/client.rs` (NEW)
- `crates/argo-mcp-connector-salesforce/src/schemas.rs` (NEW)
- `crates/argo-mcp-connector-salesforce/src/service.rs` (NEW)
- `Cargo.toml` (workspace) — narrowly: add the new crate to
  the `members` array

NO CLI subcommand — connectors are invoked via the MCP
manifest's `[mcp_servers]` section.

## Acceptance criteria
- [ ] All 7 methods callable
- [ ] OAuth client-credentials refresh + cache
- [ ] SOQL passthrough on `query`
- [ ] Approval-gate marks correct
- [ ] 4 unit tests (mocked HTTP) + 1 wire test
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
      clean

## Branch + PR
- Branch: `codex-connector-salesforce`
- PR title: `mcp: native Salesforce connector (codex-connector-salesforce)`
