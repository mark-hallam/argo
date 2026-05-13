/**
 * Duties API client — the AddResponsibilityDialog (E4-h) flow.
 *
 * `classifyDuty`, `previewDuty`, `commitDuty` back the 6-step
 * plain-English duty-add UX. Endpoints land with sibling brief
 * `mission-gateway-duty-endpoints`; v1 falls back to a deterministic
 * mock derived from the input string so the dialog is testable
 * end-to-end with the gateway offline. Mirrors the 404-fallback
 * contract of `retrain.ts`.
 */

import { apiPost } from "@/lib/api/client";
import { ApiError } from "@/lib/api/types";
import type { WorkerDuty } from "@/lib/api/worker-overview";

export type DutySource = "gateway" | "mock";
export type DutyKind = "recurring" | "event-triggered" | "one-shot";

export interface DutyConflict {
  existingDutyId: string;
  existingDutyLabel: string;
  existingSchedule: { kind: "cron" | "event"; spec: string };
  reason: string;
}

export interface DutySample {
  caseId: string;
  context: string;
  output: string;
}

export interface DutyProposal {
  kind: DutyKind;
  label: string;
  description: string;
  schedule: { kind: "cron" | "event"; spec: string };
  conflicts?: DutyConflict[];
  samples?: DutySample[];
}

export type ConflictResolution = "replace" | "keep-both" | "cancel";

export interface ClassifyDutyRequest {
  workerId: string;
  input: string;
  schedule: { kind: "cron" | "event"; spec: string };
}
export interface ClassifyDutyResult { proposal: DutyProposal; source: DutySource }

export interface PreviewDutyRequest { workerId: string; proposal: DutyProposal }
export interface PreviewDutyResult { samples: DutySample[]; source: DutySource }

export interface CommitDutyRequest {
  workerId: string;
  proposal: DutyProposal;
  conflictResolution?: ConflictResolution;
  replacedDutyId?: string;
}

export interface CommitDutyResult {
  duty: WorkerDuty;
  nextFireAt?: string;
  source: DutySource;
}

const KINDS = new Set<DutyKind>(["recurring", "event-triggered", "one-shot"]);

function isProposal(value: unknown): value is DutyProposal {
  if (!value || typeof value !== "object") return false;
  const v = value as Record<string, unknown>;
  if (typeof v.kind !== "string" || !KINDS.has(v.kind as DutyKind)) return false;
  if (typeof v.label !== "string" || typeof v.description !== "string") return false;
  const sched = v.schedule as { kind?: unknown; spec?: unknown } | undefined;
  if (!sched || typeof sched.spec !== "string") return false;
  return sched.kind === "cron" || sched.kind === "event";
}

function isDuty(value: unknown): value is WorkerDuty {
  if (!value || typeof value !== "object") return false;
  const v = value as Record<string, unknown>;
  return (
    typeof v.id === "string" &&
    typeof v.label === "string" &&
    typeof v.schedule === "string" &&
    typeof v.enabled === "boolean"
  );
}

function workerPath(workerId: string, suffix: string): string {
  return `/api/v1/workers/${encodeURIComponent(workerId)}${suffix}`;
}

function is404(err: unknown): boolean {
  return err instanceof ApiError && err.status === 404;
}

function hashString(s: string): number {
  let h = 0;
  for (let i = 0; i < s.length; i++) h = (h * 31 + s.charCodeAt(i)) >>> 0;
  return h;
}

function classifyKind(
  input: string,
  schedule: { kind: "cron" | "event"; spec: string },
): DutyKind {
  if (schedule.kind === "event") return "event-triggered";
  const spec = schedule.spec.trim();
  // cron with day-of-month + month set looks like a one-shot date
  const parts = spec.split(/\s+/);
  if (parts.length === 5) {
    const [, , dom, mon] = parts;
    if (dom !== "*" && mon !== "*") return "one-shot";
  }
  if (/\bonce\b|\bone[- ]?shot\b|\bone time\b/i.test(input)) return "one-shot";
  if (/\bwhen\b|\bif\b|\bafter\b|\bbefore\b/i.test(input)) return "event-triggered";
  return "recurring";
}

function labelFor(input: string): string {
  const trimmed = input.trim().replace(/\s+/g, " ");
  if (trimmed.length <= 60) return trimmed;
  return `${trimmed.slice(0, 57)}…`;
}

function descriptionFor(input: string, kind: DutyKind): string {
  const t = input.trim();
  switch (kind) {
    case "event-triggered":
      return `Watches for this trigger: ${t} When it fires, she'll handle it autonomously.`;
    case "one-shot":
      return `One-off task: ${t} She'll run this once at the scheduled time and stop.`;
    case "recurring":
    default:
      return `Recurring responsibility: ${t} She'll handle this on the cadence you picked.`;
  }
}

function samplesFor(input: string, kind: DutyKind): DutySample[] {
  const stem = labelFor(input);
  switch (kind) {
    case "event-triggered":
      return [
        { caseId: "ds_evt_1", context: "Trigger fires (test #1)", output: `Handles "${stem}" autonomously.` },
        { caseId: "ds_evt_2", context: "Trigger fires (test #2)", output: `Re-applies the rule with no human in the loop.` },
        { caseId: "ds_evt_3", context: "Trigger fires (edge case)", output: `Escalates to you if the case looks unusual.` },
      ];
    case "one-shot":
      return [
        { caseId: "ds_one_1", context: "Scheduled run", output: `Runs "${stem}" once at the scheduled time.` },
        { caseId: "ds_one_2", context: "Pre-run check", output: `Notifies you the morning of with a one-liner.` },
        { caseId: "ds_one_3", context: "Post-run", output: `Drops a confirmation in your inbox when done.` },
      ];
    case "recurring":
    default:
      return [
        { caseId: "ds_rec_1", context: "Tomorrow's run", output: `Executes "${stem}" on the scheduled cadence.` },
        { caseId: "ds_rec_2", context: "End-of-week", output: `Summarises this week's runs in a short note.` },
        { caseId: "ds_rec_3", context: "Anomaly", output: `Pages you if the run finds something unusual.` },
      ];
  }
}

export function mockProposal(req: ClassifyDutyRequest): DutyProposal {
  const kind = classifyKind(req.input, req.schedule);
  return {
    kind,
    label: labelFor(req.input),
    description: descriptionFor(req.input, kind),
    schedule: req.schedule,
    samples: samplesFor(req.input, kind),
  };
}

export async function classifyDuty(
  req: ClassifyDutyRequest,
): Promise<ClassifyDutyResult> {
  try {
    const raw = await apiPost<unknown>(workerPath(req.workerId, "/duties/classify"), {
      input: req.input,
      schedule: req.schedule,
    });
    if (!isProposal(raw)) return { proposal: mockProposal(req), source: "mock" };
    return { proposal: raw, source: "gateway" };
  } catch (err) {
    if (is404(err)) return { proposal: mockProposal(req), source: "mock" };
    throw err;
  }
}

export async function previewDuty(
  req: PreviewDutyRequest,
): Promise<PreviewDutyResult> {
  try {
    const raw = await apiPost<unknown>(workerPath(req.workerId, "/duties/preview"), {
      proposal: req.proposal,
    });
    if (Array.isArray(raw)) {
      return { samples: raw as DutySample[], source: "gateway" };
    }
  } catch (err) {
    if (!is404(err)) throw err;
  }
  return {
    samples: req.proposal.samples ?? samplesFor(req.proposal.label, req.proposal.kind),
    source: "mock",
  };
}

export function mockCommittedDuty(req: CommitDutyRequest): CommitDutyResult {
  const h = hashString(`${req.workerId}|${req.proposal.label}`);
  const id = `dty_${h.toString(16).slice(0, 8)}`;
  const scheduleText =
    req.proposal.schedule.kind === "cron"
      ? `cron: ${req.proposal.schedule.spec}`
      : `on: ${req.proposal.schedule.spec}`;
  return {
    duty: {
      id,
      label: req.proposal.label,
      schedule: scheduleText,
      enabled: true,
    },
    nextFireAt: new Date(Date.now() + 24 * 60 * 60_000).toISOString(),
    source: "mock",
  };
}

export async function commitDuty(
  req: CommitDutyRequest,
): Promise<CommitDutyResult> {
  try {
    const raw = await apiPost<unknown>(workerPath(req.workerId, "/duties/commit"), {
      proposal: req.proposal,
      conflictResolution: req.conflictResolution,
      replacedDutyId: req.replacedDutyId,
    });
    if (raw && typeof raw === "object") {
      const env = raw as { duty?: unknown; nextFireAt?: unknown };
      if (isDuty(env.duty)) {
        const nextFireAt = typeof env.nextFireAt === "string" ? env.nextFireAt : undefined;
        return { duty: env.duty, nextFireAt, source: "gateway" };
      }
    }
    return mockCommittedDuty(req);
  } catch (err) {
    if (is404(err)) return mockCommittedDuty(req);
    throw err;
  }
}
