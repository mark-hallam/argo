# fix-workflow-run-now-silent-failure-no-toast-or-error-state

**Owner:** claude
**Authored against:** origin/main `410e6ed1240d26f9ab6b48eb94a2fdf177c38270` on 2026-05-28
**Validate before claim:** `bash scripts/check-brief-freshness.sh fix-workflow-run-now-silent-failure-no-toast-or-error-state`
**Source:** ux-audit-2026-05-27-2351

🟡 **P1 — Workflow Run-now button's only feedback on 403/500 is a brief "Running" label flip. No toast, no error chip, no row in the run-history strip. The auditor watching this would assume the run completed.**

## Root cause

`web/app/(app)/workflows/[verticalId]/[name]/page.tsx` (around lines 252-265 per the audit) renders the Run-now button with `disabled={running || !isLive}`. On click, the handler POSTs to `/api/v1/verticals/<v>/workflows/<n>/execute`. Today the gateway returns 403 Forbidden because the demo principal lacks the workflow-execute capability (separately tracked by `fix-workflow-execute-demo-principal-capability-grant`). But the SPA's error path does not surface that failure:

> Even with the 403 (F02), the button's only feedback is the brief "Running" label flip. The auditor watching this would assume the run completed. A failure path should surface with a sonner toast + persistent error chip on the run-history strip.
> — `whole-site-mvp-claims-coverage.md` F08

This is independent of the 403 itself. Even after the capability grant lands, network failures, gateway disconnects, or future business-rule rejections need to surface to the user. Today the button silently no-ops.

## Findings this brief closes

- `whole-site-mvp-claims-coverage.md` F08 [P1] — workflow "Run now" failure is silent — no toast, no error state, no audit row

## Fix recipe

1. **Add error toast + chip to the Run-now handler** in `web/app/(app)/workflows/[verticalId]/[name]/page.tsx`:
   - In the existing `onRunClick` mutation (or equivalent), `onError` callback should call `toast.error(...)` from the existing sonner integration (verify via `rg "from \"sonner\"" web/app/(app)/workflows/` for the canonical import).
   - Surface the error envelope's `code` + `message` if present; fall back to "Run failed — see audit log" + a "Retry" affordance.
   - On 403 specifically, render a more actionable message: "Demo principal cannot execute workflows yet. Switch to a real account or contact your workspace admin."
2. **Add a persistent error chip in the run-history strip**:
   - The history-strip component (likely `web/components/skycot/workflow/RunsPanel.tsx` or similar — grep for the "RUN HISTORY" string per `fix-workflow-detail-bun-history-typo`) should render a "Last attempt failed" row when the most-recent attempt is an error.
   - This chip remains until the operator clicks Run-now again successfully.
3. **Audit-row emission on failure** (operator-decisional, can defer):
   - If the gateway emits a `workflow.run.failed` row on the 403, the run-history strip should reflect it.
   - If not, that is a separate gateway brief — keep this brief SPA-only.
4. **Tests**:
   - Component test at `web/app/(app)/workflows/[verticalId]/[name]/RunNowButton.test.tsx` (NEW or extend) that mocks `executeWorkflow` to reject with 403, then asserts the toast text + error chip render.
   - Playwright spec `web/playwright/workflows/run-now-error-surface.spec.ts` that hits a known-fail vertical/workflow and asserts the toast visible.

## Acceptance criteria

- [ ] Clicking Run-now with a 403/500/network-error response surfaces a sonner toast within 1s.
- [ ] The run-history strip shows a "Last attempt failed" chip until the next run is attempted.
- [ ] The error message includes either the gateway error code or a friendly fallback.
- [ ] Component test + Playwright spec pass cold.
- [ ] Successful runs (200 response) still update the history strip with the new run row.
- [ ] PR body cites finding F08.

## May-touch

- `web/app/(app)/workflows/[verticalId]/[name]/page.tsx`
- `web/components/skycot/workflow/RunsPanel.tsx` (if separate)
- `web/lib/api/workflows.ts` (verify the executeWorkflow function surfaces errors not swallows them)
- Test files (NEW or extend)

## Out of scope

- Granting the demo principal execute capability (covered by `fix-workflow-execute-demo-principal-capability-grant`)
- CSRF wiring (covered by `fix-workflow-execute-csrf-token-wiring-verify-before-demo`)
- isLive vs source semantics (covered by `fix-workflow-detail-run-now-disabled-when-source-not-gateway`)
- Real run-history population (covered by `fix-workflow-run-history-fabricated-rows-in-mock-offline`)

## Effort

S — ~50-100 LOC + 2 tests. ~2-3 hours.
