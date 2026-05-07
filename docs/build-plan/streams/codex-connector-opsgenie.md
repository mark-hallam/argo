# V1.2 — Opsgenie MCP connector

**Owner**: codex. **Depends on**: C-mcp-wire (#806) merged.

## Scope

Opsgenie connector. Auth via API key (`OPSGENIE_API_KEY`).

- Methods: `list_alerts`, `get_alert`,
  `ack_alert` (approval), `close_alert` (approval),
  `list_teams`, `list_oncall_schedules`
- Read methods autonomous-safe; ack/close approval-gated

## Files you may touch

NEW crate at `crates/argo-mcp-connector-opsgenie/`
mirroring the file layout of `argo-mcp-connector-zendesk`
(#951) and `argo-mcp-connector-discord` (#953):
- `crates/argo-mcp-connector-opsgenie/Cargo.toml` (NEW)
- `crates/argo-mcp-connector-opsgenie/src/lib.rs` (NEW)
- `crates/argo-mcp-connector-opsgenie/src/main.rs` (NEW)
- `crates/argo-mcp-connector-opsgenie/src/auth.rs` (NEW)
- `crates/argo-mcp-connector-opsgenie/src/client.rs` (NEW)
- `crates/argo-mcp-connector-opsgenie/src/schemas.rs` (NEW)
- `crates/argo-mcp-connector-opsgenie/src/service.rs` (NEW)
- `Cargo.toml` (workspace) — narrowly: add the new crate to
  the `members` array

NO CLI subcommand — connectors are invoked via the MCP
manifest's `[mcp_servers]` section.

Cargo.lock regenerates on first build (matches #951/#953
pattern). Every src file ≤ 500 LOC.

## Acceptance criteria
- [ ] All 6 methods callable
- [ ] `ack_alert` / `close_alert` approval-gate marks
      correct
- [ ] 4-8 unit tests + 1 wire test
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
      clean

## Branch + PR
- Branch: `codex-connector-opsgenie`
- PR title: `mcp: native Opsgenie connector
  (codex-connector-opsgenie)`
