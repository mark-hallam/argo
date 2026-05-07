# V1.2 — Zoom MCP connector

**Owner**: codex. **Depends on**: C-mcp-wire (#806) merged.

## Scope

Zoom connector. OAuth2 server-to-server auth
(`ZOOM_ACCOUNT_ID`, `ZOOM_CLIENT_ID`,
`ZOOM_CLIENT_SECRET`).

- Methods: `list_meetings`, `get_meeting`,
  `schedule_meeting` (approval),
  `cancel_meeting` (approval), `get_recording`,
  `list_users`
- Read methods autonomous-safe; schedule/cancel
  approval-gated

## Files you may touch

NEW crate at `crates/argo-mcp-connector-zoom/`
mirroring the file layout of `argo-mcp-connector-zendesk`
(#951) and `argo-mcp-connector-discord` (#953):
- `crates/argo-mcp-connector-zoom/Cargo.toml` (NEW)
- `crates/argo-mcp-connector-zoom/src/lib.rs` (NEW)
- `crates/argo-mcp-connector-zoom/src/main.rs` (NEW)
- `crates/argo-mcp-connector-zoom/src/auth.rs` (NEW)
- `crates/argo-mcp-connector-zoom/src/client.rs` (NEW)
- `crates/argo-mcp-connector-zoom/src/schemas.rs` (NEW)
- `crates/argo-mcp-connector-zoom/src/service.rs` (NEW)
- `Cargo.toml` (workspace) — narrowly: add the new crate to
  the `members` array

NO CLI subcommand — connectors are invoked via the MCP
manifest's `[mcp_servers]` section.

Cargo.lock regenerates on first build (matches #951/#953
pattern). Every src file ≤ 500 LOC.

## Acceptance criteria
- [ ] All 6 methods callable
- [ ] OAuth2 server-to-server token exchange in `auth.rs`
- [ ] `schedule_meeting` / `cancel_meeting` approval-gate
      marks correct
- [ ] 4-8 unit tests + 1 wire test
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
      clean

## Branch + PR
- Branch: `codex-connector-zoom`
- PR title: `mcp: native Zoom connector
  (codex-connector-zoom)`
