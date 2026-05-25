# fix-approvals-page-empty-state-vs-ops-dashboard-route-ambiguity

**Owner**: claude or codex.
**Authored against**: origin/main `<latest>` on `2026-05-25`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-approvals-page-empty-state-vs-ops-dashboard-route-ambiguity`.
**Source**: e2e-ux-audit Agent 4 finding F3 + F13 (`docs/skycot/ux-audits/2026-05-25-1100-scene-6-claim-6-trust-and-approvals.md`).

🔴 **P0 DEMO-GATE — operator-locked E-ops-dashboard design is unreachable on live data; routes conflict.**

Two issues, one PR's worth of work:

1. `/approvals` only renders `<ApprovalsOpsDashboard>` when `pendingCount > 0` (`web/app/(app)/approvals/page.tsx:25-48`). Live gateway returns `{"items":[]}` for `GET /api/v1/approvals`, so the operator-locked E-ops-dashboard design (KPI strip + 3 severity tiers + $ at risk + confidence donuts + 7-day approval rate ring per memory `project_approvals_page_design_locked`) does **not render at all** on current backend state.

2. The page docstring acknowledges "The canonical approval queue lives at `/`". `web/components/skycot/nav-items.tsx:23` confirms the sidebar's "Approvals" entry routes to `/`. So a buyer typing `/approvals` and a buyer clicking the sidebar see TWO DIFFERENT SURFACES. The empty-state CTA "Open the inbox" routes to `/` — confirming the dead-end.

## Fix (recommended recipe)

**Option A (preferred — converge)**: `/approvals` and `/` render the SAME surface. The ops-dashboard is the canonical surface; on empty data, show KPI tiles with `—` placeholders + a tasteful empty-state callout in the rows area (NOT a different page shell). Both routes resolve to the same Next.js page component.

**Option B (route-segregate)**: keep `/` as the canonical inbox, redirect `/approvals` to `/` permanently. Then update the docstring + sidebar.

Option A is preferred because it lets the demo recording navigate to `/approvals` (which matches the runbook + viewer mental model) and immediately see the locked design even when zero approvals are pending — no `argo demo seed --tenant <demo>` precondition required.

## Acceptance criteria

- [ ] `/approvals` renders the locked E-ops-dashboard layout when `pendingCount == 0` (with `—` placeholders for the data-driven KPIs).
- [ ] `/approvals` and `/` resolve to the same canonical surface (or `/approvals` is a permanent 301 to `/` per Option B).
- [ ] The empty-state CTA, if Option A: doesn't navigate elsewhere (or navigates to a meaningful action like "Create a test approval"). If Option B: removed entirely.
- [ ] curl-test: `curl http://localhost:3000/approvals` and `curl http://localhost:3000/` both render the ops-dashboard shell.
- [ ] No regression on the populated state (when `argo demo seed` has been run + pending > 0).

## May-touch

- `web/app/(app)/approvals/page.tsx` (the conditional + the empty-state)
- `web/app/(app)/page.tsx` (the canonical root inbox)
- `web/components/skycot/approvals/ApprovalsOpsDashboard.tsx` (if KPI placeholders need to handle zero gracefully — they may already)
- `web/components/skycot/approvals/ApprovalsEmptyState.tsx` (likely deleted under Option A; kept-but-rendered-inline under Option A.alt)
- `web/components/skycot/nav-items.tsx` (only if sidebar route changes)
- This brief file (status updates)

## Out of scope

- Re-implementing the ops-dashboard design (locked + already implemented)
- Touching the gateway's approvals endpoint (Agent-4 confirmed it returns 200 with empty items — the SPA-side rendering is the issue)
- Cross-tenant approvals isolation (separate brief: `fix-approvals-cross-tenant-read-path-verification`)

## Effort

~30-80 LOC + 1-2 curl smoke checks. ~20-45 min.
