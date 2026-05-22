# V1.2 — Asana MCP connector

**Owner**: codex. **Depends on**: C-mcp-wire (#806) merged.

## Why

Native Asana surface so agents can drive task workflows
without falling back to the generic HTTP connector. Mirrors
zendesk/discord/jira pattern.

## Scope

Asana connector. PAT auth via `ASANA_TOKEN` against
`https://app.asana.com/api/1.0`.

- Methods: `list_projects`, `get_task`, `list_tasks`
  (project-scoped), `create_task`, `update_task`,
  `add_comment` (story), `list_users` (workspace-scoped)
- Read methods autonomous-safe; create/update/add_comment
  approval-gated

## Approach

REST client wrapping `reqwest` with `Authorization: Bearer
<token>` header. Each method = one method on `client.rs`
returning a typed Asana response; `service.rs` exposes MCP
tool calls and applies the approval gate.

## Files you may touch

NEW crate at `crates/argo-mcp-connector-asana/` mirroring
`argo-mcp-connector-zendesk`:
- `crates/argo-mcp-connector-asana/Cargo.toml` (NEW)
- `crates/argo-mcp-connector-asana/src/lib.rs` (NEW)
- `crates/argo-mcp-connector-asana/src/main.rs` (NEW)
- `crates/argo-mcp-connector-asana/src/auth.rs` (NEW)
- `crates/argo-mcp-connector-asana/src/client.rs` (NEW)
- `crates/argo-mcp-connector-asana/src/schemas.rs` (NEW)
- `crates/argo-mcp-connector-asana/src/service.rs` (NEW)
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
- [ ] Bearer auth via `ASANA_TOKEN`
- [ ] Approval-gate marks correct (read=false, write=true)
- [ ] 4–8 unit tests + 1 wire test against a recorded fixture
- [ ] Each `src/*.rs` ≤ 500 LOC (HARD RULE 6)
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
      clean

## Branch + PR

- Branch: `codex-connector-asana`
- PR title: `mcp: native Asana connector (codex-connector-asana)`

## Notes

- ~700 LOC total across 6 src files plus Cargo.toml.
- Asana wraps payloads in `{ "data": ... }` — strip in
  `client.rs`, return inner type to `service.rs`.
- Comments are "stories" in Asana's API — name the public
  method `add_comment` regardless.
