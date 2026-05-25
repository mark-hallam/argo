# fix-approvals-ops-dashboard-hardcoded-kpis-and-donuts

**Owner**: claude or codex.
**Authored against**: origin/main `0653c478d` on `2026-05-25`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-approvals-ops-dashboard-hardcoded-kpis-and-donuts`.
**Source**: e2e-ux-audit Agent 4 findings F4 + F5 + F10 + F11 (`docs/skycot/ux-audits/2026-05-25-1100-scene-6-claim-6-trust-and-approvals.md`).

🟡 **P1 — Scene 6 visible-on-camera KPI strip lies. Dressing pretending to be function.**

## Root cause

`web/components/skycot/approvals/ApprovalsOpsDashboard.tsx` hardcodes the entire KPI strip + the $-at-risk donut + the 7-day approval-rate ring, regardless of real data:

- L77 — `const approvalRate = decided.length > 0 ? approved / decided.length : 0.78;` — fallback constant `0.78` with sub-label "7-day baseline" (implies a real 7-day tracker; none exists)
- L102 — sub-label "7-day baseline" literal
- L108 — `value: pending.length > 0 ? "1" : "0"` — "Active reviewers" hardcoded 1
- L135-142 — `{ headline: "78%", slices: { approved: 0.78, refused: 0.14, expired: 0.08 } }` hardcoded for the approval-rate ring
- L163 — `percent={totalAtRisk > 0 ? 0.82 : 0}` — $-at-risk donut visual fill is a flat 82% regardless of the actual dollar amount
- L180 — `<ApprovalRateRing approved={0.78} refused={0.14} expired={0.08} />` — duplicate hardcoded values
- L187 — "78% approved · 14% refused · 8% expired" string literal

The dollar number itself (summed from real `item.dollarAtRisk`) is function-side. The visual gauge percentages are pure cosmetic and would mislead any buyer asking "how is that 82% calculated?" mid-narration.

Per the function-vs-dressing rule + `feedback_no_theater_in_demo`: if the camera zooms on a donut and the operator narrates "82% of cap at risk", that becomes function-side theatre.

## Findings this brief closes

- Agent 4 F4 [P1] — `/approvals` ops-dashboard hardcodes the entire "7-day approval rate" ring (78/14/8%)
- Agent 4 F5 [P1] — `$` at risk donut visual fill is hardcoded 82%
- Agent 4 F10 [P1] — Approval-rate KPI fallback is `0.78` with sub-label "7-day baseline" (implies a real 7-day tracker)
- Agent 4 F11 [P2] — "Active reviewers" KPI is hardcoded `1` when pending > 0

## Fix recipe

For each hardcoded value, decide between two options (per the operator's function-vs-dressing rule):

**Option A — derive from real data (preferred)**:
1. **7-day approval rate ring**: derive from `useAuditList({range:'7d'})` — count audit rows where `verb === "approval.decided"` and split by `result === "approved" | "refused" | "expired"`. Build a small `useApprovalRate7d()` hook.
2. **$-at-risk donut visual**: derive from `totalAtRisk / tenantApprovalBudget` (a new field on `/api/v1/tenant/settings` or the `/api/v1/approvals` response). If the budget concept doesn't exist yet, drop the visual fill — render a flat circle with the dollar number inside.
3. **Active reviewers**: derive from a real roster query (e.g. `useReviewers()` against `/api/v1/approvers` or count of distinct `assignee` values in pending items). If no roster query exists, render "—" with a tooltip "Reviewer roster wiring pending".

**Option B — remove the visual gauges entirely (acceptable interim)**:
- Replace the donuts with the bare dollar/percent number, no visual fill ring.
- Replace "Active reviewers" KPI tile with a different tile that has real data backing.
- Drop the "7-day baseline" sub-label until real tracking exists.

Pick A for any tile that has a credible derivation source. Pick B for tiles where no real source exists yet (the brief author should NOT invent gateway endpoints — defer those to follow-on briefs and use placeholders honestly).

## Acceptance criteria

- [ ] No literal `0.78` / `0.82` / `0.14` / `0.08` in `ApprovalsOpsDashboard.tsx`.
- [ ] Approval-rate ring derives from real audit data OR shows "—" (no fabricated fill).
- [ ] $-at-risk visual is either real-derived OR removed (flat circle with the dollar number).
- [ ] Active-reviewers tile is real-derived OR replaced with a real-data tile OR shows "—".
- [ ] Sub-labels do not claim "7-day baseline" unless the data is sourced from a real 7-day window.
- [ ] Component test (`ApprovalsOpsDashboard.test.tsx`) pinning the derivation paths so the next refactor can't drop them.

## May-touch

- `web/components/skycot/approvals/ApprovalsOpsDashboard.tsx` (the four hardcoded sites)
- `web/components/skycot/approvals/ApprovalsOpsDashboard.test.tsx` (new pin)
- `web/lib/api/approvals.ts` (if a new derived hook is added)
- `web/lib/hooks/use-approval-rate-7d.ts` (NEW, optional)

## Out of scope

- Empty-state vs ops-dashboard route ambiguity (separate brief: `fix-approvals-page-empty-state-vs-ops-dashboard-route-ambiguity`, already authored)
- Approvals rows unclickable (separate brief: `fix-approvals-rows-unclickable-no-detail-route`)
- Cross-tenant approvals verification (separate brief: `fix-approvals-cross-tenant-read-path-verification`)

## Effort

~50-120 LOC + 1-2 test pins. ~45-90 min.
