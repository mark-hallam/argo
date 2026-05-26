# fix-workflow-run-history-fabricated-rows-in-mock-offline

**Owner**: claude.
**Authored against**: origin/main `4585055d32eba8a24f5872354c0225246c0aa250` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-workflow-run-history-fabricated-rows-in-mock-offline`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-27-0623/scene-4-and-5-claim-4-and-5.md` F9 [P1].

🟠 **P1 — `/workflows/[v]/[n]` viewer falls through to seed data when the gateway returns no workflow row. The page sets `isDemoRunHistory = source === "mock-offline" || source === "mock-bypass"` and shows a "DEMO RUN HISTORY" pill — honest about the mode — but the actual rows in the table are fabricated. If the Scene 5 demo drills into the run-history table during the workflow-detail pass-through, the camera reads invented timestamps + run statuses for runs that never happened. Per `feedback_no_theater_in_demo` (operator-locked 2026-05-24) this is function-side fabrication the camera sees.**

## Root cause

`web/app/(app)/workflows/[verticalId]/[name]/page.tsx:104-106`:

```ts
const source = ...; // resolved from API response or fallback path
const isDemoRunHistory = source === "mock-offline" || source === "mock-bypass";
```

The page renders a `<WorkflowRunsTable>` populated from `workflow.runs` even when `isDemoRunHistory === true`. The "DEMO RUN HISTORY" pill (rendered when `isDemoRunHistory`) is an honest mode-indicator, but the rows themselves are sample data invented by `web/lib/sample-workflows.ts` to look real (timestamps, run statuses, durations) without corresponding to actual runs.

Per the function-vs-dressing rule + `feedback_no_theater_in_demo`:
- Dressing-side: a sample table that's plainly an illustration (e.g. `<div className="opacity-40">Sample table</div>`) is acceptable.
- Function-side: a real-shaped table with timestamps and statuses that look indistinguishable from production data is theater. The "DEMO RUN HISTORY" pill is a partial mitigation but not enough — the camera lingers on the rows during Scene 5 drill-in, not the pill.

Two adjacent briefs touch the area:
- `fix-workflow-detail-skip-runs-fetch-in-mock-offline` — suppresses the fetch in mock-offline mode, but the page still falls through to `workflow.runs` from `SAMPLE_WORKFLOW_SPECS`.
- `fix-workflow-execute-csrf-token-wiring-verify-before-demo` — covers the Run-now POST path, not the historical-runs table.

This brief is the missing piece: when in `mock-offline` / `mock-bypass`, do not render the run-history TABLE with fabricated rows. Render an empty-state instead.

## Findings this brief closes

- scene-4-and-5-claim-4-and-5 F9 [P1] — `/workflows/[v]/[n]` viewer's run history pulls from `mock-offline` source and renders fabricated rows

## Fix recipe

1. **Suppress the runs table in mock-offline / mock-bypass mode** at `web/app/(app)/workflows/[verticalId]/[name]/page.tsx` (around line 104-106 plus the `<WorkflowRunsTable>` mount):
   ```tsx
   {isDemoRunHistory ? (
     <WorkflowRunsEmptyState
       label="No runs yet."
       hint="Click Run now to fire the first execution."
       data-testid="workflow-runs-empty-demo"
     />
   ) : (
     <WorkflowRunsTable rows={workflow.runs} />
   )}
   ```
2. **Reuse the existing pattern** — `/audit` already has the same shape when its gateway path returns empty: it renders an empty-state instead of fabricated rows. Mirror that component's tone (`web/components/skycot/audit/AuditEmptyState.tsx` or sibling).
3. **Remove the runs[] field from the sample-workflows fixtures** (`web/lib/sample-workflows.ts:64-149`) — the steps[] should remain (rendered on the canvas) but `runs[]` should be empty for all 8 seed workflows. Anything that wants to demo runs should seed them via `argo demo seed --tenant <id> --workflow-runs` (covered by `fix-demo-seed-publish-workflow-definitions-and-fire-one-execution-per-persona`).
4. **Vitest**: render the workflow-detail page with `source="mock-offline"` → assert the table is NOT in the DOM, the empty-state IS.

## Acceptance criteria

- [ ] When `isDemoRunHistory` is true, the run-history table is NOT rendered with fabricated rows.
- [ ] An empty-state component renders in its place ("No runs yet. Click Run now to fire the first execution.").
- [ ] `web/lib/sample-workflows.ts` no longer carries fabricated `runs[]` content on the 8 seed workflows.
- [ ] Vitest pin asserts the empty-state path on `source="mock-offline"`.
- [ ] The "DEMO RUN HISTORY" pill can stay (operator-decisional) but is no longer the only honesty signal.

## May-touch

- `web/app/(app)/workflows/[verticalId]/[name]/page.tsx:104-110` (conditional render)
- `web/components/skycot/workflows/WorkflowRunsEmptyState.tsx` (NEW or reuse)
- `web/lib/sample-workflows.ts:64-149` (remove fabricated `runs[]`)
- Vitest pin in `web/app/(app)/workflows/[verticalId]/[name]/__tests__/`

## Out of scope

- Run-now button CSRF + principal wiring (covered by `fix-workflow-execute-csrf-token-wiring-verify-before-demo`).
- Suppressing the runs fetch when in mock-offline (covered by `fix-workflow-detail-skip-runs-fetch-in-mock-offline`).
- Demo seed firing real runs (covered by `fix-demo-seed-publish-workflow-definitions-and-fire-one-execution-per-persona`).
- DEMO-RUN-HISTORY pill copy (operator can decide separately).

## Effort

~50-100 LOC: page conditional + new empty-state component + sample-workflow runs[] cleanup + vitest. ~1.5 hours. P1 because Scene 5's narration walks past the run-history table; the camera sees the table rows.
