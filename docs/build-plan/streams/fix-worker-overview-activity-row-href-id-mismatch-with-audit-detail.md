# fix-worker-overview-activity-row-href-id-mismatch-with-audit-detail

**Owner**:
**Authored against**: origin/main `354de50a0c5fdf561125c7e931699fe94daa326d` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-worker-overview-activity-row-href-id-mismatch-with-audit-detail`.
**Source**: e2e-ux-audit T0739 — `docs/skycot/ux-audits/2026-05-27-0739/scene-1-and-2-claim-1-and-2.md` F10.

🔴 **P0 — Worker Overview "Recent activity" card hrefs use IDs in the form `ap_<name>_N` (e.g. `ap_mariah_1`). The audit-detail lookup table uses `act_01J0<NAME>_NNN` form (e.g. `act_01J0MARIAH_001`). Every click in demo mode falls through to `placeholderAuditDetail()` → "Action detail unavailable" rendered. Scene 2 drill-in from the worker activity card lands on a fabricated placeholder, NOT on the real-shaped audit row the camera expects.**

## Root cause

`web/lib/sample-worker-overview.ts:36` defines the `activity()` helper that constructs each row's `href` as `/audit/action/${id}` where `id` is the row's own id. The Mariah rows at lines 95-99 use IDs `ap_mariah_1`, `ap_mariah_2`, etc.

`web/lib/sample-audit-detail.ts:270-276` defines the lookup: it first checks `SAMPLE_AUDIT_DETAILS` (which uses the `act_01J0MARIAH_001` form at lines 17-23), then falls through to `SAMPLE_AUDIT_ROWS` (which has no `ap_mariah_*` entries — confirmed via grep). When both miss, `placeholderAuditDetail()` fires and the drill-in page renders the literal "Action detail unavailable — drill into another row for full provenance." string.

`web/lib/api/audit-detail.ts:82-84` is the demo-mode fallthrough that calls the placeholder when the lookup misses.

The two sample fixtures were authored independently and never reconciled. The camera will drill in on Scene 2 to verify "the action you just ran lands as a real audit row" — instead it reads a fabrication. Per the function-vs-dressing rule: any row the camera clicks is function-side. This is P0.

## Findings this brief closes

- scene-1-and-2-claim-1-and-2 F10 [P0] — Recent activity hrefs land on the placeholder audit detail ("Action detail unavailable")

## Fix recipe

Two paths; pick whichever is least disruptive to other consumers.

**Path 1 — Rename worker-overview row IDs to match the audit-detail table.**

1. In `web/lib/sample-worker-overview.ts:95-99` (Mariah's rows; same shape for Peta/Priya/Jamal/Nori/Sam), rename IDs:
   - `ap_mariah_1` → `act_01J0MARIAH_001`
   - `ap_mariah_2` → `act_01J0MARIAH_002`
   - etc.
2. Verify each renamed id has a matching entry in `SAMPLE_AUDIT_DETAILS` at `web/lib/sample-audit-detail.ts:17-23` (and the equivalent per-persona blocks). If a persona's audit-detail table has fewer entries than the activity-card needs, extend the audit-detail table to match.
3. Vitest pin: walk every activity-row's href; resolve via `findSampleAuditDetail()`; assert non-placeholder result.

**Path 2 — Add `ap_<name>_N` entries to `SAMPLE_AUDIT_ROWS` so `synthesizeAuditDetail()` constructs them.**

4. In `web/lib/sample-audit-detail.ts`, add per-persona blocks for the `ap_NAME_*` IDs (mirror the existing `act_01J0NAME_NNN` block shape). Each block needs the row's verb, action_type, worker, and any sample provenance.
5. Same pin as Path 1.

**Recommended**: Path 1 if the existing `act_01J0NAME_NNN` entries are richer; Path 2 if the activity-card rows are the canonical source-of-truth and the audit-detail table is the duplicate.

## Acceptance criteria

- [ ] In demo mode, every "Recent activity" row href on `/workers/<id>/overview` resolves to a real audit-detail render (NOT the "Action detail unavailable" placeholder).
- [ ] `findSampleAuditDetail()` returns a non-undefined result for every activity-row id used in `SAMPLE_WORKER_OVERVIEWS`.
- [ ] Playwright pin: visit `/workers/<mariah-id>/overview`, click the first recent-activity row, assert the destination renders a non-placeholder action-detail.
- [ ] Vitest pin: walk all rows in `SAMPLE_WORKER_OVERVIEWS`, assert each href resolves to a populated detail row.

## May-touch

- `web/lib/sample-worker-overview.ts` (rename IDs if Path 1)
- `web/lib/sample-audit-detail.ts` (add `ap_NAME_*` entries if Path 2)
- `web/lib/api/audit-detail.ts` (no change expected; fallthrough still fires for unknown ids)
- `web/lib/__tests__/sample-worker-overview.test.ts` (new pin)

## Out of scope

- The placeholder fabrication itself (covered by `fix-placeholder-audit-detail-fabricated-decision-chain-and-provenance.md` — blank the fabricated fields if placeholder fires)
- The audit-feed self-traffic problem (covered by `fix-audit-feed-filter-or-bucket-spa-self-traffic-http-request.md`)
- The "See all in Activity →" redirect (covered by `fix-worker-dynamic-tab-placeholder-strip-brief-id-leak.md`)

## Effort

S — ~50-100 LOC across fixtures + pins. ~1-2 hours. P0 because Scene 2 drill-in lands on this surface.
