# V1.2 — Fireflies.ai MCP connector

**Owner**: codex. **Depends on**: C-mcp-wire (#806) merged.

## Scope

Fireflies.ai connector for meeting transcripts. Auth via API
key (`FIREFLIES_API_KEY`).

- Methods: `list_meetings` (date filter), `get_meeting`,
  `get_transcript`, `search_transcripts` (keyword), `list_users`
- Read-only connector — all methods autonomous-safe (no
  state-mutating endpoints exposed v1)

## Files you may touch

NEW crate at `crates/argo-mcp-connector-fireflies/` mirroring
`argo-mcp-connector-asana`:
- `crates/argo-mcp-connector-fireflies/Cargo.toml` (NEW)
- `crates/argo-mcp-connector-fireflies/src/{lib,main,auth,client,schemas,service}.rs` (NEW)
- `Cargo.toml` workspace — narrowly: add new member

NO CLI subcommand.

## Acceptance criteria
- [ ] All 5 methods callable
- [ ] GraphQL or REST passthrough on `search_transcripts`
- [ ] All methods marked `approval_required: false`
- [ ] 4 unit tests + 1 wire test
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean

## Branch + PR
- Branch: `codex-connector-fireflies`
- PR title: `mcp: native Fireflies.ai connector (codex-connector-fireflies)`
