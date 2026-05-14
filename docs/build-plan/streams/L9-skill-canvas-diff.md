# L9-skill-canvas-diff — Track L Phase 2 (Editor + diff), Tier 3

**Owner**: claude. **Depends on**: L4 MERGED.
**Authored against**: origin/main `36bf9984` on `2026-05-14`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh L9-skill-canvas-diff`.

## Scope

Side-by-side version diff view. Compare two IR versions: nodes added/removed/changed; edges rewired; annotations shifted. Side-by-side rendering with red/green/yellow highlights. Header summary: "3 nodes added, 1 removed, 2 modified." Reachable from canvas via version-picker dropdown.

LOC: ~800. Branch: `claude-L9-skill-canvas-diff`. PR title: `feat(web/canvas): version diff view v(n-1) vs v(n) (L9)`.

## Files you may touch

- NEW .
- NEW  — side-by-side renderer.
- NEW  — graph-diff algorithm.
- NEW .

## Files you must NOT touch

- L4 read-only viewer; L1 schema.
- Anything outside the brief's may-touch set per CLAUDE.md.

## Acceptance criteria

- [ ] Diff engine: 8 unit tests covering all change classes.
- [ ] Visual rendering: red/green/yellow on the right side, gray on the left.
- [ ] Performance: < 200ms for 100-node graphs.
- [ ] No file > 500 LOC.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean (Rust-side briefs).
- [ ] `pnpm typecheck` clean (web-side briefs).
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-L9-skill-canvas-diff`
- PR title: `feat(web/canvas): version diff view v(n-1) vs v(n) (L9)`

## Notes for the implementer

- See `streams/L0-skill-canvas-track-overview.md` §L9-skill-canvas-diff for the authoritative spec.
- Mission-claim alignment: claim 4 (Skills are shared and learned).
