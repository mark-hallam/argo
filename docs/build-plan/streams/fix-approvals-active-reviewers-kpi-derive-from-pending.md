# fix-approvals-active-reviewers-kpi-derive-from-pending

**Owner**: claude.
**Authored against**: origin/main `ea161df3a` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-approvals-active-reviewers-kpi-derive-from-pending`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-26-2007/scene-6-claim-6-trust-and-approvals.md` F7 [P1].

🟠 **P1 — The Approvals ops-dashboard 5th KPI cell `"Active reviewers"` is hardcoded to `value: "—", sub: "reviewer roster pending"` (`web/components/skycot/approvals/ApprovalsOpsDashboard.tsx:146-150`). Once the dashboard renders (post-#2406 it always does), the 5th cell visibly says "—" next to four populated cells. Prior PRs #2299 / #2319 fixed two other KPI fabrications but left this one. On the demo MP4 this reads as broken software.**

## Findings this brief closes

- scene-6 F7 [P1] — Approvals "Active reviewers" KPI permanently `—`

## Fix recipe

Two options — pick (A) if the approvals data has reviewer info, (B) if minimising surface change is preferred.

**Option A — derive from approvals data (preferred)**

1. **Inspect** `ApprovalRowJson` at `web/lib/api/approvals.ts` for the reviewer/approver field (likely `approver`, `approver_name`, or `reviewers: string[]`).
2. **Replace** the hardcoded entry in `ApprovalsOpsDashboard.tsx:146-150`:
   ```tsx
   {
     testId: "kpi-active-reviewers",
     label: "Active reviewers",
     value: distinctApproverCount > 0 ? String(distinctApproverCount) : "—",
     sub: distinctApproverCount > 0 ? "reviewing now" : "no reviewers assigned",
   }
   ```
3. **Compute** `distinctApproverCount` near the existing KPI derivations (probably top of the component):
   ```tsx
   const distinctApproverCount = new Set(
     pending
       .map(p => p.approver?.name ?? p.approver_name ?? p.approver)
       .filter((name): name is string => Boolean(name))
   ).size;
   ```
   Use `pending` (the filtered-to-pending list already in scope), not `items`, so "active" means actively reviewing.

**Option B — drop the cell to a 4-up strip**

1. **Remove** the 5th KPI cell entirely.
2. **Adjust** the KPI strip flex/grid container to accept 4 cells without breaking visual balance — the current 5-up rhythm likely needs widening per-cell or a max-width adjustment.

Operator preference: option A. Real data > pruned UI.

## Acceptance criteria

- [ ] When ≥1 pending approval has an approver, the cell shows the distinct count (e.g. `"3"` + `"reviewing now"`)
- [ ] When 0 pending approvals have approvers, cell shows `"—"` + `"no reviewers assigned"` (NOT `"reviewer roster pending"`)
- [ ] No fabricated reviewer names appear anywhere
- [ ] `pnpm --dir web exec vitest run components/skycot/approvals` green
- [ ] `pnpm --dir web exec tsc --noEmit` green

## May-touch

- `web/components/skycot/approvals/ApprovalsOpsDashboard.tsx` (~15 LOC)
- `web/components/skycot/approvals/ApprovalsOpsDashboard.test.tsx` (add 2 cases: with approvers / without)
- `web/lib/sample-approvals.ts` (if needed — populate `approver` on 2-3 pending rows so the demo shows a non-`—` value)
- `web/lib/api/approvals.ts` (read-only confirmation; do not change the envelope)

## Out of scope

- Other KPI cells (approval rate, $ at risk, time-to-decision — already populated)
- The approval-row click bug (covered by `fix-approval-row-click-router-push-and-real-action-id`)
- F8 — approval rate ring decision-undefined (related but separate brief)

## Effort

~30 LOC change + ~30 LOC test + ~10 LOC sample-data tweak. ~45 minutes. P1 demo-blocker for Scene 6.
