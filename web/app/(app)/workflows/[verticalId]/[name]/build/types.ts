import {
  Clock,
  Code2,
  Database,
  GitBranch,
  Globe,
  Inbox,
  Layers,
  Mail,
  MessageSquare,
  MousePointer2,
  Play,
  Repeat,
  ShieldCheck,
  Slack,
  Sparkles,
  Webhook,
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
  | "sub_workflow"
  | "http_request"
  | "code";

export type HttpMethod = "GET" | "POST" | "PUT" | "PATCH" | "DELETE" | "HEAD" | "OPTIONS";

export const HTTP_METHODS: readonly HttpMethod[] = [
  "GET",
  "POST",
  "PUT",
  "PATCH",
  "DELETE",
  "HEAD",
  "OPTIONS",
];

export type HttpAuthMode = "none" | "bearer" | "basic" | "custom_header";

export interface HttpHeader {
  key: string;
  value: string;
}

export interface HttpRequestConfig {
  method: HttpMethod;
  url: string;
  headers: HttpHeader[];
  body: string;
  auth: HttpAuthMode;
  authToken: string;
  authUsername: string;
  authPassword: string;
  authHeaderName: string;
  authHeaderValue: string;
  timeoutSeconds: number;
}

export type CodeLanguage = "javascript" | "python";

export interface CodeConfig {
  language: CodeLanguage;
  code: string;
}

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
  httpRequest?: HttpRequestConfig;
  code?: CodeConfig;
}

export interface WorkflowSpec {
  name: string;
  vertical: string;
  description: string;
  version: string;
  steps: WorkflowStep[];
  risk: "approval-required" | "autonomous";
}

export type NodeRunStatus = "pending" | "running" | "success" | "error";

export interface NodeRunState {
  status: NodeRunStatus;
  durationMs?: number;
  tokens?: number;
  response?: string;
  error?: string;
}

export interface SkyNodeData extends Record<string, unknown> {
  step: WorkflowStep;
  runState?: NodeRunState;
  // Set true when input-data has been pinned for this step's id. Used by
  // BuilderCanvasNode to render the inline `PINNED` chip. Projected onto
  // nodes by `decorateWithPins()` so the chip survives node-state
  // rebuilds without prop-drilling pin state through every component.
  pinned?: boolean;
}

// Free-form annotation placed on the canvas. Persists with the workflow
// document (gateway `notes[]` field is a follow-up brief — for v1 these
// live in client session state). Rendered as a draggable yellow
// Post-it via `StickyNoteNode`.
export interface StickyNote {
  id: string;
  text: string;
  x: number;
  y: number;
}

// Data payload attached to ReactFlow nodes whose `type === "sticky"`.
// Callbacks plumb edit + delete back to the annotations hook so the
// note can be a fully self-contained component in the node registry.
export interface SkyNoteData extends Record<string, unknown> {
  noteId: string;
  text: string;
  onTextChange: (id: string, text: string) => void;
  onDelete: (id: string) => void;
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
  http_request: { icon: createElement(Globe, { size: 14 }),         color: "#0891b2", label: "HTTP request" },
  code:         { icon: createElement(Code2, { size: 14 }),         color: "#3b82f6", label: "Code" },
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

// Drag-and-drop MIME type for palette items. Picked to avoid collision
// with React Flow's own pointer-event drag handles on canvas nodes.
export const PALETTE_DRAG_MIME = "application/x-skycot-palette-item";

// A single draggable entry in the left sidebar palette. Maps to a
// `StepKind` plus optional `triggerKind`/name overrides so a single
// `trigger` kind can spawn manual / schedule / webhook / email / slack
// / form variants without each needing its own `StepKind` enum value.
export interface PaletteItem {
  id: string;
  kind: StepKind;
  label: string;
  description: string;
  triggerKind?: TriggerKind;
}

export interface SidebarPaletteGroup {
  label: string;
  items: PaletteItem[];
}

export const SIDEBAR_PALETTE_GROUPS: SidebarPaletteGroup[] = [
  {
    label: "Triggers",
    items: [
      { id: "trigger-manual",   kind: "trigger", label: "Manual run",    description: "Run on demand from the builder", triggerKind: "manual" },
      { id: "trigger-schedule", kind: "trigger", label: "Schedule",      description: "Cron-style recurring trigger",    triggerKind: "schedule" },
      { id: "trigger-webhook",  kind: "trigger", label: "Webhook",       description: "HTTP endpoint trigger",           triggerKind: "webhook" },
      { id: "trigger-email",    kind: "trigger", label: "Email",         description: "Inbox-watch trigger",             triggerKind: "email" },
      { id: "trigger-slack",    kind: "trigger", label: "Slack",         description: "Slack event trigger",             triggerKind: "slack" },
      { id: "trigger-form",     kind: "trigger", label: "Web form",      description: "Public form-submit trigger",      triggerKind: "form" },
    ],
  },
  {
    label: "Skills",
    items: [
      { id: "skill-invoke", kind: "skill_invoke", label: "Invoke skill", description: "Run a learned skill from the marketplace" },
    ],
  },
  {
    label: "Logic",
    items: [
      { id: "logic-decision", kind: "decision", label: "Decision", description: "Branch on a condition" },
      { id: "logic-loop",     kind: "loop",     label: "Loop",     description: "Iterate over a list" },
      { id: "logic-wait",     kind: "wait",     label: "Wait",     description: "Pause for a duration" },
    ],
  },
  {
    label: "Approval gates",
    items: [
      { id: "approval-slack", kind: "approval", label: "Slack approval", description: "Approval via Slack DM" },
      { id: "approval-email", kind: "approval", label: "Email approval", description: "Approval via email" },
      { id: "approval-web",   kind: "approval", label: "Web approval",   description: "Approval via web inbox" },
    ],
  },
  {
    label: "Tools",
    items: [
      { id: "tool-http",    kind: "http_request", label: "HTTP request",   description: "Call an external API" },
      { id: "tool-browser", kind: "tool_call",    label: "Browser action (Playwright)", description: "Drive a real browser with Playwright" },
      { id: "tool-code",    kind: "code",         label: "Code",           description: "Run inline code" },
    ],
  },
  {
    label: "Notify",
    items: [
      { id: "notify-slack", kind: "notify", label: "Slack DM",  description: "Send a Slack message" },
      { id: "notify-email", kind: "notify", label: "Email",     description: "Send an email" },
      { id: "notify-inbox", kind: "notify", label: "Web inbox", description: "Post to the in-app inbox" },
    ],
  },
];

// Per-item icon override (overrides STEP_KIND_META so each palette
// variant can render a distinct glyph — e.g. all three Notify variants
// share the same `notify` step kind but the palette shows Slack / Mail
// / Inbox icons).
export const PALETTE_ITEM_ICON: Record<string, ReactNode> = {
  "trigger-manual":   createElement(MousePointer2, { size: 14 }),
  "trigger-schedule": createElement(Clock,         { size: 14 }),
  "trigger-webhook":  createElement(Webhook,       { size: 14 }),
  "trigger-email":    createElement(Mail,          { size: 14 }),
  "trigger-slack":    createElement(Slack,         { size: 14 }),
  "trigger-form":     createElement(Globe,         { size: 14 }),
  "skill-invoke":     createElement(Sparkles,      { size: 14 }),
  "logic-decision":   createElement(GitBranch,     { size: 14 }),
  "logic-loop":       createElement(Repeat,        { size: 14 }),
  "logic-wait":       createElement(Clock,         { size: 14 }),
  "approval-slack":   createElement(Slack,         { size: 14 }),
  "approval-email":   createElement(Mail,          { size: 14 }),
  "approval-web":     createElement(ShieldCheck,   { size: 14 }),
  "tool-http":        createElement(Globe,         { size: 14 }),
  "tool-browser":     createElement(MousePointer2, { size: 14 }),
  "tool-code":        createElement(Code2,         { size: 14 }),
  "notify-slack":     createElement(Slack,         { size: 14 }),
  "notify-email":     createElement(Mail,          { size: 14 }),
  "notify-inbox":     createElement(Inbox,         { size: 14 }),
};
