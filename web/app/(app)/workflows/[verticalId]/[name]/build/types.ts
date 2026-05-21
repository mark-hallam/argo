import {
  Clock,
  Database,
  GitBranch,
  Layers,
  MessageSquare,
  Play,
  Repeat,
  ShieldCheck,
  Sparkles,
} from "lucide-react";
import { createElement, type ReactNode } from "react";
import type { TriggerKind } from "@/components/workflows/triggers/types";

export type StepKind =
  | "trigger"
  | "skill_invoke"
  | "approval"
  | "tool_call"
  | "notify"
  | "decision"
  | "data"
  | "loop"
  | "wait"
  | "sub_workflow";

// Reference to another workflow invoked as a single composable unit.
// `stepCount` is captured at link-time so the SubWorkflowNode can render
// "6 steps" without resolving the target — the live builder refreshes
// it when the target spec changes.
export interface SubWorkflowRef {
  verticalId: string;
  name: string;
  displayName: string;
  stepCount: number;
}

export interface LoopConfig {
  items_path: string;
  item_var_name: string;
  max_iterations: number;
}

export interface WaitConfig {
  duration_seconds?: number;
  until_iso?: string;
  until_field?: string;
}

export interface DecisionBranch {
  trueLabel: string;
  falseLabel: string;
  falseSteps: WorkflowStep[];
  joinAt?: string;
}

export interface WorkflowStep {
  id: string;
  kind: StepKind;
  name: string;
  description: string;
  approvalGate?: { reason: string; channel: string };
  inputs?: string[];
  outputs?: string[];
  triggerKind?: TriggerKind;
  loopConfig?: LoopConfig;
  // Inline loop body — rendered indented with a dashed return edge.
  body?: WorkflowStep[];
  waitConfig?: WaitConfig;
  branch?: DecisionBranch;
  subWorkflow?: SubWorkflowRef;
}

export interface WorkflowSpec {
  name: string;
  vertical: string;
  description: string;
  version: string;
  steps: WorkflowStep[];
  risk: "approval-required" | "autonomous";
}

export interface SkyNodeData extends Record<string, unknown> {
  step: WorkflowStep;
}

export const STEP_KIND_META: Record<StepKind, { icon: ReactNode; color: string; label: string }> = {
  trigger:      { icon: createElement(Play, { size: 14 }),          color: "#10b981", label: "Trigger" },
  skill_invoke: { icon: createElement(Sparkles, { size: 14 }),      color: "#8b5cf6", label: "Skill" },
  approval:     { icon: createElement(ShieldCheck, { size: 14 }),   color: "#f59e0b", label: "Approval gate" },
  tool_call:    { icon: createElement(Database, { size: 14 }),      color: "#0891b2", label: "Tool call" },
  notify:       { icon: createElement(MessageSquare, { size: 14 }), color: "#ec4899", label: "Notify" },
  decision:     { icon: createElement(GitBranch, { size: 14 }),     color: "#6366f1", label: "Decision" },
  data:         { icon: createElement(Database, { size: 14 }),      color: "#52525b", label: "Data" },
  loop:         { icon: createElement(Repeat, { size: 14 }),        color: "#6366f1", label: "Loop" },
  wait:         { icon: createElement(Clock, { size: 14 }),         color: "#6366f1", label: "Wait" },
  sub_workflow: { icon: createElement(Layers, { size: 14 }),        color: "#0891b2", label: "Sub-workflow" },
};

export const TRUE_EDGE_STYLE = { stroke: "#8b5cf6", strokeWidth: 2 } as const;
export const FALSE_EDGE_STYLE = { stroke: "#f59e0b", strokeWidth: 2 } as const;
export const JOIN_EDGE_STYLE = { stroke: "#f59e0b", strokeWidth: 2, strokeDasharray: "5 4" } as const;
export const LOOP_RETURN_EDGE_STYLE = { stroke: "#6366f1", strokeWidth: 1.5, strokeDasharray: "6 5" } as const;

export const COL_MAIN_X = 100;
export const COL_LOOP_INDENT_X = 190;
export const COL_BRANCH_X = 460;
export const ROW_HEIGHT = 200;
export const ROW_START_Y = 60;

export interface PaletteGroup {
  label: string;
  kinds: StepKind[];
}

export const PALETTE_GROUPS: PaletteGroup[] = [
  { label: "Sources", kinds: ["trigger"] },
  { label: "Action", kinds: ["skill_invoke", "tool_call", "notify"] },
  { label: "Logic", kinds: ["decision", "loop", "wait"] },
  { label: "Composition", kinds: ["sub_workflow"] },
  { label: "Governance", kinds: ["approval"] },
  { label: "Data", kinds: ["data"] },
];
