# web-seed-cohorts-phantom-and-mariah-membership — fix phantom worker + Mariah's missing cohort

**Owner**: claude.
**Authored against**: origin/main `50e65e9f` on `2026-05-21`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-seed-cohorts-phantom-and-mariah-membership`.
**Priority**: P2 — wave-4 data-integrity: cohorts references phantom worker `wkr_01J0MAYA` (not seeded); Mariah is missing from her own cohort.
**Sizing**: ~80-150 LOC (seed cleanup + cohorts integrity test).

## Why this brief exists

From the wave-4 data-integrity audit:

> Other findings: cohorts references phantom worker `wkr_01J0MAYA`
> (not seeded anywhere); Mariah is missing from her own cohort.

Per the GO/NO-GO verdict the operator skips /cohorts on the demo
record, but the data integrity should still be clean for future
recordings + procurement-tour clicks.

## Goals

1. Remove all references to `wkr_01J0MAYA` (phantom worker — never seeded).
2. Add Mariah to her own cohort (whichever cohort tile features her vertical).
3. Add a cohorts-integrity test: every worker referenced in cohorts.ts must exist in sample-team.ts.

## Steps

1. `rg "wkr_01J0MAYA" web/` → find every reference; remove or replace with a real worker.
2. Open `web/lib/sample-cohorts.ts` + verify Mariah's membership in the marketplace/sales cohort.
3. Add the integrity test: read sample-team.ts + sample-cohorts.ts, assert every cohort.workers[i] exists in team.

## may-touch

- `web/lib/sample-cohorts.ts`
- `web/lib/sample-team.ts` (only if a new field needs adding)
- New: `web/lib/__tests__/cohorts-integrity.test.ts`

## must-NOT-touch

- Worker IDs in sample-team.ts (stable)
- /cohorts page rendering (only fix data)

## Acceptance criteria

- [ ] Zero references to `wkr_01J0MAYA` remain.
- [ ] Mariah appears in her own cohort.
- [ ] Integrity test passes: every cohort-referenced worker exists in the team.
- [ ] `pnpm typecheck && pnpm lint && pnpm test` clean.

## References

- Audit: `docs/skycot/audit/data-integrity/2026-05-20-wave-4-data-integrity.md`
