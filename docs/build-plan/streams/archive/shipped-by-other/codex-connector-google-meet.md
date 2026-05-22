# V1.2 — Google Meet MCP connector

**Owner**: codex. **Depends on**: C-mcp-wire (#806) merged.

## Scope

Google Meet connector. Auth via OAuth bearer token
(`GOOGLE_OAUTH_TOKEN`).

- Methods: `list_meetings`,
  `schedule_meeting` (approval), `get_meeting`,
  `list_participants`, `get_recording`
- Read methods autonomous-safe; schedule_meeting
  approval-gated

## Files you may touch

NEW crate at `crates/argo-mcp-connector-google-meet/`
mirroring the file layout of `argo-mcp-connector-zendesk`
(#951) and `argo-mcp-connector-discord` (#953):
- `crates/argo-mcp-connector-google-meet/Cargo.toml` (NEW)
- `crates/argo-mcp-connector-google-meet/src/lib.rs` (NEW)
- `crates/argo-mcp-connector-google-meet/src/main.rs` (NEW)
- `crates/argo-mcp-connector-google-meet/src/auth.rs` (NEW)
- `crates/argo-mcp-connector-google-meet/src/client.rs`
  (NEW)
- `crates/argo-mcp-connector-google-meet/src/schemas.rs`
  (NEW)
- `crates/argo-mcp-connector-google-meet/src/service.rs`
  (NEW)
- `Cargo.toml` (workspace) — narrowly: add the new crate to
  the `members` array

NO CLI subcommand — connectors are invoked via the MCP
manifest's `[mcp_servers]` section.

Cargo.lock regenerates on first build (matches #951/#953
pattern). Every src file ≤ 500 LOC.

## Acceptance criteria
- [ ] All 5 methods callable
- [ ] `schedule_meeting` approval-gate marks correct
- [ ] Bearer token forwarded on every request
- [ ] 4-8 unit tests + 1 wire test
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
      clean

## Branch + PR
- Branch: `codex-connector-google-meet`
- PR title: `mcp: native Google Meet connector
  (codex-connector-google-meet)`
