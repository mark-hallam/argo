# V1.2 — Confluence MCP connector

**Owner**: codex. **Depends on**: C-mcp-wire (#806) merged.

## Scope

Confluence connector. Auth via base URL + user + API token
(`CONFLUENCE_URL`, `CONFLUENCE_USER`,
`CONFLUENCE_API_TOKEN`).

- Methods: `list_spaces`, `list_pages`, `get_page`,
  `create_page` (approval), `update_page` (approval),
  `search_content`
- Read methods autonomous-safe; create/update page
  approval-gated

## Files you may touch

NEW crate at `crates/argo-mcp-connector-confluence/`
mirroring the file layout of `argo-mcp-connector-zendesk`
(#951) and `argo-mcp-connector-discord` (#953):
- `crates/argo-mcp-connector-confluence/Cargo.toml` (NEW)
- `crates/argo-mcp-connector-confluence/src/lib.rs` (NEW)
- `crates/argo-mcp-connector-confluence/src/main.rs` (NEW)
- `crates/argo-mcp-connector-confluence/src/auth.rs` (NEW)
- `crates/argo-mcp-connector-confluence/src/client.rs` (NEW)
- `crates/argo-mcp-connector-confluence/src/schemas.rs` (NEW)
- `crates/argo-mcp-connector-confluence/src/service.rs` (NEW)
- `Cargo.toml` (workspace) — narrowly: add the new crate to
  the `members` array

NO CLI subcommand — connectors are invoked via the MCP
manifest's `[mcp_servers]` section.

Cargo.lock regenerates on first build (matches #951/#953
pattern). Every src file ≤ 500 LOC.

## Acceptance criteria
- [ ] All 6 methods callable
- [ ] `create_page` / `update_page` approval-gate marks
      correct
- [ ] CQL passthrough on `search_content`
- [ ] 4-8 unit tests + 1 wire test
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
      clean

## Branch + PR
- Branch: `codex-connector-confluence`
- PR title: `mcp: native Confluence connector
  (codex-connector-confluence)`
