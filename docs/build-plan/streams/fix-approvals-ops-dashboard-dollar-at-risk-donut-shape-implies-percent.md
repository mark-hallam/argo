# fix-approvals-ops-dashboard-dollar-at-risk-donut-shape-implies-percent

**Owner**: claude.
**Authored against**: origin/main `80e149df55f6` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-approvals-ops-dashboard-dollar-at-risk-donut-shape-implies-percent`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-26-1028/scene-6-claim-6-trust-and-approvals.md` F11 [P2].

🟡 **P2 — `/approvals` ops-dashboard "$ at risk" donut now renders muted (no percent fill) which is honest about the missing metric, but the donut SHAPE still implies a measurable percentage exists. Either remove the donut or anchor it to a real ratio.**

## Root cause

`ApprovalsOpsDashboard.tsx:193-199` renders `<MetricDonutCard>` without `percent` prop → `pct == null` → muted ring. The visual is honest about the gap but the donut affordance still suggests "percentage of something." Improvement over the prior 82% hardcode (F5 in 2026-05-25 audit) but residual.

## Findings this brief closes

- scene-6-claim-6-trust-and-approvals F11 [P2] — "$ at risk" donut shape implies a percentage that doesn't exist

## Fix recipe

1. **Decide what the donut should encode**:
   - If "approvals-pending dollars / total-decided dollars in window" — anchor to that ratio. Compute from real data.
   - If no meaningful denominator exists yet — replace the donut with a simple stat card showing just the dollar value, no ring shape.
2. **Implementation**:
   - `web/components/skycot/approvals/ApprovalsOpsDashboard.tsx:193-199`: replace `<MetricDonutCard>` with `<MetricStatCard>` (which renders just the value + sub-label).
   - Or pass `percent={atRiskRatio}` when the underlying compute lands.
3. **Sub-label clarity**: whichever way, sub-label should clarify what's being measured ("at risk in 14d window" or similar).
4. **Test**: web test asserts the card renders without a donut ring while the percent is unknown.

## Acceptance criteria

- [ ] "$ at risk" card no longer renders a donut ring shape when no percent is known.
- [ ] Card sub-label is clear about the measurement scope.
- [ ] Web test covers the no-percent render.
- [ ] No regression on the other 4 KPI cards.

## May-touch

- `web/components/skycot/approvals/ApprovalsOpsDashboard.tsx:193-199`
- `web/components/skycot/approvals/MetricStatCard.tsx` (NEW or verify)
- `web/tests/approvals-ops-dashboard-cards.test.ts` (NEW or extend)

## Out of scope

- The other 4 KPI cards (already honest per F10)
- Real $-at-risk compute (deferred until pricing-aware workflow runtime lands)

## Effort

~30-50 LOC + 1 test. ~1 hour.
