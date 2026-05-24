# docs-mission-backlog-live-tag-sweep

**Owner**: codex.
**Depends on**: none.
**Authored against**: origin/main `9aded3f9a` on `2026-05-24`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh docs-mission-backlog-live-tag-sweep`.

P3 docs hygiene. `docs/skycot/operator/known-stale-docs-2026-05-23.md`
calls out `docs/build-plan/MISSION_BACKLOG.md` as likely drifted and
specifically recommends a tag sweep against current main. New agents use
those tags for replenishment, so stale `[NEW]`/`[BRIEF]`/`[SHIPPED]`
markers create duplicate or dead brief ideas.

## Scope

Refresh `MISSION_BACKLOG.md` tags against the live ledger and recent
main-branch history. Update only rows where the current status is clear
from `BUILD_STATUS.md` and git. Leave ambiguous items unchanged rather
than guessing. This is a documentation/status sweep only; do not author
new feature briefs as part of this doc brief.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: `docs/build-plan/MISSION_BACKLOG.md`

## Files you may touch

- `docs/build-plan/MISSION_BACKLOG.md`

## Files you must NOT touch

- `docs/build-plan/BUILD_STATUS.md` except the standard PR-number row update.
- `docs/build-plan/streams/**`
- `docs/skycot/operator/known-stale-docs-2026-05-23.md`
- Anything not on the may-touch list above.

## Acceptance criteria

- [ ] Every changed tag in `MISSION_BACKLOG.md` is justified by a current `BUILD_STATUS.md` row or a merged-on-main outcome.
- [ ] Rows with unclear status are left untouched instead of being guessed into `[SHIPPED]` or `[BRIEF]`.
- [ ] The reconciliation note at the bottom is updated with a fresh date/sha summary of what changed this pass.
- [ ] No narrative scope creep: this brief edits tags/reconciliation notes only, not the substantive backlog prose.
- [ ] `bash scripts/check-brief-freshness.sh docs-mission-backlog-live-tag-sweep` passes.
- [ ] Every modified file remains under 500 LOC.

## Branch + PR

- Branch: `codex-docs-mission-backlog-live-tag-sweep`
- PR title: `docs(backlog): refresh mission backlog tags against live ledger (docs-mission-backlog-live-tag-sweep)`

## Notes for the implementer

- Use `BUILD_STATUS.md` as ground truth when a stream file still exists with stale wording.
- Keep the note concise: counts changed, source sha, and any intentionally-left-uncertain cases.
