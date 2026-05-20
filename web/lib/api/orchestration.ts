import { apiGet, apiPost } from "@/lib/api/client";
import { ApiError } from "@/lib/api/types";

export type OrchestrationStatus = "queued" | "running" | "waiting" | "blocked" | "done";
export type SlaState = "healthy" | "warning" | "breached";

export interface OrchestrationRun {
  id: string;
  worker: string;
  workerId: string;
  skill: string;
  status: OrchestrationStatus;
  slaState: SlaState;
  latencyMs: number;
  targetLatencyMs: number;
  retryAttempt: number;
  retryMax: number;
  retryBackoff: "none" | "exponential";
  queuedAt: string;
}

export interface OrchestrationSummary {
  total: number;
  running: number;
  queued: number;
  breached: number;
}

export interface OrchestrationResponse {
  runs: OrchestrationRun[];
  summary: OrchestrationSummary;
}

export interface OrchestrationFilters {
  worker: string;
  skill: string;
  status: "all" | OrchestrationStatus;
  slaState: "all" | SlaState;
}

export type OrchestrationSource = "gateway" | "mock-offline" | "mock-bypass";

export interface FetchOrchestrationResult extends OrchestrationResponse {
  source: OrchestrationSource;
}

const WORKERS = ["Maya", "Nora", "Jules", "Anika", "Theo", "Priya"];
const SKILLS = [
  "invoice.extract",
  "vendor.followup",
  "approval.route",
  "candidate.screen",
  "deal.validate",
];
const STATUSES: OrchestrationStatus[] = ["running", "queued", "waiting", "blocked", "done"];

export const EMPTY_ORCHESTRATION_FILTERS: OrchestrationFilters = {
  worker: "",
  skill: "",
  status: "all",
  slaState: "all",
};

export const SAMPLE_ORCHESTRATION_RUNS: OrchestrationRun[] = Array.from(
  { length: 64 },
  (_, idx) => {
    const target = 90000 + (idx % 4) * 30000;
    const latency = target * (idx % 7 === 0 ? 1.35 : idx % 5 === 0 ? 0.92 : 0.44);
    const slaState: SlaState =
      latency > target ? "breached" : latency > target * 0.8 ? "warning" : "healthy";
    return {
      id: `run-${String(idx + 1).padStart(3, "0")}`,
      worker: WORKERS[idx % WORKERS.length],
      workerId: `worker-${idx % WORKERS.length}`,
      skill: SKILLS[idx % SKILLS.length],
      status: STATUSES[idx % STATUSES.length],
      slaState,
      latencyMs: Math.round(latency),
      targetLatencyMs: target,
      retryAttempt: idx % 4,
      retryMax: 3 + (idx % 2),
      retryBackoff: idx % 6 === 0 ? "none" : "exponential",
      queuedAt: new Date(Date.UTC(2026, 4, 14, 9, idx % 60)).toISOString(),
    };
  },
);

function isStrictProduction(): boolean {
  return process.env.NODE_ENV === "production" && process.env.NEXT_PUBLIC_ENV === "production";
}

function isAuthBypassed(): boolean {
  return process.env.NODE_ENV !== "production" && process.env.ARGO_GATEWAY_AUTH_BYPASS === "1";
}

function isMissingOrUnreachable(err: unknown): boolean {
  if (err instanceof ApiError) return err.status === 404;
  return err instanceof TypeError;
}

export function summarizeRuns(runs: OrchestrationRun[]): OrchestrationSummary {
  return {
    total: runs.length,
    running: runs.filter((run) => run.status === "running").length,
    queued: runs.filter((run) => run.status === "queued").length,
    breached: runs.filter((run) => run.slaState === "breached").length,
  };
}

export function applyOrchestrationFilters(
  runs: OrchestrationRun[],
  filters: OrchestrationFilters,
): OrchestrationRun[] {
  return runs.filter((run) => {
    if (filters.worker && run.worker !== filters.worker) return false;
    if (filters.skill && run.skill !== filters.skill) return false;
    if (filters.status !== "all" && run.status !== filters.status) return false;
    if (filters.slaState !== "all" && run.slaState !== filters.slaState) return false;
    return true;
  });
}

export async function fetchOrchestrationRuns(): Promise<FetchOrchestrationResult> {
  if (isAuthBypassed()) {
    return {
      runs: SAMPLE_ORCHESTRATION_RUNS,
      summary: summarizeRuns(SAMPLE_ORCHESTRATION_RUNS),
      source: "mock-bypass",
    };
  }
  try {
    const body = await apiGet<OrchestrationResponse>("/api/v1/orchestration/runs");
    return { ...body, source: "gateway" };
  } catch (err) {
    if (!isStrictProduction() && isMissingOrUnreachable(err)) {
      return {
        runs: SAMPLE_ORCHESTRATION_RUNS,
        summary: summarizeRuns(SAMPLE_ORCHESTRATION_RUNS),
        source: "mock-offline",
      };
    }
    throw err;
  }
}

export async function cancelRun(runId: string): Promise<void> {
  await apiPost(`/api/v1/orchestration/runs/${encodeURIComponent(runId)}/cancel`, {});
}

export async function resetRetry(runId: string): Promise<void> {
  await apiPost(`/api/v1/orchestration/runs/${encodeURIComponent(runId)}/reset-retry`, {});
}

export async function forceEscalate(runId: string): Promise<void> {
  await apiPost(`/api/v1/orchestration/runs/${encodeURIComponent(runId)}/force-escalate`, {});
}
