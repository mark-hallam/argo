# V1.2 — Monday.com MCP connector

**Owner**: codex. **Depends on**: C-mcp-wire (#806) merged.

## Scope

Monday.com connector. Auth via API token (`MONDAY_API_TOKEN`).

- Methods: `list_boards`, `list_items` (board-scoped),
  `get_item`, `create_item`, `update_item_status`,
  `add_update` (comment), `list_groups`
- Read autonomous-safe; create/update/comment approval-gated

## Files you may touch

NEW crate at `crates/argo-mcp-connector-monday/` mirroring
`argo-mcp-connector-asana` layout:
- `crates/argo-mcp-connector-monday/Cargo.toml` (NEW)
- `crates/argo-mcp-connector-monday/src/{lib,main,auth,client,schemas,service}.rs` (NEW)
- `Cargo.toml` workspace — narrowly: add new member

NO CLI subcommand.

## Acceptance criteria
- [x] All 7 methods callable
- [x] API-token bearer auth
- [x] Approval-gate marks correct
- [x] 4 unit tests + 1 wire test
- [x] `cargo clippy --workspace --all-targets -- -D warnings` clean

## Branch + PR
- Branch: `codex-connector-monday`
- PR title: `mcp: native Monday.com connector (codex-connector-monday)`
