import type { Node } from "@xyflow/react";
import { useCallback, useMemo } from "react";
import { toast } from "sonner";

import type { useBuilderAnnotations } from "@/lib/hooks/use-builder-annotations";

import {
  type SkyNodeData,
  type WorkflowStep,
} from "../types";
import {
  useTestRun,
  type CompletedTestRun,
  type StepInput,
  type UseTestRunResult,
} from "../useTestRun";

type Annotations = ReturnType<typeof useBuilderAnnotations>;

export interface TestRunHandlerDeps {
  verticalId: string;
  name: string;
  nodes: Node[];
  currentSteps: WorkflowStep[];
  annotations: Annotations;
  /**
   * Optional completion sink so the page-level history panel can
   * capture each finished run for replay after the overlay dismisses.
   */
  onRunComplete?: (run: CompletedTestRun) => void;
}

export interface TestRunHandlers {
  testRun: UseTestRunResult;
  stepInputs: StepInput[];
  nodesWithRunState: Node[];
  handleTestRun: () => void;
}

/**
 * Test-run + run-state-decoration handlers for the workflow builder.
 *
 * Wraps `useTestRun` (the live step-by-step execution overlay) and
 * derives the `nodesWithRunState` array — nodes painted with their
 * latest run state and merged with annotation sticky-notes — that the
 * canvas renders. Toasts when pinned inputs override live data, which
 * is a usability quirk worth flagging once per run.
 */
export function useTestRunHandlers(deps: TestRunHandlerDeps): TestRunHandlers {
  const { verticalId, name, nodes, currentSteps, annotations, onRunComplete } = deps;

  const stepInputs = useMemo<StepInput[]>(
    () => currentSteps.map((s) => ({ id: s.id, name: s.name, kind: s.kind })),
    [currentSteps],
  );
  const testRun = useTestRun(verticalId, name, stepInputs, {
    onComplete: onRunComplete,
  });

  const nodesWithRunState = useMemo(() => {
    const decorated = annotations.decorateWithPins(
      nodes.map((n) => {
        const sky = n.data as SkyNodeData;
        const next = testRun.stepStates[sky.step.id];
        if (sky.runState === next) return n;
        return { ...n, data: { ...sky, runState: next } };
      }),
    );
    return [...decorated, ...annotations.noteNodes];
  }, [nodes, testRun.stepStates, annotations]);

  const pinnedStepNames = useMemo(() => {
    const ids = Object.keys(annotations.pinnedData);
    if (ids.length === 0) return [] as string[];
    const idToName = new Map(currentSteps.map((s) => [s.id, s.name] as const));
    return ids.map((id) => idToName.get(id) ?? id);
  }, [annotations.pinnedData, currentSteps]);

  const handleTestRun = useCallback(() => {
    if (testRun.phase === "running") return;
    if (stepInputs.length === 0) {
      toast.error("Add a step before running a test.");
      return;
    }
    if (pinnedStepNames.length > 0) {
      toast.message(`Using pinned data for: ${pinnedStepNames.join(", ")}`, {
        description: "Live inputs are overridden for the pinned steps.",
      });
    }
    testRun.start();
  }, [testRun, stepInputs.length, pinnedStepNames]);

  return { testRun, stepInputs, nodesWithRunState, handleTestRun };
}
