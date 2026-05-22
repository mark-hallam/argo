# V1.2 — Trello MCP connector

**Owner**: codex. **Depends on**: C-mcp-wire (#806) merged.

## Why

Native Trello surface so SKYCOT agents can drive boards/cards
without the generic HTTP MCP. Mirrors zendesk/discord pattern.

## Scope

Trello connector. Auth via API key + token query params
(`TRELLO_KEY` + `TRELLO_TOKEN`) against
`https://api.trello.com/1`.

- Methods: `list_boards` (member-scoped), `list_cards`
  (board-scoped), `get_card`, `create_card`, `update_card`,
  `add_comment`, `move_card` (idList change)
- Read methods autonomous-safe; create/update/move/comment
  approval-gated

## Approach

REST client wrapping `reqwest`; auth flows as `?key=...&token=...`
query params (Trello has no bearer header). Each method = one
method on `client.rs`; `service.rs` exposes MCP tool calls and
applies the approval gate.

## Files you may touch

NEW crate at `crates/argo-mcp-connector-trello/` mirroring
`argo-mcp-connector-zendesk`:
- `crates/argo-mcp-connector-trello/Cargo.toml` (NEW)
- `crates/argo-mcp-connector-trello/src/lib.rs` (NEW)
- `crates/argo-mcp-connector-trello/src/main.rs` (NEW)
- `crates/argo-mcp-connector-trello/src/auth.rs` (NEW)
- `crates/argo-mcp-connector-trello/src/client.rs` (NEW)
- `crates/argo-mcp-connector-trello/src/schemas.rs` (NEW)
- `crates/argo-mcp-connector-trello/src/service.rs` (NEW)
- `Cargo.toml` (workspace) — narrowly: add the new crate to
  the `members` array

NO CLI subcommand — connectors are invoked via the MCP
manifest's `[mcp_servers]` section.

## Files you must NOT touch

- Any other connector crate
- `crates/argo-cli/` and `crates/argo-gateway/`
- Workspace `Cargo.lock` is regenerated on first build —
  matches #951/#953 boundary pattern (don't hand-edit)

## Acceptance criteria

- [ ] All 7 methods callable end-to-end
- [ ] Auth via `TRELLO_KEY` + `TRELLO_TOKEN` query params
- [ ] Approval-gate marks correct (read=false, write=true)
- [ ] 4–8 unit tests + 1 wire test against a recorded fixture
- [ ] Each `src/*.rs` ≤ 500 LOC (HARD RULE 6)
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
      clean

## Branch + PR

- Branch: `codex-connector-trello`
- PR title: `mcp: native Trello connector (codex-connector-trello)`

## Notes

- ~700 LOC total across 6 src files plus Cargo.toml.
- Auth as query params is unusual — keep secret values OUT
  of debug logs; `auth.rs` should provide a `with_auth()`
  helper that appends params on every request.
- `move_card` is implemented via PUT `/cards/{id}` with
  `idList` set — keep it as a separate public method for
  clarity even though it's a thin wrapper.
