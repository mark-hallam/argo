# web-seed-activity-id-scheme-align — align worker-overview activity hrefs to canonical audit IDs

**Owner**: claude.
**Authored against**: origin/main `50e65e9f` on `2026-05-21`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-seed-activity-id-scheme-align`.
**Priority**: P2 — wave-4 data-integrity: worker-overview activity links use a third ID scheme (`ap_priya_1`) that exists in NEITHER the audit feed nor the audit detail. Click → 404.
**Sizing**: ~80-150 LOC (seed refactor + tests).

## Why this brief exists

From the wave-4 data-integrity audit:

> Worker-overview activity links 404: rows have hrefs like
> `/audit/action/ap_priya_1` — a third ID scheme that exists in
> neither the audit feed nor the audit detail.

Wave-4 fix aligned the audit-feed IDs (uppercase `act_01J0PRIYA_NNN`)
with the hand-crafted detail IDs. This brief brings the worker-overview
activity hrefs into the same scheme.

## Goals

1. Refactor `web/lib/sample-worker-activity.ts` to emit hrefs matching the canonical `act_01J0<PERSONA>_NNN` scheme.
2. Each worker-overview activity row links to a real audit-feed row (drill-in works).
3. Add integrity test: every activity href must resolve to an existing audit row.

## Steps

1. Read current sample-worker-activity.ts.
2. For each activity entry, generate the href as `/audit/action/act_01J0${persona.toUpperCase()}_${index.padStart(3,"0")}`.
3. Add an integrity test in `web/lib/__tests__/activity-link-integrity.test.ts`.
4. Smoke test: click any worker-overview activity row in dev — drill-in lands on a real detail page.

## may-touch

- `web/lib/sample-worker-activity.ts`
- New: `web/lib/__tests__/activity-link-integrity.test.ts`

## must-NOT-touch

- Audit-feed ID scheme (already canonical post wave-4)
- Audit-detail records (already canonical post wave-4)
- Activity row content (only fix hrefs)

## Acceptance criteria

- [ ] Every activity href on /workers/[id]/overview resolves to an existing audit row.
- [ ] Activity-link-integrity test passes.
- [ ] `pnpm typecheck && pnpm lint && pnpm test` clean.

## References

- Audit: `docs/skycot/audit/data-integrity/2026-05-20-wave-4-data-integrity.md`
- Wave-4 fix: commit `d04a7032` (audit ID alignment)
