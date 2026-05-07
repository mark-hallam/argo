# V1.2 — Calendly MCP connector

**Owner**: codex. **Depends on**: C-mcp-wire (#806) merged.

## Scope

Calendly connector. Auth via personal access token
(`CALENDLY_API_KEY`).

- Methods: `list_event_types`, `list_scheduled_events`,
  `get_event`, `cancel_event` (approval), `get_invitee`
- Read methods autonomous-safe; cancel_event approval-gated

## Files you may touch

NEW crate at `crates/argo-mcp-connector-calendly/`
mirroring the file layout of `argo-mcp-connector-zendesk`
(#951) and `argo-mcp-connector-discord` (#953):
- `crates/argo-mcp-connector-calendly/Cargo.toml` (NEW)
- `crates/argo-mcp-connector-calendly/src/lib.rs` (NEW)
- `crates/argo-mcp-connector-calendly/src/main.rs` (NEW)
- `crates/argo-mcp-connector-calendly/src/auth.rs` (NEW)
- `crates/argo-mcp-connector-calendly/src/client.rs` (NEW)
- `crates/argo-mcp-connector-calendly/src/schemas.rs` (NEW)
- `crates/argo-mcp-connector-calendly/src/service.rs` (NEW)
- `Cargo.toml` (workspace) — narrowly: add the new crate to
  the `members` array

NO CLI subcommand — connectors are invoked via the MCP
manifest's `[mcp_servers]` section.

Cargo.lock regenerates on first build (matches #951/#953
pattern). Every src file ≤ 500 LOC.

## Acceptance criteria
- [ ] All 5 methods callable
- [ ] `cancel_event` approval-gate marks correct
- [ ] User URI scoping on event-type / event listings
- [ ] 4-8 unit tests + 1 wire test
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
      clean

## Branch + PR
- Branch: `codex-connector-calendly`
- PR title: `mcp: native Calendly connector
  (codex-connector-calendly)`
