# fix-approvals-inbox-waiting-on-you-fixture-drift

**Owner**: claude.
**Authored against**: origin/main `ddf44dfb4` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-approvals-inbox-waiting-on-you-fixture-drift`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-26-2007/chrome-live-walk-and-playwright.md` F1 [P0].

🔴 **P0 DEMO-BLOCKER — `/team` worker deck shows 4 of 5 cards with "Waiting on you" chip (Mariah, Peta, Priya, and others). `/approvals` shows empty-state card "No approvals waiting" with copy "When a worker hits a decision that needs sign-off... it lands in your inbox at home." Both surfaces are running off the same demo-fallback path but disagree about pending approvals.**

## Root cause

The worker-card "Waiting on you" chip is derived from a different sample dataset than the approvals inbox query. Two separate fixtures have drifted:

1. **`/team` cards** — `web/lib/sample-personas.ts` or `web/lib/sample-team.ts` carries a per-worker `pending_approvals_count` (or equivalent) field that drives the "Waiting on you" chip badge. Currently 4 of 5 personas have this set.
2. **`/approvals` inbox** — `web/lib/api/approvals.ts` reads from `SAMPLE_APPROVALS` (`web/lib/sample-approvals.ts` or similar) which may be empty OR populated with 8 items that never link back to the personas' `pending_approvals_count`.

Per the audit: both pages fall through to demo-fallback under the current dev profile (env var typo at `.claude/launch.json:8` covered by separate brief means bypass is OFF, so both pages hit the live gateway, which has no approvals OR sample data). Even when bypass works, the two fixtures don't agree.

Additionally, per `project_approvals_page_design_locked.md` the locked E-ops-dashboard design specifies a 5-KPI strip, 3 severity tiers, $ at risk + confidence donuts, 7-day approval rate ring — and the empty-state short-circuits ALL of that. So the locked design is not just under-populated; the code path appears not adopted in the empty-state branch.

## Findings this brief closes

- chrome-live-walk-and-playwright F1 [P0] — Approvals empty inbox while 4 worker cards say "Waiting on you"
- chrome-live-walk-and-playwright F1 (subpart) — Locked ApprovalsOpsDashboard does not render in the empty-state code path

## Fix recipe

1. **Make `pending_approvals_count` derived, not stored.** Refactor `web/lib/sample-personas.ts` / `web/lib/sample-team.ts` to compute the per-worker badge count from the SAMPLE_APPROVALS array:
   ```ts
   const badgeCount = SAMPLE_APPROVALS.filter(a => a.workerId === persona.workerId && a.status === "pending").length;
   ```
   That way the team page and the approvals inbox cannot disagree by construction.

2. **Seed SAMPLE_APPROVALS to match the team page** — at least 4 pending approvals (one per badge-bearing worker) with realistic ids, summaries, severity, $ at risk, and routed-to-approver:
   - Mariah → "List $4,200 vintage typewriter on eBay (item value > $2,000 floor)" — severity P2 — $4,200 at risk
   - Peta → "Approve sprint-end status memo to leadership" — severity P3 — $0 at risk
   - Priya → "Refund Acme Corp $1,250 invoice (above $1,000 self-serve limit)" — severity P1 — $1,250 at risk
   - One more from the team to round out the dashboard's 3 severity tiers

3. **Always render ApprovalsOpsDashboard, never short-circuit to empty-state when there's even 1 approval.** Audit the page logic at `web/app/(app)/approvals/page.tsx` — the short-circuit `if (approvals.length === 0) return <EmptyState />` should be removed in favour of letting the dashboard render with 0-state KPIs. The locked design has slots for "0 awaiting" so the empty case still shows the strip, donuts, and ring.

4. **Add an integration test** at `web/playwright/approvals/team-and-approvals-counts-agree.spec.ts`:
   - Load `/team`, count the cards with "Waiting on you" chip
   - Load `/approvals`, count the rows in the inbox
   - Assert the two counts match

## Acceptance criteria

- [ ] `/team` "Waiting on you" badge count per worker equals the SAMPLE_APPROVALS pending count for that worker
- [ ] `/approvals` always renders ApprovalsOpsDashboard (5-KPI strip + severity tiers + donuts + ring), even when 0 items
- [ ] When SAMPLE_APPROVALS has ≥4 items, the dashboard shows 4 rows
- [ ] New Playwright spec passes
- [ ] The dashboard is the page a viewer sees, not the EmptyState component

## May-touch

- `web/lib/sample-personas.ts` (make `pending_approvals_count` derived)
- `web/lib/sample-approvals.ts` (seed 4 realistic pending approvals)
- `web/app/(app)/approvals/page.tsx` (remove short-circuit to empty state)
- `web/components/skycot/approvals/ApprovalsOpsDashboard.tsx` (handle 0-state gracefully without redirecting)
- `web/playwright/approvals/team-and-approvals-counts-agree.spec.ts` (NEW)

## Out of scope

- Seed decided approvals to populate the 7-day ring (covered by `fix-approvals-seed-decided-approvals-for-ring`)
- Hard-coded ApprovalsOpsDashboard KPI text (covered by `fix-approvals-ops-dashboard-hardcoded-kpis-and-donuts`)
- The action-id stub when clicking through (covered by `fix-approvals-drop-hardcoded-action-id-map`)
- The route-ambiguity between `/` and `/approvals` (covered by `fix-approvals-page-empty-state-vs-ops-dashboard-route-ambiguity`)

## Effort

~150-300 LOC + 1 spec. ~4-6 hours. P0 because the team-vs-inbox disagreement reads as broken software to any investor watching.
