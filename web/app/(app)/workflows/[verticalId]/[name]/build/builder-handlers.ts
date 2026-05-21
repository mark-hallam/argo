import type { Connection, Edge, Node } from "@xyflow/react";

import type { SkyNodeData, StepKind, SubWorkflowRef, WorkflowStep } from "./types";

// Pure helpers for the visual builder's connect + add-step handlers.
// Kept React-free so the validation rules can be unit-tested without
// rendering the canvas.

export type ConnectReject =
  | "incomplete"
  | "self_loop"
  | "duplicate"
  | "trigger_incoming";

export type ConnectValidation =
  | { ok: true; edge: Edge }
  | { ok: false; reason: ConnectReject };

export function validateConnection(
  connection: Connection,
  nodes: Node[],
  edges: Edge[],
): ConnectValidation {
  if (!connection.source || !connection.target) {
    return { ok: false, reason: "incomplete" };
  }
  if (connection.source === connection.target) {
    return { ok: false, reason: "self_loop" };
  }
  if (edges.some((e) => e.source === connection.source && e.target === connection.target)) {
    return { ok: false, reason: "duplicate" };
  }
  const target = nodes.find((n) => n.id === connection.target);
  const targetKind = (target?.data as SkyNodeData | undefined)?.step?.kind;
  if (targetKind === "trigger") {
    return { ok: false, reason: "trigger_incoming" };
  }
  return {
    ok: true,
    edge: {
      id: `e-${connection.source}-${connection.target}`,
      source: connection.source,
      target: connection.target,
      sourceHandle: connection.sourceHandle ?? undefined,
      targetHandle: connection.targetHandle ?? undefined,
      animated: false,
    },
  };
}

// Cheaper shape-only check ReactFlow runs during the drag (returning
// false paints the line red and prevents drop). The completed-drop
// rules (duplicate, toast feedback) live in `validateConnection`.
export function isValidConnectionShape(
  connection: Connection,
  nodes: Node[],
): boolean {
  if (!connection.source || !connection.target) return false;
  if (connection.source === connection.target) return false;
  const target = nodes.find((n) => n.id === connection.target);
  const targetKind = (target?.data as SkyNodeData | undefined)?.step?.kind;
  if (targetKind === "trigger") return false;
  return true;
}

export const STEP_KIND_DEFAULTS: Record<StepKind, { name: string; description: string }> = {
  trigger:      { name: "Trigger",       description: "Workflow entry point" },
  skill_invoke: { name: "New skill",     description: "Run a learned skill" },
  approval:     { name: "Approval gate", description: "Request human review" },
  tool_call:    { name: "Tool call",     description: "Invoke an external tool" },
  notify:       { name: "Notify",        description: "Send a notification" },
  decision:     { name: "Decision",      description: "Branch on a condition" },
  data:         { name: "Data step",     description: "Read or transform data" },
  loop:         { name: "Loop",          description: "Iterate over a list" },
  wait:         { name: "Wait",          description: "Pause for a duration" },
  sub_workflow: { name: "Sub-workflow",  description: "Invoke another workflow as a single step" },
};

export function newStepPosition(nodes: Node[]): { x: number; y: number } {
  const last = nodes[nodes.length - 1];
  return last
    ? { x: last.position.x + 220, y: last.position.y + 60 }
    : { x: 200, y: 200 };
}

export function newStepNode(
  kind: StepKind,
  nodes: Node[],
  idSuffix: string,
  subWorkflow?: SubWorkflowRef,
): Node {
  const defaults = STEP_KIND_DEFAULTS[kind];
  const step: WorkflowStep = {
    id: `step-${idSuffix}`,
    kind,
    name: subWorkflow ? `Call ${subWorkflow.displayName}` : defaults.name,
    description: subWorkflow
      ? `Invokes ${subWorkflow.verticalId}/${subWorkflow.name} (${subWorkflow.stepCount} step${subWorkflow.stepCount === 1 ? "" : "s"}).`
      : defaults.description,
    subWorkflow,
  };
  return {
    id: step.id,
    type: kind === "sub_workflow" ? "subWorkflow" : "skycot",
    position: newStepPosition(nodes),
    data: { step },
  };
}
