# V1.2 — Monday.com MCP connector

**Owner**: codex. **Depends on**: C-mcp-wire (#806) merged.

## Why

First-class Monday.com support so SKYCOT agents can read and
mutate boards/items without reaching for a generic HTTP MCP.
Mirrors the proven zendesk/discord/jira connector pattern.

## Scope

Monday.com connector. Auth via API token (`MONDAY_API_TOKEN`,
GraphQL endpoint `https://api.monday.com/v2`).

- Methods: `list_boards`, `get_board`, `list_items`
  (board-scoped), `create_item`, `update_item`, `add_update`
  (post a comment/update), `list_users`
- Read methods autonomous-safe; create/update/add_update
  approval-gated

## Approach

Single GraphQL POST client wrapping `reqwest` with
`Authorization: <token>` header. Each method = one canned
GraphQL query/mutation in `client.rs`; `service.rs` exposes
the MCP tool surface and applies the approval gate.

## Files you may touch

NEW crate at `crates/argo-mcp-connector-monday/` mirroring
`argo-mcp-connector-zendesk`:
- `crates/argo-mcp-connector-monday/Cargo.toml` (NEW)
- `crates/argo-mcp-connector-monday/src/lib.rs` (NEW)
- `crates/argo-mcp-connector-monday/src/main.rs` (NEW)
- `crates/argo-mcp-connector-monday/src/auth.rs` (NEW)
- `crates/argo-mcp-connector-monday/src/client.rs` (NEW)
- `crates/argo-mcp-connector-monday/src/schemas.rs` (NEW)
- `crates/argo-mcp-connector-monday/src/service.rs` (NEW)
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
- [ ] Bearer auth via `MONDAY_API_TOKEN`
- [ ] Approval-gate marks correct (read=false, write=true)
- [ ] 4–8 unit tests + 1 wire test against a recorded fixture
- [ ] Each `src/*.rs` ≤ 500 LOC (HARD RULE 6)
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
      clean

## Branch + PR

- Branch: `codex-connector-monday`
- PR title: `mcp: native Monday.com connector (codex-connector-monday)`

## Notes

- ~700 LOC total across 6 src files plus Cargo.toml.
- GraphQL — single endpoint, multiple queries; not REST.
- Monday rate limit is "complexity"-based — surface 429s as
  retryable errors but do not implement client-side backoff
  (gateway handles).
