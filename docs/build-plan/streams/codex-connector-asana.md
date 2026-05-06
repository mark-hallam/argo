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
- `crates/argo-mcp/src/connectors/asana.rs` (NEW, ~400 LOC
  cap)
- `crates/argo-mcp/src/connectors/mod.rs` — narrowly: add
  `asana` module
- `crates/argo-cli/src/handlers/connector_asana.rs` (NEW)
- `crates/argo-cli/src/cli_connector_cmds.rs` — narrowly:
  `Asana` subcommand variant
- BLANKET CLI ALLOWLIST applies

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
