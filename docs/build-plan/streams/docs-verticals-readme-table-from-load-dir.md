# docs-verticals-readme-table-from-load-dir

**Owner**: codex
**Priority**: P3 - docs drift
**Authored against**: origin/main `7354f039d` on `2026-05-24`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh docs-verticals-readme-table-from-load-dir`.

## Source

The mission-vs-code gap manifest flagged a vertical inventory drift: `verticals/README.md`
lists far fewer entries than `argo_vertical::load_dir` accepts from the `verticals/`
tree. The previous ledger row was blocked only because this stream file was missing.

## Goal

Make the vertical README table derive from the same loadable-directory truth that
runtime code uses, and add a small guard so the table does not silently drift again.

## May Touch

- `verticals/README.md`
- `verticals/` fixture metadata only when required for a table source field
- `scripts/` (one narrow docs-table regeneration/check script)
- `docs/build-plan/BUILD_STATUS.md`

## Must Not Touch

- Do not change runtime vertical loader behavior.
- Do not edit `crates/argo-vertical/src/**` unless a tiny test hook is unavoidable.
- Do not archive, add, or redesign verticals in this brief.

## Acceptance

- [ ] README inventory count matches the directories loadable by the existing vertical loader.
- [ ] A repeatable command regenerates or checks the table.
- [ ] The table includes enough source fields to explain why each row is listed.
- [ ] The PR body reports any directories intentionally excluded and why.
- [ ] `git diff --check` passes, plus the narrow docs-table check command passes.

## Notes

Fast-mode candidate. This is a docs/guard repair, not a vertical content rewrite.
