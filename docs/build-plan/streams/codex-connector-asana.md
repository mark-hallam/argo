# V1.2 — Asana MCP connector

**Owner**: codex. **Depends on**: C-mcp-wire (#806) merged.

## Scope

Asana connector. PAT auth via `ASANA_PAT`.

- Methods: `list_workspaces`, `list_projects`, `list_tasks`
  (project-scoped), `get_task`, `create_task`, `update_task`,
  `complete_task`, `add_comment`
- Read autonomous-safe; create/update/complete/comment
  approval-gated

## Files you may touch

NEW crate at `crates/argo-mcp-connector-asana/` mirroring
the file layout of `argo-mcp-connector-slack`:
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

## Acceptance criteria
- [ ] All 8 methods callable
- [ ] Bearer auth via `ASANA_PAT`
- [ ] Approval-gate marks correct
- [ ] 4 unit tests + 1 wire test
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
      clean

## Branch + PR
- Branch: `codex-connector-asana`
- PR title: `mcp: native Asana connector (codex-connector-asana)`
