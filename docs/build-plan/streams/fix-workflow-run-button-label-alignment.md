# fix-workflow-run-button-label-alignment

**Owner**: claude or codex.
**Authored against**: origin/main `0653c478d` on `2026-05-25`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-workflow-run-button-label-alignment`.
**Source**: e2e-ux-audit Agent 3 finding F7 (`docs/skycot/ux-audits/2026-05-25-1100-scene-4-and-5-claim-4-and-5.md`).

🟡 **P1 polish — narration says "Run now"; button says "Run workflow". Vocabulary mismatch the camera sees.**

## Root cause

`web/app/(app)/workflows/[verticalId]/[name]/page.tsx:280` renders the workflow-detail Run button with label "Run workflow" / "Running…". The Scene 5 scope brief + the v1.0 runbook narration both refer to **"Run now"**. The button does correctly call `executeWorkflow` (POST `/api/v1/verticals/{v}/workflows/{n}/execute`) — the wiring is fine. Only the label is misaligned.

Also aligns with the "Run now" affordance on `/workers/<id>/responsibilities` per `fix-worker-detail-responsibilities-schedules-tab-end-to-end` — for consistency, every manual-trigger affordance across the SPA should read the same word.

## Findings this brief closes

- Agent 3 F7 [P1] — Workflow detail page button label is "Run workflow", not "Run now"

## Fix recipe

1. Change the button label at `web/app/(app)/workflows/[verticalId]/[name]/page.tsx:280`:
   - idle: `"Run now"` (was `"Run workflow"`)
   - in-flight: `"Running…"` (unchanged)
2. Verify the sibling "Run now" affordance on `/workers/<id>/responsibilities` (once shipped per `fix-worker-detail-responsibilities-schedules-tab-end-to-end`) uses the same label.
3. Sweep `grep -r "Run workflow" web/` for stragglers and align.

## Acceptance criteria

- [ ] `/workflows/<vertical>/<name>` workflow detail page renders a button labelled "Run now" (not "Run workflow").
- [ ] Loading state still reads "Running…".
- [ ] No other surfaces in `web/` still say "Run workflow" (sweep clean).
- [ ] Component test (`WorkflowDetail.test.tsx`) snapshot pins the label.

## May-touch

- `web/app/(app)/workflows/[verticalId]/[name]/page.tsx:280`
- Any sibling page that shows "Run workflow" (run a sweep)
- Snapshot test

## Out of scope

- The Run now button on `/workers/<id>/responsibilities` (separate brief: `fix-worker-detail-responsibilities-schedules-tab-end-to-end` — that brief should use the same label "Run now")
- The actual `executeWorkflow` API wiring (already correct)

## Effort

~5-15 LOC. ~10-20 min.
