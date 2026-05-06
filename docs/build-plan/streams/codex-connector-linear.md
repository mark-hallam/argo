# V1.2 — Linear MCP connector

**Owner**: codex. **Depends on**: C-mcp-wire (#806) merged.

## Scope

Linear connector. Auth via API key (`LINEAR_API_KEY`).

- Methods: `list_teams`, `list_issues` (filter passthrough),
  `get_issue`, `create_issue`, `update_issue`, `add_comment`,
  `list_workflows`
- Read autonomous-safe; create/update/comment approval-gated
- GraphQL passthrough on `list_issues` filter

## Files you may touch

NEW crate at `crates/argo-mcp-connector-linear/` mirroring
the file layout of `argo-mcp-connector-asana`:
- `crates/argo-mcp-connector-linear/Cargo.toml` (NEW)
- `crates/argo-mcp-connector-linear/src/lib.rs` (NEW)
- `crates/argo-mcp-connector-linear/src/main.rs` (NEW)
- `crates/argo-mcp-connector-linear/src/auth.rs` (NEW)
- `crates/argo-mcp-connector-linear/src/client.rs` (NEW)
- `crates/argo-mcp-connector-linear/src/schemas.rs` (NEW)
- `crates/argo-mcp-connector-linear/src/service.rs` (NEW)
- `Cargo.toml` (workspace) — narrowly: add new member

NO CLI subcommand — connectors invoked via MCP manifest.

## Acceptance criteria
- [ ] All 7 methods callable
- [ ] GraphQL filter passthrough on `list_issues`
- [ ] API-key bearer auth
- [ ] Approval-gate marks correct
- [ ] 4 unit tests + 1 wire test
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean

## Branch + PR
- Branch: `codex-connector-linear`
- PR title: `mcp: native Linear connector (codex-connector-linear)`
