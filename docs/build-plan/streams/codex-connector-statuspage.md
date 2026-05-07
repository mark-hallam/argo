# V1.2 — StatusPage.io MCP connector

**Owner**: codex. **Depends on**: C-mcp-wire (#806) merged.

## Scope

StatusPage.io connector. Auth via API key + page id
(`STATUSPAGE_API_KEY`, `STATUSPAGE_PAGE_ID`).

- Methods: `list_pages`, `list_components`,
  `list_incidents`, `create_incident` (approval),
  `update_incident` (approval), `get_metrics`
- Read methods autonomous-safe; create/update incident
  approval-gated

## Files you may touch

NEW crate at `crates/argo-mcp-connector-statuspage/`
mirroring the file layout of `argo-mcp-connector-zendesk`
(#951) and `argo-mcp-connector-discord` (#953):
- `crates/argo-mcp-connector-statuspage/Cargo.toml` (NEW)
- `crates/argo-mcp-connector-statuspage/src/lib.rs` (NEW)
- `crates/argo-mcp-connector-statuspage/src/main.rs` (NEW)
- `crates/argo-mcp-connector-statuspage/src/auth.rs` (NEW)
- `crates/argo-mcp-connector-statuspage/src/client.rs` (NEW)
- `crates/argo-mcp-connector-statuspage/src/schemas.rs` (NEW)
- `crates/argo-mcp-connector-statuspage/src/service.rs` (NEW)
- `Cargo.toml` (workspace) — narrowly: add the new crate to
  the `members` array

NO CLI subcommand — connectors are invoked via the MCP
manifest's `[mcp_servers]` section.

Cargo.lock regenerates on first build (matches #951/#953
pattern). Every src file ≤ 500 LOC.

## Acceptance criteria
- [ ] All 6 methods callable
- [ ] `create_incident` / `update_incident` approval-gate
      marks correct
- [ ] Page-id passthrough on all calls
- [ ] 4-8 unit tests + 1 wire test
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
      clean

## Branch + PR
- Branch: `codex-connector-statuspage`
- PR title: `mcp: native StatusPage.io connector
  (codex-connector-statuspage)`
