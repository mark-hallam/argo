# web-seed-temporal-consistency — fix date-ordering bugs (Sara Lin, Rio, Priya, Sam)

**Owner**: claude.
**Authored against**: origin/main `50e65e9f` on `2026-05-21`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-seed-temporal-consistency`.
**Priority**: P2 — wave-4 data-integrity: 4 temporal bugs in seed data (Sara Lin contradictory states; Sam invoice reversed causality; Rio + Priya past-date references that have now-passed).
**Sizing**: ~100-200 LOC (date refactor to use relative-from-now offsets + tests).

## Why this brief exists

From the wave-4 data-integrity audit:

> Sara Lin is both "moved to backup pool 10 days ago" and pending a
> rejection today; Jamal's SUP-29382 is both resolved and paused;
> Rio "paused on May 7" + Priya's interview "proposed Thursday
> May 14" are now in the past (today is 2026-05-20).
>
> Sam's invoice flow has reversed causality — extraction 6 min ago,
> CFO approval queued 1 h 32 m ago for the same invoice.

The seed data has hardcoded absolute dates that "go stale" over time
and causality bugs that read as broken on camera.

## Goals

1. Replace hardcoded absolute dates with relative-from-NOW offsets so the demo always shows fresh dates.
2. Fix Sam invoice causality: extraction must come BEFORE approval (not after).
3. Resolve Sara Lin contradiction (pick one state).
4. Resolve Jamal SUP-29382 contradiction.
5. Add a temporal-integrity test: for each "action chain" (predecessor → action → followup), assert the timestamps are in order.

## Steps

1. `rg "May 7|May 14|May 20" web/lib/sample-*.ts` → find hardcoded dates.
2. Convert each to `new Date(Date.now() - N * 24 * 60 * 60 * 1000).toISOString()` style.
3. Fix Sam invoice: extraction agoMs=92 min → approval agoMs=6 min (or vice versa — whichever is realistic).
4. Pick one state for Sara Lin + Jamal SUP-29382.
5. Add `web/lib/__tests__/temporal-integrity.test.ts`.

## may-touch

- `web/lib/sample-audit.ts`
- `web/lib/sample-replay.ts`
- `web/lib/sample-worker-activity.ts`
- `web/lib/sample-audit-detail.ts`
- Other sample-*.ts files with absolute dates
- New: temporal-integrity test

## must-NOT-touch

- Worker IDs / personality / role (handled by sibling brief)
- Action verbs / hashes (only fix timestamps + ordering)

## Acceptance criteria

- [ ] Zero hardcoded "May NN" or specific past-date strings remain.
- [ ] Sam's invoice extraction happens before CFO approval queue (timeline-correct).
- [ ] Sara Lin has one consistent state.
- [ ] Jamal SUP-29382 has one consistent state.
- [ ] Temporal-integrity test passes.
- [ ] `pnpm typecheck && pnpm lint && pnpm test` clean.

## References

- Audit: `docs/skycot/audit/data-integrity/2026-05-20-wave-4-data-integrity.md`
