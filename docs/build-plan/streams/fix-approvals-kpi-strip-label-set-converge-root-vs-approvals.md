# fix-approvals-kpi-strip-label-set-converge-root-vs-approvals

**Owner**: claude or codex.
**Authored against**: origin/main `2cc2ec42b` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-approvals-kpi-strip-label-set-converge-root-vs-approvals`.
**Source**: ux-e2e-audit-T2309MEL `scene-6-claim-6-trust-and-approvals.md` F8 [P1].

🟠 **P1 — Root canonical inbox (`/`) and `/approvals` standalone landing render different KPI label sets, both claiming to be the 5-KPI approval strip. `/` shows `PENDING / RESOLVED / WK / AVG RESPONSE / $ AT RISK / APPROVAL RATE`. `/approvals` shows `PENDING / $ AT RISK / MEDIAN TIME / APPROVAL RATE / ACTIVE REVIEWERS`. Different labels, different positions for `$ AT RISK`. The locked `project_approvals_page_design_locked` memo specifies "5-KPI strip" but doesn't name the 5 — both pages comply with the spec while disagreeing with each other.**

## Root cause

Two pages render approvals KPIs through different code paths:

1. **`/` (root inbox)** — `web/app/(app)/page.tsx:24` uses `computeKpiCells` from `KpiStrip`. The KpiStrip component picks `{PENDING, RESOLVED/WK, AVG RESPONSE, $ AT RISK, APPROVAL RATE}` as its 5 cells.
2. **`/approvals` (standalone E-ops dashboard)** — `web/components/skycot/approvals/ApprovalsOpsDashboard.tsx` renders `{PENDING, $ AT RISK, MEDIAN TIME, APPROVAL RATE, ACTIVE REVIEWERS}`.

Both routes show 5 cells, but the labels disagree. For a buyer doing a 30-second tour these read as two different products. For Scene 6 narration that says "Mark approves a refund — see the KPI tick up," the operator needs to know which page's KPI tile will respond.

The audit recommends converging on the `/approvals` set (PENDING / $ AT RISK / MEDIAN TIME / APPROVAL RATE / ACTIVE REVIEWERS) because it reads more enterprise/SLA-shaped and matches the locked-design memo better.

This brief converges on a single shared KPI strip and is the natural companion to the existing brief `fix-approvals-page-empty-state-vs-ops-dashboard-route-ambiguity` (which addresses the broader "two pages, one purpose" route question).

## Findings this brief closes

- scene-6-claim-6-trust-and-approvals F8 [P1] — `/` and `/approvals` show inconsistent KPI labels (PENDING / RESOLVED-WK / AVG RESPONSE vs PENDING / MEDIAN TIME / ACTIVE REVIEWERS)

## Fix recipe

The end state depends on the route-converge decision from `fix-approvals-page-empty-state-vs-ops-dashboard-route-ambiguity`. Author the KPI consolidation INDEPENDENTLY of that route decision — both Options A (route-converge) and B (route-segregate) need the same single KPI definition.

1. **Pick the canonical KPI set.** Per audit recommendation + locked design:
   - PENDING (count of pending approvals)
   - $ AT RISK (sum of pending approval dollar values)
   - MEDIAN TIME (median time-to-decision over rolling 7d)
   - APPROVAL RATE (% approved over rolling 7d)
   - ACTIVE REVIEWERS (distinct approver count over rolling 7d)

2. **Extract the strip into a shared component.** `web/components/skycot/approvals/ApprovalsKpiStrip.tsx`:
   ```tsx
   export type ApprovalsKpis = {
     pendingCount: number;
     dollarsAtRisk: number;
     medianTimeMinutes: number;
     approvalRatePercent: number;
     activeReviewersCount: number;
   };
   export function ApprovalsKpiStrip({ kpis }: { kpis: ApprovalsKpis }) { ... }
   ```

3. **Wire to both routes.**
   - `web/app/(app)/page.tsx` swaps its current KpiStrip for `<ApprovalsKpiStrip kpis={kpis} />`
   - `web/components/skycot/approvals/ApprovalsOpsDashboard.tsx` consumes the same component
   - Both routes derive `kpis` from the same `useApprovalsKpis()` hook (write if doesn't exist)

4. **Backend KPI computation.** Verify the gateway endpoint that supplies these. The current `/api/v1/approvals` returns `{items:[]}` only; if the KPI computation lives client-side off the items array, the empty-state KPIs are all 0/—. If a separate `GET /api/v1/approvals/kpis` exists, hit that. If not, compute client-side from items + the audit-derived rolling-7d response stats (or document the gap and seed the KPIs in `fix-approvals-seed-decided-approvals-for-ring`).

5. **Drop dead labels.** Remove `RESOLVED/WK` + `AVG RESPONSE` from the home strip — neither survives the converge.

6. **Update vitest.** `web/components/skycot/approvals/__tests__/ApprovalsKpiStrip.test.tsx`:
   - Pin the 5 labels (PENDING / $ AT RISK / MEDIAN TIME / APPROVAL RATE / ACTIVE REVIEWERS)
   - Snapshot test for empty-state (all 0 / —)
   - Snapshot test for populated state

## Acceptance criteria

- [ ] `/` and `/approvals` render the SAME 5-KPI strip with the SAME labels in the SAME order
- [ ] Labels are: PENDING / $ AT RISK / MEDIAN TIME / APPROVAL RATE / ACTIVE REVIEWERS
- [ ] The strip is a single reusable component, consumed by both routes
- [ ] Both routes derive KPIs from the same hook / data source
- [ ] Vitest spec pins the 5 labels in the canonical order
- [ ] `pnpm --dir web exec vitest run components/skycot/approvals` green
- [ ] `pnpm --dir web exec tsc --noEmit` green

## May-touch

- `web/components/skycot/approvals/ApprovalsKpiStrip.tsx` (~80 LOC new component) OR refactor of existing `KpiStrip`
- `web/app/(app)/page.tsx` (~10 LOC swap)
- `web/components/skycot/approvals/ApprovalsOpsDashboard.tsx` (~10 LOC swap)
- `web/lib/hooks/use-approvals-kpis.ts` (~40 LOC new hook if not present)
- `web/components/skycot/approvals/__tests__/ApprovalsKpiStrip.test.tsx` (~80 LOC)

## Out of scope

- Route convergence decision (`fix-approvals-page-empty-state-vs-ops-dashboard-route-ambiguity` — orthogonal to the KPI label set)
- Empty-state KPI hardcoding (covered by `fix-home-approvals-kpi-strip-hardcoded-7d-stats` + `fix-approvals-active-reviewers-kpi-derive-from-pending`)
- Donuts + 7-day ring (`fix-approvals-ops-dashboard-dollar-at-risk-donut-shape-implies-percent` + `fix-approvals-ops-dashboard-hardcoded-kpis-and-donuts`)
- Approve/deny click path (`fix-approvals-ops-dashboard-inline-approve-reject-buttons`)

## Effort

S–M. ~140 LOC component + ~80 LOC test + ~40 LOC hook. ~1.5 hours. P1 because the camera sees two different shapes of the same product depending on entry-route.
