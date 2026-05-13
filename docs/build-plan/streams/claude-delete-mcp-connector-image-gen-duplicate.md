# claude-delete-mcp-connector-image-gen-duplicate — delete dead duplicate of canonical argo-mcp-image-gen

**Owner**: claude. **Depends on**: nothing.
**Authored against**: origin/main `6e3e31d8` on `2026-05-14`.

> Audit cluster 4 found TWO image-gen MCP server crates: `argo-mcp-image-gen` (wired, approval-gated, in argo-cli deps) AND `argo-mcp-connector-image-gen` (newer 3-backend connector with NO workspace consumer). Both define an `image.generate` MCP tool. The second is a parallel that nothing uses.

## Scope

Delete `crates/argo-mcp-connector-image-gen/` entirely + remove from workspace.

Verify the canonical `argo-mcp-image-gen` (different crate name; not the same hyphenation) stays untouched.

## Files you may touch
- `crates/argo-mcp-connector-image-gen/` — `git rm -r`.
- `Cargo.toml` (workspace) — remove the entry.

## Files you must NOT touch
- `crates/argo-mcp-image-gen/` — canonical; keep.

## Acceptance
- [ ] Directory removed; workspace member removed.
- [ ] `cargo build --workspace` clean.
- [ ] `argo mcp tools` still lists image-gen (via argo-mcp-image-gen).
- [ ] Net LOC ~-150.

## Branch
- `claude-delete-mcp-connector-image-gen-duplicate`
