import type { Edge, Node } from "@xyflow/react";
import {
  COL_BRANCH_X,
  COL_LOOP_INDENT_X,
  COL_MAIN_X,
  FALSE_EDGE_STYLE,
  JOIN_EDGE_STYLE,
  LOOP_RETURN_EDGE_STYLE,
  ROW_HEIGHT,
  ROW_START_Y,
  TRUE_EDGE_STYLE,
  type WaitConfig,
  type WorkflowStep,
} from "./types";

export function formatWaitSummary(cfg: WaitConfig | undefined): string {
  if (!cfg) return "Wait";
  if (cfg.duration_seconds != null) {
    const s = cfg.duration_seconds;
    if (s < 60) return `Wait ${s}s`;
    if (s < 3600) return `Wait ${Math.round(s / 60)}m`;
    return `Wait ${Math.round(s / 3600)}h`;
  }
  if (cfg.until_iso) {
    // Render "Wait until 14:00 UTC" if it parses as a time of day; otherwise fall back to raw value.
    const d = new Date(cfg.until_iso);
    if (!Number.isNaN(d.getTime())) {
      const hh = String(d.getUTCHours()).padStart(2, "0");
      const mm = String(d.getUTCMinutes()).padStart(2, "0");
      return `Wait until ${hh}:${mm} UTC`;
    }
    return `Wait until ${cfg.until_iso}`;
  }
  if (cfg.until_field) return `Wait until ${cfg.until_field}`;
  return "Wait";
}

// Layout the workflow into React Flow nodes + edges. Three kinds of step
// expand into extra geometry:
//   • Loop steps with a `body` render the inner steps indented one column
//     beneath the loop header, with a dashed indigo return edge from the
//     last body step back to the header.
//   • Decision steps with a `branch` keep the "true" path running down the
//     main column and place `falseSteps` in a parallel branch column to
//     the right, joined by amber edges.
//   • A `branch.joinAt` re-merges the false path back into the main column
//     via a dashed amber "rejoin" edge to the target node.
export function stepsToFlow(steps: WorkflowStep[]): { nodes: Node[]; edges: Edge[] } {
  const nodes: Node[] = [];
  const edges: Edge[] = [];
  let y = ROW_START_Y;

  const emit = (step: WorkflowStep, x: number) => {
    nodes.push({
      id: step.id,
      type: "skycot",
      position: { x, y },
      data: { step },
    });
    y += ROW_HEIGHT;
  };

  // First pass: emit main-column steps + inline loop bodies, and the
  // forward main-column edges. Track each main step's y so the second
  // pass can position decision-branch nodes relative to the decision.
  const mainStepY = new Map<string, number>();
  let prevMainId: string | null = null;
  let prevMainStep: WorkflowStep | null = null;

  for (const step of steps) {
    mainStepY.set(step.id, y);
    emit(step, COL_MAIN_X);

    if (prevMainId && prevMainStep) {
      const fromDecision = prevMainStep.kind === "decision" && !!prevMainStep.branch;
      edges.push({
        id: `e-${prevMainId}-${step.id}`,
        source: prevMainId,
        target: step.id,
        sourceHandle: fromDecision ? "true" : undefined,
        animated: true,
        style: TRUE_EDGE_STYLE,
        label: fromDecision ? prevMainStep.branch!.trueLabel : undefined,
        labelStyle: fromDecision
          ? { fontFamily: "JetBrains Mono, monospace", fontSize: 10, fill: "#8b5cf6", fontWeight: 700 }
          : undefined,
        labelBgStyle: fromDecision ? { fill: "white", fillOpacity: 0.92 } : undefined,
      });
    }
    prevMainId = step.id;
    prevMainStep = step;

    if (step.kind === "loop" && step.body && step.body.length > 0) {
      let lastBodyId = step.id;
      for (const child of step.body) {
        emit(child, COL_LOOP_INDENT_X);
        edges.push({
          id: `e-${lastBodyId}-${child.id}`,
          source: lastBodyId,
          target: child.id,
          animated: true,
          style: TRUE_EDGE_STYLE,
        });
        lastBodyId = child.id;
      }
      edges.push({
        id: `r-${lastBodyId}-${step.id}`,
        source: lastBodyId,
        target: step.id,
        animated: false,
        style: LOOP_RETURN_EDGE_STYLE,
        label: "↻ loop back",
        labelStyle: {
          fontFamily: "JetBrains Mono, monospace",
          fontSize: 9,
          fill: "#6366f1",
          letterSpacing: "0.08em",
        },
        labelBgStyle: { fill: "white" },
        labelBgPadding: [4, 6],
        labelBgBorderRadius: 4,
      });
    }
  }

  // Second pass: render decision-branch falseSteps in the branch column.
  for (const step of steps) {
    if (step.kind !== "decision" || !step.branch || step.branch.falseSteps.length === 0) continue;
    const branch = step.branch;
    const baseY = mainStepY.get(step.id) ?? ROW_START_Y;
    branch.falseSteps.forEach((bStep, j) => {
      nodes.push({
        id: bStep.id,
        type: "skycot",
        position: { x: COL_BRANCH_X, y: baseY + (j + 1) * ROW_HEIGHT },
        data: { step: bStep },
      });
      const sourceId = j === 0 ? step.id : branch.falseSteps[j - 1].id;
      edges.push({
        id: `e-${sourceId}-${bStep.id}`,
        source: sourceId,
        target: bStep.id,
        sourceHandle: j === 0 ? "false" : undefined,
        animated: true,
        style: FALSE_EDGE_STYLE,
        label: j === 0 ? branch.falseLabel : undefined,
        labelStyle: j === 0
          ? { fontFamily: "JetBrains Mono, monospace", fontSize: 10, fill: "#b45309", fontWeight: 700 }
          : undefined,
        labelBgStyle: j === 0 ? { fill: "white", fillOpacity: 0.92 } : undefined,
      });
    });
    if (branch.joinAt) {
      const last = branch.falseSteps[branch.falseSteps.length - 1];
      edges.push({
        id: `e-${last.id}-${branch.joinAt}-join`,
        source: last.id,
        target: branch.joinAt,
        animated: true,
        style: JOIN_EDGE_STYLE,
        label: "rejoin",
        labelStyle: { fontFamily: "JetBrains Mono, monospace", fontSize: 9.5, fill: "#b45309", fontWeight: 700 },
        labelBgStyle: { fill: "white", fillOpacity: 0.92 },
      });
    }
  }

  return { nodes, edges };
}
