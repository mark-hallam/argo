# web-workflow-test-run-history-panel

**Authored against**: `origin/main` `1617b41ea` on `2026-05-21`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-workflow-test-run-history-panel`.

## Summary

#1971 just landed live-execution overlay: NDJSON streaming + `useTestRun` hook + `RunOverlay`. A user can click "Test run" and see realtime progress. After the run, the overlay closes and the result is lost. Surface a "Recent test runs" panel below the canvas showing the last 5 runs with status + duration + ability to re-open the overlay for a completed run.

Pre-condition: **#1971 MERGED ✅** (NDJSON infrastructure + RunOverlay exist).

## Acceptance

- [ ] `/workflows/<vid>/<name>/build` shows a "Recent test runs" panel (below the canvas, or in a collapsible drawer).
- [ ] Each run row: timestamp, success/fail badge, duration, "View" button that re-opens the RunOverlay with that run's events.
- [ ] Runs persist in localStorage (v1 fallback — gateway persistence in a follow-up).
- [ ] Cap at 10 most-recent runs; older entries evicted FIFO.
- [ ] Empty state: "No test runs yet" with a hint to click the Test Run button.
- [ ] No regression on the current Test Run flow or RunOverlay.
- [ ] One unit test: log 12 runs → assert only 10 remain, oldest 2 evicted.

## May touch

- `web/app/(app)/workflows/[verticalId]/[name]/build/page.tsx` (mount the panel — but DON'T grow it; extract into a component)
- `web/components/skycot/builder/TestRunHistoryPanel.tsx` (new)
- `web/lib/hooks/use-test-run-history.ts` (new — localStorage wrapper)
- `web/lib/hooks/use-test-run.ts` (extend to emit on-completion event so the history hook can capture)

## Must NOT touch

- The NDJSON streaming infrastructure (#1971 settled).
- The RunOverlay component (#1971 settled).
- `web/app/api/workflows/[verticalId]/[name]/test-run/route.ts` (#1971 settled).

## Notes

- Sizing: 200–300 LOC.
- Pre-flight: read `web/lib/hooks/use-test-run.ts` from #1971 to see the canonical event shape.
- Operator memory: build/page.tsx is at 687 LOC — extract the panel into a sibling component, don't inline.
- Demo-visible polish; pairs with the broader #1971 storyline.
