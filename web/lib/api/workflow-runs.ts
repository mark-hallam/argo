/**
 * Workflow runs API client.
 *
 * Wires the `/workflows/<v>/<n>` detail page's "Run history" stat
 * grid + recent-runs table to:
 *
 *   GET /api/v1/verticals/<v>/workflows/<n>/runs
 *       [?days=30|&since_ts=…&until_ts=…]
 *       [&page=1&page_size=25]
 *       [&status[]=success|failed|running|queued]
 *
 * Replaces the hardcoded `82` runs / `94%` success-rate / `6.2s`
 * latency literals on that page with real numbers pulled from the
 * `workflow_run` table.
 *
 * Fallback model (mirrors `workflows.ts` for consistency):
 *
 * - 404 / `TypeError` in non-strict-prod → mock-offline sample.
 * - `ARGO_GATEWAY_AUTH_BYPASS=1` in non-prod → mock-bypass sample.
 * - Any other error in strict prod propagates so the demo recording
 *   surfaces the failure (per the wave-2 audit).
 */
import { apiGet } from "@/lib/api/client";
import { ApiError } from "@/lib/api/types";
import {
  sampleWorkflowRunsSummary,
  type SampleWorkflowRunsSummary,
} from "@/lib/sample-workflow-runs";

export type WorkflowRunsDataSource = "gateway" | "mock-offline" | "mock-bypass";

export type WorkflowRunStatus = "success" | "failed" | "running" | "queued";

export const WORKFLOW_RUN_STATUSES: readonly WorkflowRunStatus[] = [
  "success",
  "failed",
  "running",
  "queued",
] as const;

export interface WorkflowRun {
  id: string;
  started_at: string;
  finished_at: string | null;
  ok: boolean;
  duration_ms: number | null;
  /**
   * Server-side derived status. Older gateway revs that predate this
   * brief omit the field; the client falls back to deriving it from
   * `ok` + `finished_at` so the chip rendering is robust either way.
   */
  status?: WorkflowRunStatus;
}

export interface WorkflowRunsSummary {
  vertical_id: string;
  workflow_name: string;
  window_days: number;
  /** `null` when the gateway predates absolute-range support. */
  since_ts?: number | null;
  /** `null` unless an explicit upper bound was passed. */
  until_ts?: number | null;
  run_count: number;
  /** `null` when `run_count === 0` (zero of zero is undefined). */
  success_rate: number | null;
  avg_duration_ms: number;
  p95_duration_ms: number;
  last_run_at: string | null;
  /** 1-indexed page. Optional for older gateway revs. */
  page?: number;
  page_size?: number;
  /** Rows matching status+date filters (drives pagination math). */
  total_filtered?: number;
  runs: WorkflowRun[];
}

export interface FetchWorkflowRunsResult {
  summary: WorkflowRunsSummary;
  source: WorkflowRunsDataSource;
}

export interface FetchWorkflowRunsArgs {
  verticalId: string;
  name: string;
  /** Legacy days-window (default 30). Ignored when sinceTs is set. */
  days?: number;
  /** Legacy upper limit on returned rows. */
  limit?: number;
  /** 1-indexed page. */
  page?: number;
  /** 25 | 50 | 100 (server clamps to [1, 100]). */
  pageSize?: number;
  /** Multi-select. Empty = no filter. */
  statuses?: readonly WorkflowRunStatus[];
  /** Epoch micros. Overrides `days` when set. */
  sinceTs?: number;
  /** Epoch micros. Open-ended when omitted. */
  untilTs?: number;
}

function isStrictProduction(): boolean {
  return (
    process.env.NODE_ENV === "production" &&
    process.env.NEXT_PUBLIC_ENV === "production"
  );
}

function isAuthBypassed(): boolean {
  return (
    process.env.NODE_ENV !== "production" &&
    process.env.ARGO_GATEWAY_AUTH_BYPASS === "1"
  );
}

function isMissingOrUnreachable(err: unknown): boolean {
  if (err instanceof ApiError) return err.status === 404;
  return err instanceof TypeError;
}

export function buildWorkflowRunsQueryString(
  args: Omit<FetchWorkflowRunsArgs, "verticalId" | "name">,
): string {
  const { days, limit, page, pageSize, statuses, sinceTs, untilTs } = args;
  const params = new URLSearchParams();
  if (days !== undefined) params.set("days", String(days));
  if (limit !== undefined) params.set("limit", String(limit));
  if (page !== undefined) params.set("page", String(page));
  if (pageSize !== undefined) params.set("page_size", String(pageSize));
  if (sinceTs !== undefined) params.set("since_ts", String(sinceTs));
  if (untilTs !== undefined) params.set("until_ts", String(untilTs));
  if (statuses && statuses.length > 0) {
    for (const s of statuses) params.append("status[]", s);
  }
  return params.toString();
}

export async function fetchWorkflowRuns(
  args: FetchWorkflowRunsArgs,
): Promise<FetchWorkflowRunsResult> {
  const { verticalId, name, days, pageSize, statuses } = args;
  if (isAuthBypassed()) {
    return {
      summary: mockSummary(verticalId, name, days, pageSize, statuses),
      source: "mock-bypass",
    };
  }
  const qs = buildWorkflowRunsQueryString(args);
  const path = `/api/v1/verticals/${encodeURIComponent(verticalId)}/workflows/${encodeURIComponent(name)}/runs${qs ? `?${qs}` : ""}`;
  try {
    const summary = await apiGet<WorkflowRunsSummary>(path);
    return { summary, source: "gateway" };
  } catch (err) {
    if (!isStrictProduction() && isMissingOrUnreachable(err)) {
      return {
        summary: mockSummary(verticalId, name, days, pageSize, statuses),
        source: "mock-offline",
      };
    }
    throw err;
  }
}

/** Client-side status derivation for older gateway revs. */
export function deriveRunStatus(run: WorkflowRun): WorkflowRunStatus {
  if (run.status) return run.status;
  if (run.finished_at == null) return "running";
  return run.ok ? "success" : "failed";
}

function mockSummary(
  verticalId: string,
  name: string,
  days: number | undefined,
  pageSize: number | undefined,
  statuses: readonly WorkflowRunStatus[] | undefined,
): WorkflowRunsSummary {
  const base = castSample(sampleWorkflowRunsSummary(verticalId, name, days ?? 30));
  // Apply status filter client-side so the mock-offline / mock-bypass
  // paths behave like the gateway (preview environment regression
  // guard).
  const allRuns = base.runs.map((r) => ({ ...r, status: deriveRunStatus(r) }));
  const filtered = statuses && statuses.length > 0
    ? allRuns.filter((r) => statuses.includes(deriveRunStatus(r)))
    : allRuns;
  const size = pageSize ?? 25;
  return {
    ...base,
    runs: filtered.slice(0, size),
    page: 1,
    page_size: size,
    total_filtered: filtered.length,
  };
}

function castSample(sample: SampleWorkflowRunsSummary): WorkflowRunsSummary {
  return sample;
}
