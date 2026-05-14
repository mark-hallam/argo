# L17-skill-canvas-e2e — Track L Phase 4 (Composition + library + e2e), Tier 5

**Owner**: claude. **Depends on**: L13 MERGED.
**Authored against**: origin/main `36bf9984` on `2026-05-14`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh L17-skill-canvas-e2e`.

## Scope

Full Playwright e2e scenarios covering canvas happy paths: view → edit → save → AB-promote; Ghostwriter → canvas → refine → save; CU-recording → IR → review → adopt; sub-skill drill-down; suggestion apply.

LOC: ~700. Branch: `claude-L17-skill-canvas-e2e`. PR title: `test(web/canvas): full e2e Playwright suite (L17)`.

## Files you may touch

- NEW .
- NEW .

## Files you must NOT touch

- Any production code; this is test-only.
- Anything outside the brief's may-touch set per CLAUDE.md.

## Acceptance criteria

- [ ] Five Playwright scenarios green.
- [ ] Runs in deploy-smoke workflow (K5).
- [ ] No file > 500 LOC.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean (Rust-side briefs).
- [ ] `pnpm typecheck` clean (web-side briefs).
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-L17-skill-canvas-e2e`
- PR title: `test(web/canvas): full e2e Playwright suite (L17)`

## Notes for the implementer

- See `streams/L0-skill-canvas-track-overview.md` §L17-skill-canvas-e2e for the authoritative spec.
- Mission-claim alignment: claim 4 (Skills are shared and learned).
