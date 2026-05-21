/**
 * Step ↔ TOML encoder for the workflow builder Save flow.
 *
 * The gateway's `POST /api/v1/verticals/{vid}/workflows/{name}` endpoint
 * accepts a raw `workflow.toml` body. This module encodes the
 * builder's in-memory `WorkflowStep[]` to that wire shape and
 * round-trips it back (decoder) so the canvas state survives a Save
 * → reload through the gateway.
 *
 * Phase 1 ships the four fields every visual step carries: `id`,
 * `name`, `kind`, `description`. Advanced step shape (approval gate,
 * loop config, decision branches, sub-workflow refs) lands in a
 * follow-up brief; encoder/decoder are forward-compatible — unknown
 * keys are silently passed through on encode (none today) and
 * ignored on decode.
 */
import type { StepKind, WorkflowStep } from "./types";

const STEP_KINDS: ReadonlySet<StepKind> = new Set<StepKind>([
  "trigger",
  "skill_invoke",
  "approval",
  "tool_call",
  "notify",
  "decision",
  "data",
  "loop",
  "wait",
  "sub_workflow",
]);

function escapeTomlString(s: string): string {
  return s.replace(/\\/g, "\\\\").replace(/"/g, '\\"').replace(/\n/g, "\\n");
}

function tomlString(value: string): string {
  return `"${escapeTomlString(value)}"`;
}

export interface EncodeOptions {
  workflowName: string;
  workflowDescription?: string;
}

export function encodeStepsToToml(
  steps: WorkflowStep[],
  opts: EncodeOptions,
): string {
  const lines: string[] = ["[workflow]"];
  lines.push(`name = ${tomlString(opts.workflowName)}`);
  if (opts.workflowDescription) {
    lines.push(`description = ${tomlString(opts.workflowDescription)}`);
  }
  for (const step of steps) {
    lines.push("", "[[step]]");
    lines.push(`id = ${tomlString(step.id)}`);
    lines.push(`name = ${tomlString(step.name)}`);
    lines.push(`kind = ${tomlString(step.kind)}`);
    if (step.description) {
      lines.push(`description = ${tomlString(step.description)}`);
    }
  }
  lines.push("");
  return lines.join("\n");
}

export interface DecodedWorkflow {
  workflowName: string | null;
  workflowDescription: string | null;
  steps: WorkflowStep[];
}

interface PendingStep {
  id?: string;
  name?: string;
  kind?: string;
  description?: string;
}

function unescapeTomlString(s: string): string {
  let out = "";
  for (let i = 0; i < s.length; i++) {
    if (s[i] === "\\" && i + 1 < s.length) {
      const next = s[i + 1];
      if (next === "\\") { out += "\\"; i += 1; continue; }
      if (next === "\"") { out += "\""; i += 1; continue; }
      if (next === "n") { out += "\n"; i += 1; continue; }
      if (next === "t") { out += "\t"; i += 1; continue; }
    }
    out += s[i];
  }
  return out;
}

function stripQuoted(value: string): string {
  if (value.length >= 2 && value.startsWith("\"") && value.endsWith("\"")) {
    return unescapeTomlString(value.slice(1, -1));
  }
  return value;
}

function commit(pending: PendingStep, idx: number): WorkflowStep | null {
  const name = pending.name?.trim();
  if (!name) return null;
  const id = pending.id?.trim() || `step-${idx + 1}`;
  const rawKind = pending.kind?.trim() ?? "";
  const kind: StepKind = STEP_KINDS.has(rawKind as StepKind)
    ? (rawKind as StepKind)
    : "tool_call";
  return { id, kind, name, description: pending.description ?? "" };
}

/**
 * Decode a TOML document produced by `encodeStepsToToml`. Tolerant of
 * comments, blank lines, and unknown sections so encoder evolution
 * never breaks a half-written document.
 */
export function decodeStepsFromToml(toml: string): DecodedWorkflow {
  let workflowName: string | null = null;
  let workflowDescription: string | null = null;
  const steps: WorkflowStep[] = [];
  let section: "none" | "workflow" | "step" = "none";
  let pending: PendingStep | null = null;
  for (const rawLine of toml.split(/\r?\n/)) {
    const line = rawLine.replace(/\s+#.*$/, "").replace(/^\s*#.*$/, "").trim();
    if (!line) continue;
    if (line === "[workflow]") {
      if (pending) {
        const s = commit(pending, steps.length);
        if (s) steps.push(s);
        pending = null;
      }
      section = "workflow";
      continue;
    }
    if (line === "[[step]]") {
      if (pending) {
        const s = commit(pending, steps.length);
        if (s) steps.push(s);
      }
      pending = {};
      section = "step";
      continue;
    }
    if (line.startsWith("[")) {
      if (pending) {
        const s = commit(pending, steps.length);
        if (s) steps.push(s);
        pending = null;
      }
      section = "none";
      continue;
    }
    const eq = line.indexOf("=");
    if (eq < 0) continue;
    const key = line.slice(0, eq).trim();
    const value = stripQuoted(line.slice(eq + 1).trim());
    if (section === "workflow") {
      if (key === "name") workflowName = value;
      else if (key === "description") workflowDescription = value;
    } else if (section === "step" && pending) {
      if (key === "id") pending.id = value;
      else if (key === "name") pending.name = value;
      else if (key === "kind") pending.kind = value;
      else if (key === "description") pending.description = value;
    }
  }
  if (pending) {
    const s = commit(pending, steps.length);
    if (s) steps.push(s);
  }
  return { workflowName, workflowDescription, steps };
}
