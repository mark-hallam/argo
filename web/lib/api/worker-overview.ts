/**
 * Worker overview API client.
 *
 * Routes through `lib/api/client.ts`. The gateway endpoint
 * `GET /api/v1/workers/<id>/overview` is being authored under
 * `mission-gateway-worker-overview-endpoint`; until that lands, every
 * call here falls back to the bundled mock dataset in dev so the
 * Overview tab renders without a running gateway.
 *
 * Mirrors the fallback contract of `lib/api/team.ts`:
 * - `NEXT_PUBLIC_GATEWAY_AUTH_BYPASS=1` → mock immediately.
 * - 404 / network error in dev → bundled mock.
 * - Strict production (`NODE_ENV === 'production'` AND
 *   `NEXT_PUBLIC_ENV === 'production'`) propagates all errors.
 */

import { apiGet } from "@/lib/api/client";
import { ApiError } from "@/lib/api/types";
import { findSampleWorkerOverview } from "@/lib/sample-worker-overview";

export type WorkerOverviewSource = "gateway" | "mock-offline" | "mock-bypass";

export type KpiTrend = "up" | "down" | "flat";
export type KpiTone = "success" | "warning" | "muted";

export interface WorkerDuty {
  id: string;
  /** Short imperative phrase ("Screen inbound applicants"). */
  label: string;
  /** Human cadence, e.g. "weekdays 09:00" or "as messages arrive". */
  schedule: string;
  enabled: boolean;
}

export interface WorkerKpi {
  id: string;
  label: string;
  /** Display-ready value string (e.g. "12", "$340", "4m 21s"). */
  value: string;
  /** 7-day series, oldest → newest. */
  series: number[];
  /** Percentage change vs prior period; negative = down. */
  deltaPct: number;
  trend: KpiTrend;
  tone: KpiTone;
}

export interface WorkerActivityRow {
  id: string;
  /** ISO timestamp. */
  at: string;
  /** Verb-phrase summary, e.g. "sent reply to buyer #4821". */
  verb: string;
  /** Outcome chip, e.g. "approved by Sam", "auto-sent", "needs review". */
  status: string;
  /** Audit drill-in target — relative URL. */
  href: string;
}

export interface WorkerOverview {
  charter: string;
  duties: WorkerDuty[];
  /** v1 contract: exactly 4 KPIs; vertical-aware selection lives upstream. */
  kpis: WorkerKpi[];
  /** v1 contract: last 5 actions. */
  recentActivity: WorkerActivityRow[];
}

export interface FetchWorkerOverviewResult {
  overview: WorkerOverview;
  source: WorkerOverviewSource;
}

/** Thrown when the worker id doesn't resolve to an overview. */
export class WorkerOverviewNotFoundError extends Error {
  readonly workerId: string;
  constructor(workerId: string) {
    super(`Worker overview for '${workerId}' not found`);
    this.name = "WorkerOverviewNotFoundError";
    this.workerId = workerId;
  }
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
    process.env.NEXT_PUBLIC_GATEWAY_AUTH_BYPASS === "1"
  );
}

function isMissingOrUnreachable(err: unknown): boolean {
  if (err instanceof ApiError) return err.status === 404;
  return err instanceof TypeError;
}

function mockResult(workerId: string): FetchWorkerOverviewResult {
  const overview = findSampleWorkerOverview(workerId);
  if (!overview) throw new WorkerOverviewNotFoundError(workerId);
  return { overview, source: "mock-offline" };
}

export async function fetchWorkerOverview(
  workerId: string,
): Promise<FetchWorkerOverviewResult> {
  if (isAuthBypassed()) {
    const overview = findSampleWorkerOverview(workerId);
    if (!overview) throw new WorkerOverviewNotFoundError(workerId);
    return { overview, source: "mock-bypass" };
  }

  try {
    const overview = await apiGet<WorkerOverview>(
      `/api/v1/workers/${encodeURIComponent(workerId)}/overview`,
    );
    return { overview, source: "gateway" };
  } catch (err) {
    if (!isStrictProduction() && isMissingOrUnreachable(err)) {
      return mockResult(workerId);
    }
    if (err instanceof ApiError && err.status === 404) {
      throw new WorkerOverviewNotFoundError(workerId);
    }
    throw err;
  }
}
