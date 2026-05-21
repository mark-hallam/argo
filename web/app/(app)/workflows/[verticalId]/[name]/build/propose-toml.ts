import type { StepKind, WorkflowStep } from "./types";

// Minimal TOML reader for the streamed `/api/workflows/propose` output.
// Only the subset the propose route emits today is parsed:
//   • [workflow] name / description
//   • [[step]] name / kind / description / prompt / depends_on
// Anything unknown is ignored so partial streams stay safe to render.
// Returning `null` for parse() means "not enough complete data yet".

const TOML_KINDS: Record<string, StepKind> = {
  trigger: "trigger",
  skill_invoke: "skill_invoke",
  skill: "skill_invoke",
  prompt: "skill_invoke",
  approval: "approval",
  tool_call: "tool_call",
  tool: "tool_call",
  notify: "notify",
  notification: "notify",
  decision: "decision",
  branch: "decision",
  data: "data",
  loop: "loop",
  wait: "wait",
};

function stripString(value: string): string | null {
  const trimmed = value.trim();
  const match = /^"((?:\\.|[^"\\])*)"$/.exec(trimmed) ?? /^'([^']*)'$/.exec(trimmed);
  if (!match) return null;
  return match[1].replace(/\\"/g, '"').replace(/\\\\/g, "\\");
}

function parseList(value: string): string[] | null {
  const trimmed = value.trim();
  if (!trimmed.startsWith("[")) return null;
  // Only return when the closing bracket arrived; mid-stream skips.
  const close = trimmed.lastIndexOf("]");
  if (close < 0) return null;
  const inner = trimmed.slice(1, close);
  if (inner.trim().length === 0) return [];
  return inner
    .split(",")
    .map((s) => stripString(s) ?? s.trim())
    .filter((s) => s.length > 0);
}

function slug(input: string): string {
  return input
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "_")
    .replace(/^_+|_+$/g, "")
    .slice(0, 48);
}

export interface ParsedProposal {
  workflowName: string | null;
  workflowDescription: string | null;
  steps: WorkflowStep[];
}

interface PendingStep {
  name?: string;
  kind?: string;
  description?: string;
  prompt?: string;
  depends_on?: string[];
}

function finalize(pending: PendingStep, idx: number): WorkflowStep | null {
  const rawName = pending.name?.trim();
  if (!rawName) return null;
  const id = slug(rawName) || `step_${idx}`;
  const kindKey = pending.kind?.trim().toLowerCase() ?? "";
  const kind: StepKind = TOML_KINDS[kindKey] ?? "skill_invoke";
  const description = pending.description?.trim()
    || pending.prompt?.trim()
    || "";
  return { id, kind, name: rawName, description };
}

/**
 * Parse a (possibly partial) TOML stream into the workflow shape used
 * by the visual builder. Tolerant of incomplete tail content; only
 * complete `[[step]]` blocks with a `name` value are emitted.
 */
export function parseProposalToml(toml: string): ParsedProposal {
  const result: ParsedProposal = {
    workflowName: null,
    workflowDescription: null,
    steps: [],
  };
  if (toml.length === 0) return result;

  let section: "none" | "workflow" | "step" = "none";
  let pendingStep: PendingStep | null = null;
  let stepIdx = 0;

  const lines = toml.split(/\r?\n/);
  for (const line of lines) {
    const stripped = line.trim();
    if (stripped.length === 0 || stripped.startsWith("#")) continue;

    if (stripped === "[workflow]") {
      if (pendingStep) {
        const done = finalize(pendingStep, stepIdx);
        if (done) {
          result.steps.push(done);
          stepIdx += 1;
        }
        pendingStep = null;
      }
      section = "workflow";
      continue;
    }
    if (stripped === "[[step]]") {
      if (pendingStep) {
        const done = finalize(pendingStep, stepIdx);
        if (done) {
          result.steps.push(done);
          stepIdx += 1;
        }
      }
      pendingStep = {};
      section = "step";
      continue;
    }
    if (/^\[.+\]$/.test(stripped) || /^\[\[.+\]\]$/.test(stripped)) {
      // Unknown section header → flush pending step, stop collecting.
      if (pendingStep) {
        const done = finalize(pendingStep, stepIdx);
        if (done) {
          result.steps.push(done);
          stepIdx += 1;
        }
        pendingStep = null;
      }
      section = "none";
      continue;
    }

    const eq = stripped.indexOf("=");
    if (eq <= 0) continue;
    const key = stripped.slice(0, eq).trim();
    const value = stripped.slice(eq + 1).trim();
    if (section === "workflow") {
      if (key === "name") {
        result.workflowName = stripString(value);
      } else if (key === "description") {
        result.workflowDescription = stripString(value);
      }
      continue;
    }
    if (section === "step" && pendingStep) {
      if (key === "name") pendingStep.name = stripString(value) ?? undefined;
      else if (key === "kind") pendingStep.kind = stripString(value) ?? undefined;
      else if (key === "description") pendingStep.description = stripString(value) ?? undefined;
      else if (key === "prompt") pendingStep.prompt = stripString(value) ?? undefined;
      else if (key === "depends_on") {
        const list = parseList(value);
        if (list) pendingStep.depends_on = list;
      }
    }
  }
  // Tail: emit the trailing step only if it has its name field. This
  // keeps the diff stable as new keys stream in below the name line.
  if (pendingStep) {
    const done = finalize(pendingStep, stepIdx);
    if (done) result.steps.push(done);
  }
  return result;
}

export interface ProposalDiff {
  added: WorkflowStep[];
  removed: WorkflowStep[];
  changed: { before: WorkflowStep; after: WorkflowStep }[];
  unchangedCount: number;
}

export function diffSteps(
  current: WorkflowStep[],
  proposed: WorkflowStep[],
): ProposalDiff {
  const byId = (s: WorkflowStep) => s.id;
  const currentMap = new Map(current.map((s) => [byId(s), s]));
  const proposedMap = new Map(proposed.map((s) => [byId(s), s]));
  const added: WorkflowStep[] = [];
  const removed: WorkflowStep[] = [];
  const changed: { before: WorkflowStep; after: WorkflowStep }[] = [];
  let unchanged = 0;
  for (const p of proposed) {
    const prev = currentMap.get(p.id);
    if (!prev) {
      added.push(p);
    } else if (prev.kind !== p.kind || prev.name !== p.name || prev.description !== p.description) {
      changed.push({ before: prev, after: p });
    } else {
      unchanged += 1;
    }
  }
  for (const c of current) {
    if (!proposedMap.has(c.id)) removed.push(c);
  }
  return { added, removed, changed, unchangedCount: unchanged };
}
