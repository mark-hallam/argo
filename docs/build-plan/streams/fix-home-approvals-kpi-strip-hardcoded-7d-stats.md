# fix-home-approvals-kpi-strip-hardcoded-7d-stats

**Owner**: claude or codex.
**Authored against**: origin/main `d15461a3a` on `2026-05-25`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-home-approvals-kpi-strip-hardcoded-7d-stats`.
**Source**: e2e-ux-audit Agent (scene-6-claim-6-trust-and-approvals) finding F10 (`docs/skycot/ux-audits/2026-05-25-2309/scene-6-claim-6-trust-and-approvals.md`).

🟡 **P1 — Home (`/`) approvals KPI strip hardcodes "42 resolved / wk", "8m avg response", "78% approval rate" on a brand-new tenant with zero decision history. Sits beside the real `PENDING · 0` tile creating an "wait, where did the 42 go?" trust risk.**

## Root cause

Even on a brand-new tenant with `pendingCount = 0` and no decision history, the home inbox at `/` renders:
- "RESOLVED / WK · 42 · rolling 7-day total"
- "AVG RESPONSE · 8m · SLA target 2h"
- "APPROVAL RATE · 78% · 7-day rolling"
- Right-hand donut "78% 7-DAY APPROVAL RATE · 78% approved · 14% refused · 8% expired"

The dollar-at-risk tile correctly falls back to "—" — good pattern. The other tiles do not.

Touch site: `web/components/skycot/approvals/KpiStrip.tsx:53-80` (the `computeKpiCells` function).

This is a different file from the existing `fix-approvals-ops-dashboard-hardcoded-kpis-and-donuts` brief, which targets `ApprovalsOpsDashboard.tsx`. Both files duplicate the same anti-pattern (hardcoded fallback stats); the fixes are analogous but the surfaces are different. `KpiStrip` is rendered on `/` (home); `ApprovalsOpsDashboard` is on `/approvals`. A buyer landed signed-in sees `KpiStrip` first.

Per the function-vs-dressing rule: KPI cards on the home page are dressing the camera doesn't drill into — BUT they appear on the same row as the real `PENDING · 0` tile, so the inconsistency reads as confusing or untrustworthy.

## Findings this brief closes

- scene-6-claim-6-trust-and-approvals F10 [P1] — Home `/` Approvals inbox KPI strip hardcodes 42 / 8m / 78%

## Fix recipe

Two options; pick (a) for the smaller patch (mirrors `fix-approvals-ops-dashboard-hardcoded-kpis-and-donuts` resolution):

**(a) Derive from real data**:
1. Touch site: `web/components/skycot/approvals/KpiStrip.tsx:53-80`.
2. Build a `useApprovalKpis7d()` hook that derives from `useAuditList({range:'7d'})`:
   - Resolved/wk = count of `verb === "approval.decided"` rows
   - Avg response = `mean(decided.timestamp - pending.created_at)` across decided rows
   - Approval rate = `approved / decided`
3. Pass derived values to the strip; if `decided.length === 0`, render "—" (mirror the dollar-at-risk pattern).

**(b) Remove the hardcoded tiles**:
1. Replace the 3 hardcoded tiles with tiles that have real data backing (e.g. "PENDING", "DOLLAR AT RISK", "ACTIVE REVIEWERS" with a "—" fallback).
2. Reduce the strip from 5 tiles to 2-3 honest ones until real 7-day tracking exists.

Recommend (a). Operator-locked memory `feedback_no_theater_in_demo` makes this a clear win.

## Acceptance criteria

- [ ] No literal `42` / `8m` / `78%` / `0.78` / `0.14` / `0.08` in `KpiStrip.tsx`.
- [ ] On a brand-new tenant: tiles render "—" or honest zeros.
- [ ] On a tenant with decision history: tiles render derived values, not fallbacks.
- [ ] Donut visual: real-derived OR removed (flat circle with the percent text).
- [ ] `KpiStrip.test.tsx` pin: assert no hardcoded literals; assert "—" branch for empty data.

## May-touch

- `web/components/skycot/approvals/KpiStrip.tsx`
- `web/components/skycot/approvals/KpiStrip.test.tsx` (NEW or extend)
- `web/lib/hooks/use-approval-kpis-7d.ts` (NEW, optional)
- `web/lib/api/audit.ts` (only if existing `useAuditList` doesn't accept a `range` param)

## Out of scope

- The ops-dashboard hardcoded KPIs (separate brief `fix-approvals-ops-dashboard-hardcoded-kpis-and-donuts`)
- Cross-tenant approvals verification (separate brief `fix-approvals-cross-tenant-read-path-verification`)
- Approvals rows unclickable (separate brief `fix-approvals-rows-unclickable-no-detail-route`)

## Effort

~50-120 LOC + 1 test. ~1-2 hours.
