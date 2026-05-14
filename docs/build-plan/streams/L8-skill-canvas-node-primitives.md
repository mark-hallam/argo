# L8-skill-canvas-node-primitives — Track L Phase 2 (Editor + diff), Tier 3

**Owner**: claude. **Depends on**: L7 MERGED.
**Authored against**: origin/main `36bf9984` on `2026-05-14`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh L8-skill-canvas-node-primitives`.

## Scope

9 distinct React components — one per IR node kind. Each renders the node body + an inspector panel for editing its config. Inspector forms validate against L1 schema.

LOC: ~800. Branch: `claude-L8-skill-canvas-node-primitives`. PR title: `feat(web/canvas): 9 node-kind primitives + inspector (L8)`.

## Files you may touch

- NEW  (≤150 LOC).
- NEW .
- NEW .
- NEW .
- NEW .
- NEW .
- NEW .
- NEW .
- NEW .
- NEW  — per-kind form router.

## Files you must NOT touch

- L4 SkillCanvas core renderer; L1 schema.
- Anything outside the brief's may-touch set per CLAUDE.md.

## Acceptance criteria

- [ ] All 9 node kinds render + edit cleanly.
- [ ] Inspector validation surfaces L1 schema errors inline.
- [ ] Storybook-style component test for each (or equivalent).
- [ ] No file > 500 LOC.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean (Rust-side briefs).
- [ ] `pnpm typecheck` clean (web-side briefs).
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-L8-skill-canvas-node-primitives`
- PR title: `feat(web/canvas): 9 node-kind primitives + inspector (L8)`

## Notes for the implementer

- See `streams/L0-skill-canvas-track-overview.md` §L8-skill-canvas-node-primitives for the authoritative spec.
- Mission-claim alignment: claim 4 (Skills are shared and learned).
