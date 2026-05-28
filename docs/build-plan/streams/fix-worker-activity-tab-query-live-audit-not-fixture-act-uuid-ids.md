# fix-worker-activity-tab-query-live-audit-not-fixture-act-uuid-ids

**Severity**: 🔴 P0 — MVP demo Scene 2 blocker

**Authored against**: `origin/main` SHA `58ea1c93f` on 2026-05-28T03:55Z.

**Discovered during**: 2026-05-28 e2e walkthrough — `docs/skycot/demos/walkthroughs/2026-05-28-e2e-dry-run-with-operator.md` Scene 2.

## Symptom

`/workers/<id>/activity` page shows 2 rows with action-IDs in the format `act_018f67d0-0000-7000-8000-000000000003_001` (worker-UUID-derived). Clicking the "escalated refund request to support lead" row navigates to `/audit/action/act_018f67d0-...-001` which returns "**Action not found**" — the canonical Claim-2 delegation-drill-in moment dead-ends on a 404.

## Why PR #2464 didn't close this

`fix-worker-overview-activity-row-href-id-mismatch-with-audit-detail` (#2464, MERGED 2026-05-27) renamed activity-card IDs in `web/lib/sample-worker-overview.ts` to `act_01J0<NAME>_011..._015` AND added matching `AuditDetail` entries in `web/lib/sample-audit-detail.ts`. That fix closed the **Overview tab's** activity card drill-in.

The **full Activity tab** at `/workers/<id>/activity` is a separate code path powered by `web/lib/sample-worker-activity.ts`. Its `build()` function at line 185-188 generates IDs dynamically:

```ts
function build(workerId: string, seeds: Seed[]): WorkerActivity[] {
  const persona = getSamplePersona(workerId);
  return seeds.map((seed, i) => ({
    id: `act_${workerId}_${i.toString().padStart(3, "0")}`,
    ...
```

These IDs use the literal worker UUID, NOT the canonical `act_01J0<NAME>_NNN` format. They have no matching entry in `sample-audit-detail.ts` (which keys off the canonical format). Result: drill-in 404 on every Activity-tab row.

## Two fix paths

### Path A — extend the fixture pattern (fast, demo-safe)

- Change `sample-worker-activity.ts:188` to use `act_01J0${persona.initials.toUpperCase()}_NNN` format
- Choose `NNN` suffix range that does NOT collide with existing entries in `sample-audit-detail.ts` (#2464 used `_011`-`_015`; choose `_021`-`_030` or similar)
- Add matching `AuditDetail` entries in `sample-audit-detail.ts` for each (persona × seed) — there are 5 personas × ~5 seeds = ~25 new entries
- Verify the drill-in lands on a real C2 card

### Path B — query the live audit feed (real fix, larger)

- Change `sample-worker-activity.ts` to NOT serve fixture data
- Replace with a fetcher that calls `GET /api/v1/audit?worker=<worker_id>&limit=25`
- Render the live audit-feed rows in the Activity tab
- Drill-in URLs use the real audit-row IDs (which are real hex blobs in the audit store)
- Requires the audit drill-in 404 fix to also be in place (see companion brief `fix-audit-detail-real-id-tenant-scope-drift-not-fully-closed-by-2589`)

**Recommendation**: ship Path A for the demo recording (this brief). Path B is the post-MVP follow-up — same shape as the operator's NO-THEATER rule pushes toward, but blocked on the audit drill-in real-ID fix.

## Acceptance criteria

For Path A:

- [ ] `/workers/<jamal-uuid>/activity` shows 2-5 activity rows
- [ ] Clicking any row navigates to `/audit/action/<id>` where `<id>` matches the canonical `act_01J0<NAME>_NNN` format
- [ ] The drill-in page renders a real `AuditDetail` (predecessor + payload + followUp chain) — NOT "Action not found"
- [ ] All 5 canonical personas (Mariah / Peta / Priya / Jamal / Nori) have matching audit-detail entries for every fixture activity-row
- [ ] Existing `sample-worker-overview.ts` activity-card IDs (the `_011`-`_015` range from #2464) still resolve

## May-touch

- `web/lib/sample-worker-activity.ts` (the build() function, line 185+)
- `web/lib/sample-audit-detail.ts` (add ~25 new `AuditDetail` entries)
- `web/lib/__tests__/sample-worker-overview.test.ts` (extend pin to also cover Activity-tab IDs)
- new vitest: `web/lib/__tests__/sample-worker-activity-ids-resolve.test.ts`

## Must-not-touch

- The Activity tab page component itself — only swap the data source
- The `WorkerActivity` type shape

## References

- Operator NO-THEATER directive: `feedback_no_theater_in_demo`
- PR #2464 (related but incomplete fix): `fix-worker-overview-activity-row-href-id-mismatch-with-audit-detail`
- PR #2603 IN_REVIEW (DIRTY): `fix-audit-feed-action-kind-variety` — once that lands and the audit drill-in 404 closes, Path B becomes the right fix
- Walkthrough source: `docs/skycot/demos/walkthroughs/2026-05-28-e2e-dry-run-with-operator.md` Scene 2
