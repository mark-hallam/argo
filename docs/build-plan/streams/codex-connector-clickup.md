# V1.2 — ClickUp MCP connector

**Owner**: codex. **Depends on**: C-mcp-wire (#806) merged.

## Why

Native ClickUp surface so agents can drive task workflows.
Mirrors zendesk/discord pattern.

## Scope

ClickUp connector. Auth via personal token
(`CLICKUP_API_TOKEN`) against `https://api.clickup.com/api/v2`.

- Methods: `list_workspaces`, `list_lists` (folder/space
  scoped), `list_tasks` (list-scoped), `get_task`,
  `create_task`, `update_task`, `add_comment`
- Read methods autonomous-safe; create/update/add_comment
  approval-gated

## Approach

REST client wrapping `reqwest` with `Authorization:
<token>` header (NOT bearer — ClickUp uses raw header value).
`client.rs` exposes one method per public surface; `service.rs`
applies the approval gate.

## Files you may touch

NEW crate at `crates/argo-mcp-connector-clickup/` mirroring
`argo-mcp-connector-zendesk`:
- `crates/argo-mcp-connector-clickup/Cargo.toml` (NEW)
- `crates/argo-mcp-connector-clickup/src/lib.rs` (NEW)
- `crates/argo-mcp-connector-clickup/src/main.rs` (NEW)
- `crates/argo-mcp-connector-clickup/src/auth.rs` (NEW)
- `crates/argo-mcp-connector-clickup/src/client.rs` (NEW)
- `crates/argo-mcp-connector-clickup/src/schemas.rs` (NEW)
- `crates/argo-mcp-connector-clickup/src/service.rs` (NEW)
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
- [ ] `Authorization: <token>` header auth via
      `CLICKUP_API_TOKEN`
- [ ] Approval-gate marks correct (read=false, write=true)
- [ ] 4–8 unit tests + 1 wire test against a recorded fixture
- [ ] Each `src/*.rs` ≤ 500 LOC (HARD RULE 6)
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
      clean

## Branch + PR

- Branch: `codex-connector-clickup`
- PR title: `mcp: native ClickUp connector (codex-connector-clickup)`

## Notes

- ~700 LOC total across 6 src files plus Cargo.toml.
- ClickUp uses `Authorization: <token>` (raw value, no
  `Bearer` prefix) — easy footgun if you copy the bearer
  path from zendesk/discord auth.rs.
