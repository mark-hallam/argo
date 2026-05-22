# claude-followup-pr-2016-test-run-history-panel

**Owner**: claude.
**Depends on**: none.
**Authored against**: origin/main `9faa2cd58` on `2026-05-22`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-followup-pr-2016-test-run-history-panel`.

Re-author of closed-as-stale PR #2016 (`claude-web-workflow-test-run-history-panel`) after the tick-38 rebase pass hit a HARD RULE 6 ceiling collision. Original intent — `TestRunHistoryPanel` below the builder canvas with replay-from-history — is unchanged.

## Why a re-author (not a rebase)

The closed PR's resolution would have grown `web/app/(app)/workflows/[verticalId]/[name]/build/page.tsx` from 493 LOC (post-#2013 extraction of `useTestRunHandlers`) to 529 LOC, breaching the 500-LOC ceiling. The original brief assumed the file was smaller — but #2013 only shrank it modestly, and the PR's +36 LOC of replay state + history-panel JSX pushes it over.

A clean follow-up does the same panel + replay work but lands the bookkeeping behind two small extractions, keeping `page.tsx` under 500 LOC.

## Scope

Re-implement the closed PR's deliverable with these refactors:

1. **New hook**: `web/app/(app)/workflows/[verticalId]/[name]/build/useTestRunHistoryReplay.ts` (~50 LOC) — wraps `useTestRunHistory(workflowId)` + `replayEntry` state + `handleViewHistoryEntry` callback + builds the `onComplete` callback that `useTestRunHandlers` consumes. Returns `{ history, replayEntry, setReplayEntry, recordRun, handleViewHistoryEntry }`.

2. **Extend `useTestRunHandlers`**: add an optional `onComplete?: (run: CompletedTestRun) => void` field to `TestRunHandlerDeps` in `crates/.../build/builder-handlers/test-run.ts` and pass it through to the underlying `useTestRun({ onComplete })`. No behavioral change when omitted.

3. **Reuse existing `web/lib/hooks/use-test-run-history.ts`** as authored on the closed branch (preserved at commit `8ff97025c:web/lib/hooks/use-test-run-history.ts`).

4. **Mount `<TestRunHistoryPanel>` below the canvas** in `page.tsx` (~6 LOC) plus a replay-aware `<RunOverlay>` invocation (~7 LOC using a single ternary on each prop, not the duplicated overlay-render branch).

5. **Add `useTestRun.ts` `onComplete` ref-stable callback path** (preserved at commit `8ff97025c:.../build/useTestRun.ts`) — exposes `UseTestRunOptions { onComplete }` so the new hook + extended `useTestRunHandlers` can record completed runs.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `web/app/(app)/workflows/[verticalId]/[name]/build/useTestRunHistoryReplay.ts` (new file).
- `web/app/(app)/workflows/[verticalId]/[name]/build/useTestRun.ts` (extend with `UseTestRunOptions`).
- `web/app/(app)/workflows/[verticalId]/[name]/build/builder-handlers/test-run.ts` (extend `TestRunHandlerDeps` with `onComplete?`).
- `web/app/(app)/workflows/[verticalId]/[name]/build/page.tsx` (mount hook + panel + replay-aware overlay; MUST stay < 500 LOC).
- `web/components/skycot/builder/TestRunHistoryPanel.tsx` (new file, ~80 LOC; preserved at commit `8ff97025c`).
- `web/lib/hooks/use-test-run-history.ts` + `use-test-run-history.test.ts` (new files; preserved at commit `8ff97025c`).

## Files you must NOT touch

- Any file outside this list.
- `page.tsx` must end up under 500 LOC; verify with `wc -l` before push.

## Acceptance criteria

- [ ] `<TestRunHistoryPanel>` renders below the canvas on `/workflows/<v>/<name>/build`.
- [ ] Each completed test run lands in the panel (via `useTestRunHistoryReplay` → `onComplete` → `runHistory.record`).
- [ ] Clicking a panel row opens the `<RunOverlay>` in replay mode (`replayEntry`-driven props).
- [ ] Replay opens are gated when `testRun.phase === "running"`.
- [ ] `page.tsx` < 500 LOC after the patch.
- [ ] Every modified file < 500 LOC.
- [ ] `useTestRunHandlers`'s existing callers continue to work without passing `onComplete`.

## Branch + PR

- Branch: `claude-followup-pr-2016-test-run-history-panel`
- PR title: `web/workflow — builder — TestRunHistoryPanel + replay (re-author of closed #2016, ≤500 LOC)`

## Notes for the implementer

- ~250-350 LOC across ~6 files (panel + hook + replay-hook + test files + page.tsx wiring).
- The closed PR's TestRunHistoryPanel JSX + `use-test-run-history.ts` hook are good starting templates — pull them from `git show 8ff97025c:<path>`.
- Verify final `page.tsx` LOC with `wc -l web/app/\(app\)/workflows/\[verticalId\]/\[name\]/build/page.tsx` BEFORE push.
