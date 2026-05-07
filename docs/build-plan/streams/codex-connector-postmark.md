# V1.2 — Postmark MCP connector

**Owner**: codex. **Depends on**: C-mcp-wire (#806) merged.

## Scope

Postmark connector. Auth via server token
(`POSTMARK_SERVER_TOKEN`).

- Methods: `list_servers`, `send_email` (approval),
  `list_messages`, `get_message`, `list_templates`
- Read methods autonomous-safe; send_email approval-gated

## Files you may touch

NEW crate at `crates/argo-mcp-connector-postmark/`
mirroring the file layout of `argo-mcp-connector-zendesk`
(#951) and `argo-mcp-connector-discord` (#953):
- `crates/argo-mcp-connector-postmark/Cargo.toml` (NEW)
- `crates/argo-mcp-connector-postmark/src/lib.rs` (NEW)
- `crates/argo-mcp-connector-postmark/src/main.rs` (NEW)
- `crates/argo-mcp-connector-postmark/src/auth.rs` (NEW)
- `crates/argo-mcp-connector-postmark/src/client.rs` (NEW)
- `crates/argo-mcp-connector-postmark/src/schemas.rs` (NEW)
- `crates/argo-mcp-connector-postmark/src/service.rs` (NEW)
- `Cargo.toml` (workspace) — narrowly: add the new crate to
  the `members` array

NO CLI subcommand — connectors are invoked via the MCP
manifest's `[mcp_servers]` section.

Cargo.lock regenerates on first build (matches #951/#953
pattern). Every src file ≤ 500 LOC.

## Acceptance criteria
- [ ] All 5 methods callable
- [ ] `send_email` approval-gate marks correct
- [ ] `X-Postmark-Server-Token` header on every request
- [ ] 4-8 unit tests + 1 wire test
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
      clean

## Branch + PR
- Branch: `codex-connector-postmark`
- PR title: `mcp: native Postmark connector
  (codex-connector-postmark)`
