/**
 * Worker-activity API client. Mirrors the `team.ts` / `notifications.ts`
 * 404→mock-fallback contract; the gateway endpoint
 * `GET /api/v1/workers/<id>/activity` arrives via
 * `mission-gateway-worker-activity-endpoint`. Types are colocated
 * here (not in `types/`) because E5-a (the audit-timeline surface)
 * re-imports them directly when it consumes ActivityRow.
 */

import { apiGet } from "@/lib/api/client";
import { ApiError } from "@/lib/api/types";
import { findSampleWorkerActivity } from "@/lib/sample-worker-activity";

/** Canonical audit action types surfaced in the per-worker feed. */
export type WorkerActivityActionType =
  | "message_sent"
  | "draft_queued"
  | "approval_asked"
  | "refusal_issued"
  | "escalation"
  | "skill_learned"
  | "memory_written";

export const WORKER_ACTIVITY_ACTION_TYPES: readonly WorkerActivityActionType[] = [
  "message_sent",
  "draft_queued",
  "approval_asked",
  "refusal_issued",
  "escalation",
  "skill_learned",
  "memory_written",
] as const;

export const WORKER_ACTIVITY_ACTION_LABELS: Record<WorkerActivityActionType, string> = {
  message_sent: "Message sent",
  draft_queued: "Draft queued",
  approval_asked: "Approval asked",
  refusal_issued: "Refusal issued",
  escalation: "Escalation",
  skill_learned: "Skill learned",
  memory_written: "Memory written",
};

export type WorkerActivityStatus = "succeeded" | "failed" | "refused" | "pending";

export const WORKER_ACTIVITY_STATUSES: readonly WorkerActivityStatus[] = [
  "succeeded",
  "failed",
  "refused",
  "pending",
] as const;

export const WORKER_ACTIVITY_STATUS_LABELS: Record<WorkerActivityStatus, string> = {
  succeeded: "Succeeded",
  failed: "Failed",
  refused: "Refused",
  pending: "Pending",
};

/** Filter param: time-range short codes (URL-safe). */
export type WorkerActivityRange = "1h" | "today" | "yesterday" | "7d" | "30d";

export const WORKER_ACTIVITY_RANGES: readonly WorkerActivityRange[] = [
  "1h",
  "today",
  "yesterday",
  "7d",
  "30d",
] as const;

export const WORKER_ACTIVITY_RANGE_LABELS: Record<WorkerActivityRange, string> = {
  "1h": "Last hour",
  today: "Today",
  yesterday: "Yesterday",
  "7d": "Last 7 days",
  "30d": "Last 30 days",
};

export const DEFAULT_RANGE: WorkerActivityRange = "7d";
export const PAGE_SIZE = 50;

export interface WorkerActivityCostBreakdown {
  tokensIn: number;
  tokensOut: number;
  model: string;
}

/** One audit row in the per-worker feed. `verb` may carry PII —
 *  E1-e's `<RedactedText>` wraps it at render (defense-in-depth;
 *  the gateway redaction in `J2` is the mandatory first fence). */
export interface WorkerActivity {
  id: string;
  workerId: string;
  workerName: string;
  workerInitials: string;
  timestamp: string;
  verb: string;
  actionType: WorkerActivityActionType;
  status: WorkerActivityStatus;
  costUsd?: number;
  costBreakdown?: WorkerActivityCostBreakdown;
}

export interface WorkerActivityFilters {
  range: WorkerActivityRange;
  /** Empty / "all" omitted from the gateway query string. */
  actionType?: WorkerActivityActionType;
  status?: WorkerActivityStatus;
}

export interface WorkerActivityPage {
  rows: WorkerActivity[];
  /** Opaque cursor token; absent → no more pages. */
  nextCursor: string | null;
  /** Total page count for the load-more label; absent if unknown. */
  totalPages?: number;
}

export type WorkerActivitySource = "gateway" | "mock-offline" | "mock-bypass";

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

/** Lower bound for a time-range filter; rows older than this are dropped. */
export function rangeFloor(range: WorkerActivityRange, now: number = Date.now()): number {
  switch (range) {
    case "1h":
      return now - 60 * 60_000;
    case "today": {
      const d = new Date(now);
      d.setHours(0, 0, 0, 0);
      return d.getTime();
    }
    case "yesterday": {
      const d = new Date(now);
      d.setHours(0, 0, 0, 0);
      return d.getTime() - 24 * 60 * 60_000;
    }
    case "7d":
      return now - 7 * 24 * 60 * 60_000;
    case "30d":
      return now - 30 * 24 * 60 * 60_000;
  }
}

/** Upper bound for the "yesterday" range; other ranges have no upper cut-off. */
export function rangeCeiling(range: WorkerActivityRange, now: number = Date.now()): number {
  if (range !== "yesterday") return now;
  const d = new Date(now);
  d.setHours(0, 0, 0, 0);
  return d.getTime();
}

function buildQueryString(
  filters: WorkerActivityFilters,
  cursor: string | null,
): string {
  const params = new URLSearchParams();
  params.set("range", filters.range);
  if (filters.actionType) params.set("type", filters.actionType);
  if (filters.status) params.set("status", filters.status);
  if (cursor) params.set("cursor", cursor);
  return params.toString();
}

function applyFilters(
  rows: WorkerActivity[],
  filters: WorkerActivityFilters,
): WorkerActivity[] {
  const now = Date.now();
  const floor = rangeFloor(filters.range, now);
  const ceiling = rangeCeiling(filters.range, now);
  return rows.filter((row) => {
    const t = Date.parse(row.timestamp);
    if (Number.isNaN(t) || t < floor || t > ceiling) return false;
    if (filters.actionType && row.actionType !== filters.actionType) return false;
    if (filters.status && row.status !== filters.status) return false;
    return true;
  });
}

function paginate(
  rows: WorkerActivity[],
  cursor: string | null,
): WorkerActivityPage {
  const start = cursor ? Number.parseInt(cursor, 10) || 0 : 0;
  const end = start + PAGE_SIZE;
  const slice = rows.slice(start, end);
  const hasMore = end < rows.length;
  const totalPages = Math.max(1, Math.ceil(rows.length / PAGE_SIZE));
  return {
    rows: slice,
    nextCursor: hasMore ? String(end) : null,
    totalPages,
  };
}

function mockPage(
  workerId: string,
  filters: WorkerActivityFilters,
  cursor: string | null,
): WorkerActivityPage {
  const all = findSampleWorkerActivity(workerId);
  const filtered = applyFilters(all, filters);
  return paginate(filtered, cursor);
}

/**
 * Fetch one page of audit rows for `workerId`, filtered server-side
 * (or client-side in mock mode). Cursor `null` → first page.
 */
export async function fetchWorkerActivity(
  workerId: string,
  filters: WorkerActivityFilters,
  cursor: string | null = null,
): Promise<WorkerActivityPage> {
  if (isAuthBypassed()) {
    return mockPage(workerId, filters, cursor);
  }

  const qs = buildQueryString(filters, cursor);
  const path = `/api/v1/workers/${encodeURIComponent(workerId)}/activity?${qs}`;
  try {
    return await apiGet<WorkerActivityPage>(path);
  } catch (err) {
    if (!isStrictProduction() && isMissingOrUnreachable(err)) {
      return mockPage(workerId, filters, cursor);
    }
    throw err;
  }
}
