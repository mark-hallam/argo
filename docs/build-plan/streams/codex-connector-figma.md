# V1.2 — Figma MCP connector

**Owner**: codex. **Depends on**: C-mcp-wire (#806) merged.

## Scope

Figma connector. Auth via personal access token
(`FIGMA_TOKEN`).

- Methods: `list_files`, `get_file`, `get_file_components`,
  `list_comments`, `add_comment` (approval)
- Read methods autonomous-safe; add_comment approval-gated

## Files you may touch

NEW crate at `crates/argo-mcp-connector-figma/`
mirroring the file layout of `argo-mcp-connector-zendesk`
(#951) and `argo-mcp-connector-discord` (#953):
- `crates/argo-mcp-connector-figma/Cargo.toml` (NEW)
- `crates/argo-mcp-connector-figma/src/lib.rs` (NEW)
- `crates/argo-mcp-connector-figma/src/main.rs` (NEW)
- `crates/argo-mcp-connector-figma/src/auth.rs` (NEW)
- `crates/argo-mcp-connector-figma/src/client.rs` (NEW)
- `crates/argo-mcp-connector-figma/src/schemas.rs` (NEW)
- `crates/argo-mcp-connector-figma/src/service.rs` (NEW)
- `Cargo.toml` (workspace) — narrowly: add the new crate to
  the `members` array

NO CLI subcommand — connectors are invoked via the MCP
manifest's `[mcp_servers]` section.

Cargo.lock regenerates on first build (matches #951/#953
pattern). Every src file ≤ 500 LOC.

## Acceptance criteria
- [ ] All 5 methods callable
- [ ] `add_comment` approval-gate marks correct
- [ ] File-key passthrough on get_file / components / comments
- [ ] 4-8 unit tests + 1 wire test
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
      clean

## Branch + PR
- Branch: `codex-connector-figma`
- PR title: `mcp: native Figma connector
  (codex-connector-figma)`
