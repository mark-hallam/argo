# V1.2 — Loom MCP connector (read-mostly)

**Owner**: codex. **Depends on**: C-mcp-wire (#806) merged.

## Why

Native Loom surface so agents can list videos, fetch metadata
+ transcripts and inspect workspace membership. Mirrors
zendesk/discord pattern.

## Scope

Loom connector. Auth via API token (`LOOM_API_TOKEN`) against
the Loom REST API.

- Methods: `list_videos`, `get_video`, `get_video_transcript`,
  `list_workspace_members`
- **All read-only** — every method autonomous-safe.

## Approach

REST client wrapping `reqwest` with `Authorization: Bearer
<token>` header. `client.rs` exposes one method per public
surface; `service.rs` marks every method autonomous-safe.

## Files you may touch

NEW crate at `crates/argo-mcp-connector-loom/` mirroring
`argo-mcp-connector-zendesk`:
- `crates/argo-mcp-connector-loom/Cargo.toml` (NEW)
- `crates/argo-mcp-connector-loom/src/lib.rs` (NEW)
- `crates/argo-mcp-connector-loom/src/main.rs` (NEW)
- `crates/argo-mcp-connector-loom/src/auth.rs` (NEW)
- `crates/argo-mcp-connector-loom/src/client.rs` (NEW)
- `crates/argo-mcp-connector-loom/src/schemas.rs` (NEW)
- `crates/argo-mcp-connector-loom/src/service.rs` (NEW)
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

- [ ] All 4 methods callable end-to-end
- [ ] Bearer auth via `LOOM_API_TOKEN`
- [ ] All methods marked autonomous-safe (read-only)
- [ ] Transcript returned as captions array (do not
      flatten/concatenate)
- [ ] 4–8 unit tests + 1 wire test against a recorded fixture
- [ ] Each `src/*.rs` ≤ 500 LOC (HARD RULE 6)
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
      clean

## Branch + PR

- Branch: `codex-connector-loom`
- PR title: `mcp: native Loom connector (codex-connector-loom)`

## Notes

- ~700 LOC total across 6 src files plus Cargo.toml.
- Loom's transcript endpoint returns timed captions — keep
  the `start_ms`/`end_ms`/`text` shape; downstream agents
  will want to seek by timestamp.
- This connector is intentionally minimal — no upload,
  share-link, or comment-write methods in v1.
