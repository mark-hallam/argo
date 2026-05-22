/**
 * Workflow runs API client.
 *
 * Wires the `/workflows/<v>/<n>` detail page's "Run history" stat
 * grid + recent-runs table to:
 *
 *   GET /api/v1/verticals/<v>/workflows/<n>/runs[?days=30&limit=100]
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

export interface WorkflowRun {
  id: string;
  started_at: string;
  finished_at: string | null;
  ok: boolean;
  duration_ms: number | null;
}

export interface WorkflowRunsSummary {
  vertical_id: string;
  workflow_name: string;
  window_days: number;
  run_count: number;
  /** `null` when `run_count === 0` (zero of zero is undefined). */
  success_rate: number | null;
  avg_duration_ms: number;
  p95_duration_ms: number;
  last_run_at: string | null;
  runs: WorkflowRun[];
}

export interface FetchWorkflowRunsResult {
  summary: WorkflowRunsSummary;
  source: WorkflowRunsDataSource;
}

export interface FetchWorkflowRunsArgs {
  verticalId: string;
  name: string;
  days?: number;
  limit?: number;
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

export async function fetchWorkflowRuns(
  args: FetchWorkflowRunsArgs,
): Promise<FetchWorkflowRunsResult> {
  const { verticalId, name, days, limit } = args;
  if (isAuthBypassed()) {
    return {
      summary: castSample(sampleWorkflowRunsSummary(verticalId, name, days ?? 30)),
      source: "mock-bypass",
    };
  }
  const params = new URLSearchParams();
  if (days !== undefined) params.set("days", String(days));
  if (limit !== undefined) params.set("limit", String(limit));
  const qs = params.toString();
  const path = `/api/v1/verticals/${encodeURIComponent(verticalId)}/workflows/${encodeURIComponent(name)}/runs${qs ? `?${qs}` : ""}`;
  try {
    const summary = await apiGet<WorkflowRunsSummary>(path);
    return { summary, source: "gateway" };
  } catch (err) {
    if (!isStrictProduction() && isMissingOrUnreachable(err)) {
      return {
        summary: castSample(sampleWorkflowRunsSummary(verticalId, name, days ?? 30)),
        source: "mock-offline",
      };
    }
    throw err;
  }
}

function castSample(sample: SampleWorkflowRunsSummary): WorkflowRunsSummary {
  return sample;
}
