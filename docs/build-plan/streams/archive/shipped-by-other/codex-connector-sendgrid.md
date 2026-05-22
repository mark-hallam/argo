# V1.2 — SendGrid MCP connector

**Owner**: codex. **Depends on**: C-mcp-wire (#806) merged.

## Scope

SendGrid connector. Auth via API key (`SENDGRID_API_KEY`).

- Methods: `list_templates`, `send_email` (approval),
  `list_contacts`, `add_contact`, `list_lists`
- Read + add_contact autonomous-safe; send_email
  approval-gated

## Files you may touch

NEW crate at `crates/argo-mcp-connector-sendgrid/`
mirroring the file layout of `argo-mcp-connector-zendesk`
(#951) and `argo-mcp-connector-discord` (#953):
- `crates/argo-mcp-connector-sendgrid/Cargo.toml` (NEW)
- `crates/argo-mcp-connector-sendgrid/src/lib.rs` (NEW)
- `crates/argo-mcp-connector-sendgrid/src/main.rs` (NEW)
- `crates/argo-mcp-connector-sendgrid/src/auth.rs` (NEW)
- `crates/argo-mcp-connector-sendgrid/src/client.rs` (NEW)
- `crates/argo-mcp-connector-sendgrid/src/schemas.rs` (NEW)
- `crates/argo-mcp-connector-sendgrid/src/service.rs` (NEW)
- `Cargo.toml` (workspace) — narrowly: add the new crate to
  the `members` array

NO CLI subcommand — connectors are invoked via the MCP
manifest's `[mcp_servers]` section.

Cargo.lock regenerates on first build (matches #951/#953
pattern). Every src file ≤ 500 LOC.

## Acceptance criteria
- [ ] All 5 methods callable
- [ ] `send_email` approval-gate marks correct
- [ ] Template-id passthrough on send_email
- [ ] 4-8 unit tests + 1 wire test
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
      clean

## Branch + PR
- Branch: `codex-connector-sendgrid`
- PR title: `mcp: native SendGrid connector
  (codex-connector-sendgrid)`
