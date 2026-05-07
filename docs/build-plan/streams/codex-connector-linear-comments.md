# V1.2 — Linear comments MCP connector

**Owner**: codex. **Depends on**: C-mcp-wire (#806) merged.

## Scope

Linear comments-focused connector (companion to existing
linear-issues connector if present). Auth via API key
(`LINEAR_API_KEY`).

- Methods: `list_comments`, `add_comment`,
  `edit_comment` (approval), `delete_comment` (approval)
- Read + add autonomous-safe; edit/delete approval-gated

## Files you may touch

NEW crate at `crates/argo-mcp-connector-linear-comments/`
mirroring the file layout of `argo-mcp-connector-zendesk`
(#951) and `argo-mcp-connector-discord` (#953):
- `crates/argo-mcp-connector-linear-comments/Cargo.toml`
  (NEW)
- `crates/argo-mcp-connector-linear-comments/src/lib.rs`
  (NEW)
- `crates/argo-mcp-connector-linear-comments/src/main.rs`
  (NEW)
- `crates/argo-mcp-connector-linear-comments/src/auth.rs`
  (NEW)
- `crates/argo-mcp-connector-linear-comments/src/client.rs`
  (NEW)
- `crates/argo-mcp-connector-linear-comments/src/schemas.rs`
  (NEW)
- `crates/argo-mcp-connector-linear-comments/src/service.rs`
  (NEW)
- `Cargo.toml` (workspace) — narrowly: add the new crate to
  the `members` array

NO CLI subcommand — connectors are invoked via the MCP
manifest's `[mcp_servers]` section.

Cargo.lock regenerates on first build (matches #951/#953
pattern). Every src file ≤ 500 LOC.

## Acceptance criteria
- [ ] All 4 methods callable
- [ ] `edit_comment` / `delete_comment` approval-gate
      marks correct
- [ ] GraphQL passthrough working (Linear is GraphQL-only)
- [ ] 4-8 unit tests + 1 wire test
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
      clean

## Branch + PR
- Branch: `codex-connector-linear-comments`
- PR title: `mcp: native Linear comments connector
  (codex-connector-linear-comments)`
