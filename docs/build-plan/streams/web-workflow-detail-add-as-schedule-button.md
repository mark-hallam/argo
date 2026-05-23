# web-workflow-detail-add-as-schedule-button

**Owner**: claude.
**Depends on**: `web-rename-duty-to-schedule-in-ui-copy` (cosmetic — this brief uses the new vocab from day one).
**Authored against**: origin/main `bdf57f458` on `2026-05-23`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-workflow-detail-add-as-schedule-button`.

🟡 **P2 — operators can hire a workflow but can't easily schedule it.** Source: evaluator round-15 finding (architecture supports vertical-wide workflow access but the operator UI surface doesn't let an operator turn a newly-added workflow into a recurring schedule for a specific worker).

> "1 worker = 1 vertical = N workflows = N duties (schedules). New workflows in a vertical are auto-accessible to existing workers, but operators have no easy 'add as recurring schedule for Mariah' CTA on the workflow detail page."

This closes the operator journey gap: after browsing or creating a workflow on the workflow-detail page, operators get a one-click `+ Add as Schedule for <worker>` flow that creates a new duty row keyed to (worker_id, workflow_name, cadence).

## Scope

On `web/app/(app)/workflows/[verticalId]/page.tsx` (workflow detail), add an `Add as Schedule` CTA in the page header (right of title), and a drawer/modal flow:

1. **CTA placement** — Top-right of the workflow header, brand-restrained register (NOT brand-loud), matching the audit page style (Inter, gradient sparingly).
2. **Drawer step 1: pick a worker** — list workers currently in the vertical (call existing gateway list-workers-in-vertical endpoint); each row = worker name + cohort + status. Single-select.
3. **Drawer step 2: pick a cadence** — radio group of common cadences (every-15-min, hourly, daily-9am, weekdays-9am, weekly-monday-9am) + a "Custom cron" text input gated behind an advanced disclosure.
4. **Drawer step 3: confirm + create** — POST to existing duty-create endpoint (the path that today's CLI `argo duty create` calls — read from gateway routes file). On success: toast "Schedule created — first fire at <ts>" + drawer closes + a sonner success.

## Files you may touch

- `web/app/(app)/workflows/[verticalId]/page.tsx` — header CTA insertion (~30 LOC).
- `web/app/(app)/workflows/[verticalId]/add-as-schedule-drawer.tsx` — NEW (~250 LOC, the 3-step drawer).
- `web/lib/api/workers.ts` — possible additive helper if list-workers-in-vertical isn't already there (~30 LOC).
- `web/lib/api/duties.ts` — possible additive helper if create-duty isn't already there (~30 LOC).

## Files you must NOT touch

- `crates/**` — gateway endpoints already exist; this is UI-only.
- Workflow detail page LOAD logic (don't refactor the existing fetch).
- Other workflow-tab routes.

## Acceptance criteria

- [ ] `+ Add as Schedule` button visible on the workflow detail page header.
- [ ] Clicking opens a 3-step drawer (worker → cadence → confirm).
- [ ] Submitting the drawer with valid inputs POSTs to the duty-create endpoint and returns 200/201.
- [ ] Success toast shown + drawer closes.
- [ ] Drawer is keyboard-accessible (Tab + Enter through steps; Escape closes).
- [ ] Mobile-responsive (drawer collapses to full-screen on viewport <640px).
- [ ] Playwright spec exercises the happy path (stubbed gateway response).
- [ ] `pnpm typecheck` + `pnpm lint` clean.
- [ ] BUILD_STATUS.md updated with PR number.

## Test plan

- Playwright: open `/workflows/<vertical>/<workflow>`, click `+ Add as Schedule`, step through, submit, assert toast.
- Type/lint clean.
- Manual: PR body screenshot showing the new CTA + drawer.

## Parallelism

- Manager-worker allowed: no.
- Max children: 1.

## Out of scope

- A standalone `/schedules` index page (separate brief if operator wants one).
- Editing/deleting an existing schedule (separate brief).
- Per-worker SLA / escalation override at create-time (gateway doesn't expose that on duty-create yet).

~300-400 LOC.
