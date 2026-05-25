# fix-approvals-rows-unclickable-no-detail-route

**Owner**: claude or codex.
**Authored against**: origin/main `0653c478d` on `2026-05-25`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-approvals-rows-unclickable-no-detail-route`.
**Source**: e2e-ux-audit Agent 4 finding F6 (`docs/skycot/ux-audits/2026-05-25-1100-scene-6-claim-6-trust-and-approvals.md`).

🟡 **P1 — Approval rows look clickable (button + hover + focus ring) but have no onClick/href. The visual lies about the affordance.**

## Root cause

`ApprovalRow` (`web/components/skycot/approvals/ApprovalsOpsDashboard.tsx:242-296`) is rendered as a `<button>` with `aria-label`, hover style, focus ring — every visual affordance of a clickable item — but **no `onClick` and no `href`**. The Scene 6 brief asks to "click into an approval row" and verify real worker_id + audit lineage. There is nowhere to click into.

Either the rows need to navigate to `/audit/action/{approval.actionId}` (linking approvals to their underlying audit row) OR the visual affordance needs to be removed so the buttons don't lie.

## Findings this brief closes

- Agent 4 F6 [P1] — Approval rows are unclickable; no detail surface exists

## Fix recipe

**Option A (preferred — wire to existing audit detail surface)**:
1. Each approval already references an underlying audit action via `approval.actionId` (verify the field name in `web/lib/api/approvals.ts`).
2. Add `onClick={() => router.push(`/audit/action/${approval.actionId}`)}` to `ApprovalRow` at `ApprovalsOpsDashboard.tsx:242-296`.
3. Add `cursor: pointer` to make the affordance honest.
4. Verify the cross-link works against the canonical 5-persona seed approvals.

**Option B (if no action linkage exists)**:
1. Build a dedicated `/approvals/<id>` detail page showing the approval's policy, dollar at risk, severity, requestor, full request payload, decision history.
2. Wire `ApprovalRow` to `router.push(`/approvals/${approval.id}`)`.

**Option C (defer detail; remove the lie)**:
1. Convert the `<button>` to a `<div>` — drop the focus ring, hover style.
2. Add a single "View audit" link button on each row that explicitly navigates somewhere meaningful.

Recommendation: ship Option A. It reuses the audit detail surface (which is being fixed under `fix-audit-list-detail-endpoint-id-mismatch`) and gives the Scene 6 click-through a real destination.

Note: this brief is coupled to `fix-audit-list-detail-endpoint-id-mismatch` — Option A only works once the audit detail endpoint reliably resolves the IDs that approvals reference. Sequence accordingly.

## Acceptance criteria

- [ ] Clicking an `ApprovalRow` navigates to `/audit/action/{action_id}` (Option A) or `/approvals/<id>` (Option B), not nowhere.
- [ ] The cursor shows pointer on hover, not text/default.
- [ ] No focus ring on non-interactive elements (DOM lie removed).
- [ ] The destination page renders the underlying audit row (Option A) or the approval detail (Option B) without 404.
- [ ] Playwright / component test: clicking a sample-approval row navigates to a 200 destination.

## May-touch

- `web/components/skycot/approvals/ApprovalsOpsDashboard.tsx:242-296` (ApprovalRow)
- `web/lib/api/approvals.ts` (verify the `actionId` field exists; add it if not)
- `web/app/(app)/approvals/[id]/page.tsx` (NEW, only under Option B)

## Out of scope

- Audit list/detail endpoint mismatch (separate brief: `fix-audit-list-detail-endpoint-id-mismatch`, already authored — this brief sequences after that one)
- Ops-dashboard hardcoded KPIs (separate brief: `fix-approvals-ops-dashboard-hardcoded-kpis-and-donuts`)
- Empty-state vs ops-dashboard route ambiguity (separate brief: `fix-approvals-page-empty-state-vs-ops-dashboard-route-ambiguity`)

## Effort

~20-50 LOC for Option A; ~150-300 LOC for Option B. ~30-45 min for A; ~2 hours for B.
