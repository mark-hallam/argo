# L10-skill-canvas-export-import — Track L Phase 2 (Editor + diff), Tier 3

**Owner**: claude. **Depends on**: L7 MERGED.
**Authored against**: origin/main `36bf9984` on `2026-05-14`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh L10-skill-canvas-export-import`.

## Scope

Export canvas as  (canonical IR + metadata). Import a , validate, preview diff against existing, optional adopt. Portable across tenants (tenant-id stripped on export).

LOC: ~500. Branch: `claude-L10-skill-canvas-export-import`. PR title: `feat(web/canvas): export/import skill IR as JSON (L10)`.

## Files you may touch

- NEW .
- NEW .
- NEW  — endpoint.
- NEW  — bundle format.

## Files you must NOT touch

- L1 schema, L7 editor.
- Anything outside the brief's may-touch set per CLAUDE.md.

## Acceptance criteria

- [ ] Round-trip: export → import produces identical hash.
- [ ] Bad files surface validation errors.
- [ ] No file > 500 LOC.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean (Rust-side briefs).
- [ ] `pnpm typecheck` clean (web-side briefs).
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-L10-skill-canvas-export-import`
- PR title: `feat(web/canvas): export/import skill IR as JSON (L10)`

## Notes for the implementer

- See `streams/L0-skill-canvas-track-overview.md` §L10-skill-canvas-export-import for the authoritative spec.
- Mission-claim alignment: claim 4 (Skills are shared and learned).
