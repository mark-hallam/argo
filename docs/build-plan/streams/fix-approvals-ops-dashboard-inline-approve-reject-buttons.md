# fix-approvals-ops-dashboard-inline-approve-reject-buttons

**Owner**: claude.
**Authored against**: origin/main `78f162a5` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-approvals-ops-dashboard-inline-approve-reject-buttons`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-26-2152/scene-6-claim-6-trust-and-approvals.md` F3 [P0].

🔴 **P0 DEMO-BLOCKER for Scene 6 — The `/approvals` ops-dashboard rows have no inline `Approve` / `Refuse` / `Undo` buttons. Each row is a `<button>` that only navigates to `/audit/action/<actionId>`. Scene 6's central beat is "Operator clicks Approve. Slack POSTs the interactivity callback…" — there's no Approve button to click anywhere on `/approvals`.**

## Root cause

`web/components/skycot/approvals/ApprovalsOpsDashboard.tsx:297-358 ApprovalRow` renders rows as click-to-drill-in buttons only. The mutations exist already in `web/lib/hooks/use-approvals.ts`:

- `useSubmitDecision()` — approve / refuse
- `useBatchSubmitDecisions()` — bulk
- `useUndoDecision()` — undo

But these are consumed only by the canonical inbox at `/` (the card-style root inbox surface). The locked E-ops-dashboard at `/approvals` doesn't surface them. The operator would have to navigate to a DIFFERENT surface (`/`) which renders a DIFFERENT design language to perform the headline action of the scene.

Per `project_approvals_page_design_locked` (locked 2026-05-19, E-ops-dashboard register), the dashboard SHOULD show actionable controls — the design itself promises operator agency. Today it's a read-only ops view.

## Findings this brief closes

- scene-6-claim-6-trust-and-approvals F3 [P0] — No inline approve / reject affordance in the `/approvals` ops dashboard

## Fix recipe

1. **Add inline Approve / Refuse / Undo buttons to `ApprovalRow`** at `web/components/skycot/approvals/ApprovalsOpsDashboard.tsx:297-358`:
   ```tsx
   <div className="approval-row-actions">
     <button
       onClick={(e) => { e.stopPropagation(); submitDecision({ id: approval.id, decision: "approve" }); }}
       data-testid={`approve-${approval.id}`}
     >Approve</button>
     <button
       onClick={(e) => { e.stopPropagation(); submitDecision({ id: approval.id, decision: "refuse" }); }}
       data-testid={`refuse-${approval.id}`}
     >Refuse</button>
   </div>
   ```
2. **Keep the row drill-in** — clicking outside the action buttons still navigates to `/audit/action/<actionId>`. Use `e.stopPropagation()` on the action buttons.
3. **Wire `useSubmitDecision`** at the dashboard top-level so the buttons fire mutations against the gateway endpoint `POST /api/v1/approvals/<id>/decide`.
4. **Show optimistic UI** — on click, remove the row from the pending list with a 5s "Undo" toast (consuming `useUndoDecision`). Match the inbox-card surface's pattern at `/`.
5. **Visually distinguish severity** — the existing severity-tiered sections should keep their visual register; the Approve button color tracks severity (e.g. red border on P0 rows, neutral on P2).
6. **Add a Playwright spec** at `web/playwright/approvals/inline-approve-reject.spec.ts`:
   - Open `/approvals` with seeded pending approvals.
   - Click the Approve button on the first P0 row.
   - Assert the row disappears from the pending list.
   - Assert an "Undo" toast appears.
   - Click Undo; assert the row returns to pending.

## Acceptance criteria

- [ ] Every pending-approval row on `/approvals` has visible Approve + Refuse buttons.
- [ ] Clicking Approve removes the row optimistically and calls the gateway endpoint.
- [ ] Clicking Refuse opens a brief reason prompt (or fires a default refuse), then mutates.
- [ ] Undo toast appears for 5s; clicking it reverts.
- [ ] Clicking outside the buttons still navigates to the audit drill-in.
- [ ] New Playwright spec passes.
- [ ] No regression on the `/` inbox-card surface (existing tests still pass).

## May-touch

- `web/components/skycot/approvals/ApprovalsOpsDashboard.tsx` (ApprovalRow component)
- `web/components/skycot/approvals/ApprovalRowActions.tsx` (NEW — extract the button cluster if it grows)
- `web/lib/hooks/use-approvals.ts` (verify hooks expose the needed shape — no changes expected)
- `web/playwright/approvals/inline-approve-reject.spec.ts` (NEW)

## Out of scope

- Row drill-in to audit (already exists via the row `<button>`).
- Hardcoded action-id map drift (covered by `fix-approvals-drop-hardcoded-action-id-map`).
- KPI strip semantics (covered by `fix-approvals-ops-dashboard-hardcoded-kpis-and-donuts`).
- Approve via Slack interactivity (separate brief — Scene 6 references this but the SPA beat lands first).

## Effort

~200-400 LOC across component + tests + styling. ~4-8 hours. P0 because Scene 6's central beat is unscriptable without it.
