# fix-approvals-row-layout-collision-sla-bar-vs-inline-actions-narrow-viewport

**Owner**:
**Authored against**: origin/main `354de50a0c5fdf561125c7e931699fe94daa326d` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-approvals-row-layout-collision-sla-bar-vs-inline-actions-narrow-viewport`.
**Source**: e2e-ux-audit T0739 — `docs/skycot/ux-audits/2026-05-27-0739/scene-6-claim-6-trust-and-approvals.md` F4.

🟡 **P1 — Post-#2414 inline Approve / Refuse buttons now sit beside the existing SLA bar + dollar-at-risk + confidence donut on every approval row. The grid template at `ApprovalsOpsDashboard.tsx:392` is `lg:grid-cols-[minmax(0,1fr)_7.5rem_5rem_6.5rem_auto]`. On <1280px viewports the cluster competes for horizontal space, the SLA bar's "2h 60m left" overflow text wraps awkwardly, and the new action buttons crowd into the donut. The visual collision is the second-order effect of correctly adding the buttons; the row layout needs a second pass.**

## Root cause

`web/components/skycot/approvals/ApprovalsOpsDashboard.tsx:392` defines the row grid:
```
lg:grid-cols-[minmax(0,1fr)_7.5rem_5rem_6.5rem_auto]
```

The columns are: (a) `minmax(0,1fr)` — the row body/description; (b) `7.5rem` — SLA bar; (c) `5rem` — dollar-at-risk; (d) `6.5rem` — confidence donut; (e) `auto` — actions cluster (newly added inline Approve/Refuse from #2414).

At <1280px the `lg:` breakpoint stops applying and the layout falls back to whatever the default mobile/tablet template defines (vertical stack, presumably). But in the intermediate viewport range (1024px-1279px) the layout currently uses an unstacked grid that doesn't account for the new actions column — the buttons overflow into the row width or wrap into a second visual line.

Separately, `SLABar.tsx:46-50` (unchanged) emits "2h 60m left" when minutes overflow 60 — a pre-existing rounding bug covered by `fix-approvals-sla-bar-rounding-60-minute-overflow.md` (existing). This brief is the new layout-collision finding triggered by the actions addition.

## Findings this brief closes

- scene-6-claim-6-trust-and-approvals F4 [P1] CARRY-WORSENED — SLA timer "2h 60m left" overflow + visual collision with new inline Approve/Refuse cluster

## Fix recipe

1. **Add a `md:` breakpoint to the row grid** so the layout transitions cleanly between mobile/tablet/desktop. Three reasonable patterns:
   - **(a) Stack actions below the row at <1280px.** Add `md:grid-cols-[...]` template that drops the actions to a second visual line at `md:` and `lg:`.
   - **(b) Hide the actions on tablet + show them in a row-tap-expand drawer.** More work; better mobile UX.
   - **(c) Use container queries (`@container`) instead of media queries.** Cleanest but requires Tailwind config change.
   Recommend (a) for v1.0 demo — smallest change.
2. **Tighten the donut + SLA bar column widths** at the `md:` breakpoint:
   - SLA bar: `5.5rem` (from `7.5rem`).
   - Dollar-at-risk: `4rem` (from `5rem`).
   - Confidence donut: `5rem` (from `6.5rem`).
   - Actions: stacks below at `md:` per pattern (a).
3. **Verify the "2h 60m" overflow is handled separately** — that's the scope of the existing SLA bar brief; not this one.
4. **Vitest pin via Playwright** at 3 viewport widths (375px, 1024px, 1920px); assert no horizontal overflow and the actions cluster is reachable.

## Acceptance criteria

- [ ] At 1024px viewport, the approvals row renders with no horizontal overflow; the actions cluster is visible and clickable.
- [ ] At 375px viewport, the actions cluster stacks below the row body cleanly.
- [ ] At 1920px viewport, the existing single-line layout is preserved (no regression).
- [ ] Playwright pin: walk the approvals page at the 3 widths; assert no overflow + actions visible.

## May-touch

- `web/components/skycot/approvals/ApprovalsOpsDashboard.tsx` (grid template + `md:` breakpoint)
- `web/components/skycot/approvals/ApprovalRowActions.tsx` (no shape change expected)
- `web/components/skycot/approvals/__tests__/ApprovalsOpsDashboard.test.tsx` (pin)

## Out of scope

- The SLA bar minute-overflow rounding (covered by `fix-approvals-sla-bar-rounding-60-minute-overflow.md`)
- The active-reviewers KPI hardcode (covered by `fix-approvals-active-reviewers-kpi-derive-from-pending.md`)
- The `/` vs `/approvals` dual-surface question (covered by `fix-approvals-page-empty-state-vs-ops-dashboard-route-ambiguity.md`)

## Effort

S — ~40-60 LOC + Playwright pin. ~1 hour. P1 because the visual collision is reachable on any <1280px viewport — most laptops/recording screens.
