import { useCallback, useState } from "react";

import {
  useTestRunHistory,
  type TestRunHistoryEntry,
  type UseTestRunHistoryResult,
} from "@/lib/hooks/use-test-run-history";

import type {
  CompletedTestRun,
  StepInput,
  UseTestRunResult,
} from "../useTestRun";

export interface TestRunHistoryHandlers {
  runHistory: UseTestRunHistoryResult;
  replayEntry: TestRunHistoryEntry | null;
  handleRunComplete: (run: CompletedTestRun) => void;
  openReplay: (entry: TestRunHistoryEntry) => void;
  closeReplay: () => void;
}

/**
 * Bundles the run-history side of the builder's test-run flow:
 * persisting completed runs to the localStorage-backed history, plus
 * the `replayEntry` slot that re-opens RunOverlay against a previous
 * run. The page wires `handleRunComplete` into `useTestRunHandlers` so
 * each finished run records itself; the panel below the canvas reads
 * `runHistory.entries` and calls `openReplay` to surface a historical
 * run in the overlay.
 */
export function useTestRunHistoryHandlers(
  workflowId: string,
): TestRunHistoryHandlers {
  const runHistory = useTestRunHistory(workflowId);
  const [replayEntry, setReplayEntry] = useState<TestRunHistoryEntry | null>(null);

  const handleRunComplete = useCallback(
    (run: CompletedTestRun) => {
      const id =
        typeof crypto !== "undefined" && "randomUUID" in crypto
          ? crypto.randomUUID()
          : `run-${run.endedAt}`;
      runHistory.record({
        id,
        startedAt: run.startedAt,
        endedAt: run.endedAt,
        status: run.status,
        transportError: run.transportError ?? undefined,
        steps: run.steps,
        stepStates: run.stepStates,
        summary: run.summary,
      });
    },
    [runHistory],
  );

  const openReplay = useCallback(
    (entry: TestRunHistoryEntry) => setReplayEntry(entry),
    [],
  );
  const closeReplay = useCallback(() => setReplayEntry(null), []);

  return {
    runHistory,
    replayEntry,
    handleRunComplete,
    openReplay,
    closeReplay,
  };
}

/**
 * Resolve RunOverlay's props for either a live test run or a replay of
 * a historical entry. Replay wins; otherwise we surface the live
 * `useTestRun` state.
 */
export function buildRunOverlayProps(
  history: TestRunHistoryHandlers,
  testRun: UseTestRunResult,
  liveSteps: StepInput[],
) {
  const r = history.replayEntry;
  if (r) {
    return {
      phase: r.status,
      steps: r.steps,
      stepStates: r.stepStates,
      summary: r.summary,
      transportError: r.transportError ?? null,
      onDismiss: history.closeReplay,
    };
  }
  return {
    phase: testRun.phase,
    steps: liveSteps,
    stepStates: testRun.stepStates,
    summary: testRun.summary,
    transportError: testRun.transportError,
    onDismiss: testRun.reset,
  };
}
