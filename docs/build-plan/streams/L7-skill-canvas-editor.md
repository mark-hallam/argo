# L7-skill-canvas-editor — Track L Phase 2 (Editor + diff), Tier 3

**Owner**: claude. **Depends on**: L4 MERGED; L6 MERGED.
**Authored against**: origin/main `36bf9984` on `2026-05-14`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh L7-skill-canvas-editor`.

## Scope

Edit mode for the canvas. Operators drag from a palette, reroute edges, delete nodes. Save creates new IR version → existing AB-promote schedules eval. Per-node policy toggle (auto vs ask). Tool-binding picker per action node (lists MCP tools available to the tenant). Topology validation: graph must have entry + at least one terminal; no orphaned nodes.

LOC: ~1400. Branch: `claude-L7-skill-canvas-editor`. PR title: `feat(web/canvas): editor mode + save → AB-promote (L7)`.

## Files you may touch

-  — TOUCH. Add mode toggle (read / edit).
- NEW .
- NEW  — draggable source.
- NEW .
- NEW  — .
-  — TOUCH. Add .
- NEW  — save → validate → hash-chain → AB-test schedule.

## Files you must NOT touch

- L1 schema, L4 read-only viewer core, existing  runtime (use its API, don't refactor).
- Anything outside the brief's may-touch set per CLAUDE.md.

## Acceptance criteria

- [ ] Insert / delete / reroute work.
- [ ] Save rejects invalid topology with clear error.
- [ ] AB-test auto-scheduled after save.
- [ ] Edit history visible (last 5 versions).
- [ ] 10 unit tests + 1 Playwright e2e.
- [ ] No file > 500 LOC.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean (Rust-side briefs).
- [ ] `pnpm typecheck` clean (web-side briefs).
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-L7-skill-canvas-editor`
- PR title: `feat(web/canvas): editor mode + save → AB-promote (L7)`

## Notes for the implementer

- See `streams/L0-skill-canvas-track-overview.md` §L7-skill-canvas-editor for the authoritative spec.
- Mission-claim alignment: claim 4 (Skills are shared and learned).
