# V1.2 — Zendesk MCP connector

**Owner**: codex. **Depends on**: C-mcp-wire (#806) merged.

## Scope

Zendesk connector. Auth via email + API token + subdomain
(`ZENDESK_EMAIL`, `ZENDESK_API_TOKEN`, `ZENDESK_SUBDOMAIN`).

- Methods: `list_tickets` (search query), `get_ticket`,
  `create_ticket`, `update_ticket`, `add_comment`,
  `list_users`, `list_macros`
- Read methods autonomous-safe; create/update/comment
  approval-gated

## Files you may touch

NEW crate at `crates/argo-mcp-connector-zendesk/` mirroring
the file layout of `argo-mcp-connector-slack`:
- `crates/argo-mcp-connector-zendesk/Cargo.toml` (NEW)
- `crates/argo-mcp-connector-zendesk/src/lib.rs` (NEW)
- `crates/argo-mcp-connector-zendesk/src/main.rs` (NEW)
- `crates/argo-mcp-connector-zendesk/src/auth.rs` (NEW)
- `crates/argo-mcp-connector-zendesk/src/client.rs` (NEW)
- `crates/argo-mcp-connector-zendesk/src/schemas.rs` (NEW)
- `crates/argo-mcp-connector-zendesk/src/service.rs` (NEW)
- `Cargo.toml` (workspace) — narrowly: add the new crate to
  the `members` array

NO CLI subcommand — connectors are invoked via the MCP
manifest's `[mcp_servers]` section.

## Acceptance criteria
- [ ] All 7 methods callable
- [ ] Search-query passthrough on `list_tickets`
- [ ] Approval-gate marks correct
- [ ] 4 unit tests + 1 wire test
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
      clean

## Branch + PR
- Branch: `codex-connector-zendesk`
- PR title: `mcp: native Zendesk connector (codex-connector-zendesk)`
