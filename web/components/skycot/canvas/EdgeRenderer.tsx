"use client";

import { memo } from "react";
import {
  BaseEdge,
  EdgeLabelRenderer,
  getSmoothStepPath,
  type EdgeProps,
} from "@xyflow/react";
import type { EdgeCond } from "@/lib/types/skill-ir";

/**
 * Edge styling per L1 `EdgeCond.kind`. Stroke colors map onto the
 * standard light-bulb of "always = neutral, success = green, failure
 * = red, pattern = amber" so the viewer reads cleanly without a
 * legend. Pattern arms additionally show their match string as an
 * inline label.
 */

export const EDGE_COND_STROKE: Record<EdgeCond["kind"], string> = {
  always: "var(--border)",
  "on-success": "var(--success, #16a34a)",
  "on-failure": "var(--destructive, #dc2626)",
  "on-pattern": "var(--warning, #d97706)",
};

export interface SkillEdgeData extends Record<string, unknown> {
  cond: EdgeCond;
}

function EdgeRendererImpl(props: EdgeProps) {
  const data = props.data as SkillEdgeData | undefined;
  const cond: EdgeCond = data?.cond ?? { kind: "always" };
  const stroke = EDGE_COND_STROKE[cond.kind];
  const dashed = cond.kind === "on-failure";
  const [path, labelX, labelY] = getSmoothStepPath({
    sourceX: props.sourceX,
    sourceY: props.sourceY,
    sourcePosition: props.sourcePosition,
    targetX: props.targetX,
    targetY: props.targetY,
    targetPosition: props.targetPosition,
  });

  const labelText =
    cond.kind === "on-pattern"
      ? cond.pattern
      : cond.kind === "always"
        ? null
        : cond.kind.replace("on-", "");

  return (
    <>
      <BaseEdge
        id={props.id}
        path={path}
        style={{
          stroke,
          strokeWidth: 1.5,
          strokeDasharray: dashed ? "4 3" : undefined,
        }}
      />
      {labelText ? (
        <EdgeLabelRenderer>
          <div
            data-testid={`canvas-edge-label-${props.id}`}
            data-cond={cond.kind}
            style={{
              position: "absolute",
              transform: `translate(-50%, -50%) translate(${labelX}px, ${labelY}px)`,
              pointerEvents: "all",
            }}
            className="rounded-[var(--radius-sm)] border border-[var(--border)] bg-[var(--background)] px-2 py-0.5 text-[10px] font-medium text-[var(--muted-foreground)] shadow-sm"
          >
            {labelText}
          </div>
        </EdgeLabelRenderer>
      ) : null}
    </>
  );
}

export const EdgeRenderer = memo(EdgeRendererImpl);
EdgeRenderer.displayName = "SkillCanvasEdgeRenderer";
