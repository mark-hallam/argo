# V1.2 — Mailchimp MCP connector

**Owner**: codex. **Depends on**: C-mcp-wire (#806) merged.

## Scope

Mailchimp connector. Auth via API key + datacenter
(`MAILCHIMP_API_KEY`, `MAILCHIMP_DC`).

- Methods: `list_audiences`, `list_campaigns`,
  `get_campaign`, `send_campaign` (approval),
  `list_subscribers`, `add_subscriber`
- Read + add_subscriber autonomous-safe; send_campaign
  approval-gated

## Files you may touch

NEW crate at `crates/argo-mcp-connector-mailchimp/`
mirroring the file layout of `argo-mcp-connector-zendesk`
(#951) and `argo-mcp-connector-discord` (#953):
- `crates/argo-mcp-connector-mailchimp/Cargo.toml` (NEW)
- `crates/argo-mcp-connector-mailchimp/src/lib.rs` (NEW)
- `crates/argo-mcp-connector-mailchimp/src/main.rs` (NEW)
- `crates/argo-mcp-connector-mailchimp/src/auth.rs` (NEW)
- `crates/argo-mcp-connector-mailchimp/src/client.rs` (NEW)
- `crates/argo-mcp-connector-mailchimp/src/schemas.rs` (NEW)
- `crates/argo-mcp-connector-mailchimp/src/service.rs` (NEW)
- `Cargo.toml` (workspace) — narrowly: add the new crate to
  the `members` array

NO CLI subcommand — connectors are invoked via the MCP
manifest's `[mcp_servers]` section.

Cargo.lock regenerates on first build (matches #951/#953
pattern). Every src file ≤ 500 LOC.

## Acceptance criteria
- [ ] All 6 methods callable
- [ ] `send_campaign` approval-gate marks correct
- [ ] Base URL composed from `MAILCHIMP_DC` per-region
- [ ] 4-8 unit tests + 1 wire test
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
      clean

## Branch + PR
- Branch: `codex-connector-mailchimp`
- PR title: `mcp: native Mailchimp connector
  (codex-connector-mailchimp)`
