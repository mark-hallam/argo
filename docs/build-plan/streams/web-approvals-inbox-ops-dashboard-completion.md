# web-approvals-inbox-ops-dashboard-completion — finish the locked E-ops-dashboard design

**Owner**: claude.
**Authored against**: origin/main `50e65e9f` on `2026-05-21`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-approvals-inbox-ops-dashboard-completion`.
**Priority**: P1 — closes the wave-1 trust+safety audit gap (Approvals inbox missing the entire ops-dashboard signal set the design-lock prescribes).
**Sizing**: ~600-900 LOC (5 KPI cells + SLA bars + $-at-risk + confidence donuts + severity tiers + 7-day rate ring + skeletons + tests).

## Why this brief exists

Per the operator-locked Approvals page design (memory:
`project_approvals_page_design_locked.md`), the inbox is **E-ops-dashboard**
register: 5-KPI strip + 3 severity tiers + SLA bars + $-at-risk +
confidence donuts + 7-day approval-rate ring. Wave-1 trust+safety audit:

> Approvals inbox is missing the entire ops-dashboard signal set the
> design-lock prescribes. No 5-KPI strip (currently 3 placeholder
> KPIs at web/app/(app)/page.tsx:512-545); no SLA bars; no $-at-risk;
> no confidence donuts; no severity tiers. A Diego-persona walkthrough
> asks "where's the rest?"

The page renders `<ApprovalsHero pendingCount={pendingCount}>` with
3 placeholder KPIs (`resolvedThisWeek = 42 // placeholder`,
`avgResponse = "8m" // placeholder`, pendingCount). The locked
design has 5 KPIs and rich approval-card decoration.

## Goals

1. **5-KPI strip** above the inbox: Pending decisions / Resolved this week / Avg response / $ at risk / Approval rate (7-day).
2. **3 severity tiers** on each approval card: P0 (red), P1 (amber), P2 (slate). Sorting respects severity first.
3. **SLA bars** on each card: time-to-expire visualized as a colored bar (green ≥4h, amber ≥1h, red <1h).
4. **$-at-risk** per card when the action has a monetary impact (spend approval, refund, payout).
5. **Confidence donuts** per card showing the worker's stated confidence in the proposed action (0-100%).
6. **7-day approval-rate ring** in the KPI strip — visual donut showing approval/refusal/expire ratios over the rolling week.
7. **Reduced-motion handling** for any animated donuts/bars (mirror BrandLoudHero pattern).

## Steps

1. Read the locked E-ops-dashboard HTML reference under `docs/skycot/design-explorations/2026-05-19/approvals/option-E.html` (or wherever the operator-picked variant lives).
2. Read `web/app/(app)/page.tsx` `ApprovalsHero` (~line 522) + `ApprovalCard` to see current state.
3. Extend `ApprovalCard` props to include `severity`, `$AtRisk`, `confidence`, `slaExpiresAt`.
4. Build `<SeverityPill>`, `<SLABar>`, `<DollarAtRisk>`, `<ConfidenceDonut>` reusable components under `web/components/skycot/approvals/`.
5. Build `<KpiStrip>` with 5 cells + the 7-day ring donut.
6. Wire sorting: severity desc → SLA asc.
7. Add seed data for severity/$/confidence on sample approvals (sample-approvals.ts).
8. Verify reduced-motion CSS guards every animated component.

## may-touch

- `web/app/(app)/page.tsx` (ApprovalsHero + main render path)
- `web/components/skycot/ApprovalCard.tsx`
- New: `web/components/skycot/approvals/{SeverityPill,SLABar,DollarAtRisk,ConfidenceDonut,ApprovalRateRing,KpiStrip}.tsx`
- `web/lib/sample-approvals.ts` (extend seed with new fields)
- `web/lib/api/approvals.ts` (gateway envelope additions; keep backward-compat)
- Test files for each new component

## must-NOT-touch

- `web/app/(app)/layout.tsx` (out-of-scope)
- BrandLoudHero (this is a brand-restrained page; do NOT introduce it)
- Other workers/* pages

## Acceptance criteria

- [ ] 5-KPI strip renders with Pending / Resolved this week / Avg response / $ at risk / Approval rate.
- [ ] 3 severity tiers visible on each approval card with correct colour token.
- [ ] SLA bars render with correct color thresholds (green ≥4h, amber ≥1h, red <1h).
- [ ] $-at-risk shows where applicable; absent (not "$0") where not.
- [ ] Confidence donuts render 0-100% with a single-digit-precision center number.
- [ ] 7-day approval-rate ring donut animates on mount (or static when `prefers-reduced-motion`).
- [ ] Sorting respects severity desc → SLA asc.
- [ ] `pnpm typecheck && pnpm lint && pnpm test` clean.
- [ ] Trust+safety re-audit: Approvals inbox trust score moves from ~5 to 9 on the relevant rubric.

## References

- Operator memory: `project_approvals_page_design_locked.md`
- Audit: `docs/skycot/audit/trust-safety/2026-05-20-wave-1-trust-safety-audit.md`
