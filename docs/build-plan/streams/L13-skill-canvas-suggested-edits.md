# L13-skill-canvas-suggested-edits — Track L Phase 3 (Observation extraction), Tier 4

**Owner**: claude. **Depends on**: L11 MERGED; L12 MERGED.
**Authored against**: origin/main `36bf9984` on `2026-05-14`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh L13-skill-canvas-suggested-edits`.

## Scope

Heuristics that detect low-confidence repeated-failure patterns. Suggested edits are templated patterns: 'insert approval gate before low-confidence action,' 'split branch into two paths by observed outcome,' etc. Banner on canvas shows top 3 suggestions; one-click apply creates new version + schedules AB-promote.

LOC: ~900. Branch: `claude-L13-skill-canvas-suggested-edits`. PR title: `feat(web/canvas): low-confidence suggested-edits banner (L13)`.

## Files you may touch

- NEW  — heuristics.
- NEW .
- NEW  — endpoint.
- NEW .
- NEW .

## Files you must NOT touch

- L1 schema; L4 viewer core; L12 telemetry write path (read-only).
- Anything outside the brief's may-touch set per CLAUDE.md.

## Acceptance criteria

- [ ] Engine generates ≥ 3 suggestion kinds.
- [ ] Suggestions only surface for skills past the sample-count threshold (>10 samples).
- [ ] Apply creates valid new IR version.
- [ ] 6 unit tests.
- [ ] No file > 500 LOC.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean (Rust-side briefs).
- [ ] `pnpm typecheck` clean (web-side briefs).
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-L13-skill-canvas-suggested-edits`
- PR title: `feat(web/canvas): low-confidence suggested-edits banner (L13)`

## Notes for the implementer

- See `streams/L0-skill-canvas-track-overview.md` §L13-skill-canvas-suggested-edits for the authoritative spec.
- Mission-claim alignment: claim 4 (Skills are shared and learned).
