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
 * - `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=1` → mock immediately.
 * - 404 / network error in dev → bundled mock.
 * - Strict production (`NODE_ENV === 'production'` AND
 *   `NEXT_PUBLIC_ENV === 'production'`) propagates all errors.
 */

import { apiGet } from "@/lib/api/client";
import { ApiError } from "@/lib/api/types";
import { findDemoWorkerOverview } from "@/lib/api/demo-roster-store";
import { SAMPLE_PERSONA_IDS } from "@/lib/sample-personas";
import { findSampleWorkerOverview } from "@/lib/sample-worker-overview";
import { isDemoLive } from "@/lib/skycot/demo-mode";

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

/** Raw KPI shape as the gateway `/overview` endpoint returns it. */
interface RawGatewayKpi {
  key?: string;
  id?: string;
  label?: string;
  value?: unknown;
  sparkline_7d?: number[];
  series?: number[];
  trend?: { direction?: string; delta_pct?: number } | KpiTrend;
  deltaPct?: number;
  direction_means_good?: boolean;
}

/**
 * Map a gateway KPI to the SPA {@link WorkerKpi} contract, deriving
 * `tone` from the trend direction + whether that direction is good.
 * Tolerates the already-SPA-shaped form (mock/legacy) too.
 */
function mapGatewayKpi(k: RawGatewayKpi): WorkerKpi {
  const trendObj = typeof k.trend === "object" && k.trend !== null ? k.trend : undefined;
  const direction: KpiTrend =
    (trendObj?.direction as KpiTrend) ??
    (typeof k.trend === "string" ? (k.trend as KpiTrend) : "flat");
  const meansGood = k.direction_means_good ?? true;
  const isGood =
    (direction === "up" && meansGood) || (direction === "down" && !meansGood);
  const tone: KpiTone = direction === "flat" ? "muted" : isGood ? "success" : "warning";
  return {
    id: k.key ?? k.id ?? k.label ?? "kpi",
    label: k.label ?? "",
    value: String(k.value ?? ""),
    series: k.sparkline_7d ?? k.series ?? [],
    deltaPct: trendObj?.delta_pct ?? k.deltaPct ?? 0,
    trend: direction,
    tone,
  };
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
  /**
   * Genesis plan id that synthesised this worker (e.g.
   * `marketing-intern-maya`). Present when the worker was hired via
   * the Genesis-driven first-hire path; absent / null on the pre-
   * Genesis or fallback path. Drives the "Plan provenance" chip.
   * Wire field uses snake_case to match the gateway response.
   */
  genesis_plan_id?: string | null;
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
    process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS === "1"
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
  // Auth-bypass mocks the gateway, but DEMO_LIVE=1 opts back into real
  // gateway calls so the recording-day SPA can prove a real round-trip.
  if (isAuthBypassed() && !isDemoLive()) {
    // Session-local demo hires take precedence — they're newer than
    // anything bundled. Fall through to SAMPLE_WORKER_OVERVIEWS so
    // the five seeded personas keep working.
    const demoOverview = findDemoWorkerOverview(workerId);
    if (demoOverview) return { overview: demoOverview, source: "mock-bypass" };
    // Explicit not-found for any id outside the canonical persona set.
    // Throwing here (rather than relying on findSampleWorkerOverview
    // returning undefined) keeps the contract obvious: an unseeded
    // UUID in demo mode 404s, it does not fabricate a worker.
    if (!SAMPLE_PERSONA_IDS.includes(workerId)) {
      throw new WorkerOverviewNotFoundError(workerId);
    }
    const overview = findSampleWorkerOverview(workerId);
    if (!overview) throw new WorkerOverviewNotFoundError(workerId);
    return { overview, source: "mock-bypass" };
  }

  try {
    const raw = await apiGet<{
      charter?: unknown;
      duties?: WorkerDuty[];
      kpis?: RawGatewayKpi[];
      recentActivity?: WorkerActivityRow[];
      recent_activity?: WorkerActivityRow[];
      genesis_plan_id?: string | null;
    }>(`/api/v1/workers/${encodeURIComponent(workerId)}/overview`);
    // Normalize the live gateway shape to the SPA contract so a sparse
    // (freshly-seeded, zero-activity) worker can never crash the render:
    //  - `charter` arrives as a versioned object { text, version, ... };
    //    the SPA renders a plain string → take `.text`.
    //  - the gateway sends `recent_activity` (snake); the SPA reads
    //    `recentActivity` (camel) → map it.
    //  - KPIs arrive as { key, sparkline_7d, trend:{direction,delta_pct},
    //    direction_means_good }; the SPA needs { id, series, deltaPct,
    //    trend, tone } → map + derive tone.
    //  - every list defaults to [] so no `.length`/`.map` hits undefined.
    const charter =
      typeof raw.charter === "string"
        ? raw.charter
        : ((raw.charter as { text?: string } | null | undefined)?.text ?? "");
    const overview: WorkerOverview = {
      charter,
      duties: raw.duties ?? [],
      kpis: (raw.kpis ?? []).map(mapGatewayKpi),
      recentActivity: raw.recentActivity ?? raw.recent_activity ?? [],
      genesis_plan_id: raw.genesis_plan_id ?? null,
    };
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
