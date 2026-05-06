# V1.2 — ClickUp MCP connector

**Owner**: codex. **Depends on**: C-mcp-wire (#806) merged.

## Scope

ClickUp connector. Auth via personal-token (`CLICKUP_TOKEN`).

- Methods: `list_workspaces`, `list_spaces` (workspace-scoped),
  `list_lists` (space-scoped), `list_tasks` (list-scoped),
  `get_task`, `create_task`, `update_task`, `add_comment`
- Read autonomous-safe; create/update/comment approval-gated

## Files you may touch

NEW crate at `crates/argo-mcp-connector-clickup/` mirroring
`argo-mcp-connector-asana`:
- `crates/argo-mcp-connector-clickup/Cargo.toml` (NEW)
- `crates/argo-mcp-connector-clickup/src/{lib,main,auth,client,schemas,service}.rs` (NEW)
- `Cargo.toml` workspace — narrowly: add new member

NO CLI subcommand.

## Acceptance criteria
- [ ] All 8 methods callable
- [ ] Personal-token bearer auth via CLICKUP_TOKEN
- [ ] Approval-gate marks correct
- [ ] 4 unit tests + 1 wire test
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean

## Branch + PR
- Branch: `codex-connector-clickup`
- PR title: `mcp: native ClickUp connector (codex-connector-clickup)`
