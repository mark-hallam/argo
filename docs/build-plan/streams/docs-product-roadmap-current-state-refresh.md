# docs-product-roadmap-current-state-refresh

**Owner**: codex.
**Depends on**: none.
**Authored against**: origin/main `a9675bad2` on `2026-05-24`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh docs-product-roadmap-current-state-refresh`.

P3 docs drift. `PRODUCT_ROADMAP.md` was authored against 2026-05-11
state. Since then the web/product surface has moved through many
design-system and demo-SWOT passes, while the live ledger now carries
the operative page-port and polish briefs.

## Scope

Refresh `docs/build-plan/PRODUCT_ROADMAP.md` as a current roadmap
index. The goal is to reduce stale instructions for future web/UX
workers: state which parts are historical, point to the current
operator-locked design explorations and live `web-*` stream rows, and
preserve the rule that design-language/roadmap decisions are
operator-driven when the ledger header says so.

This is documentation-only. Do not change the web app or author new
frontend briefs in this PR.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- `docs/build-plan/PRODUCT_ROADMAP.md` - refresh status and handoff pointers only.

## Files you must NOT touch

- `web/**` - no frontend implementation in this doc refresh.
- `docs/build-plan/BUILD_STATUS.md` except the standard PR-number row update.
- `docs/skycot/design-explorations/**` - source material only, no edits.
- `docs/build-plan/MISSION_BACKLOG.md` - over the 500-LOC budget and not part of this brief.
- Anything not on the may-touch list above.

## Acceptance criteria

- [ ] The top status reflects 2026-05-24 and says `BUILD_STATUS.md` is the live queue.
- [ ] Tracks C-E reference the current design-exploration and web-port stream reality without claiming stale "not started" state.
- [ ] The doc tells web/UX workers to check the ledger header before starting new frontend work.
- [ ] Product-surface backend fill-in sections point at live mission/gap stream rows rather than old advisory files as the operative source.
- [ ] No web source, CSS, or design asset is modified.
- [ ] `rg -n 'BUILD_STATUS.md|design-explorations|web-port|demo SWOT|ledger header' docs/build-plan/PRODUCT_ROADMAP.md` shows the refreshed pointers.
- [ ] `bash scripts/check-brief-freshness.sh docs-product-roadmap-current-state-refresh` passes.
- [ ] Every modified file remains under 500 LOC.

## Branch + PR

- Branch: `codex-docs-product-roadmap-current-state-refresh`
- PR title: `docs(product-roadmap): refresh current web roadmap handoff (docs-product-roadmap-current-state-refresh)`

## Notes for the implementer

- Use the ledger and current docs as sources. Avoid memory-only claims.
- If you find a substantive planning conflict, document it in the PR body rather than resolving it by editing unrelated files.
