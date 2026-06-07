/**
 * Worker-activity API client. Mirrors the `team.ts` / `notifications.ts`
 * 404→mock-fallback contract; the gateway endpoint
 * `GET /api/v1/workers/<id>/activity` arrives via
 * `mission-gateway-worker-activity-endpoint`. Types are colocated
 * here (not in `types/`) because E5-a (the audit-timeline surface)
 * re-imports them directly when it consumes ActivityRow.
 */

import { apiGet } from "@/lib/api/client";
import { hasRealSession, isDemoLive } from "@/lib/skycot/demo-mode";
import { ApiError } from "@/lib/api/types";
import { findSampleWorkerActivity } from "@/lib/sample-worker-activity";
import { isMissingOrUnreachable } from "./unreachable";

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
export type WorkerActivityHashKind = "self" | "sample";

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
  /** Optional per-row audit-chain hash. Surfaced by `ActivityRow` as
   *  a truncated Etherscan-style cell when present; absence hides the
   *  cell (the per-worker feed currently omits it — the cross-worker
   *  audit feed populates it via the AuditDayGroup adapter). */
  hash?: string;
  hashKind?: WorkerActivityHashKind;
  parentHash?: string;
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
  // Per-user gate (sprint, 2026-06-07, mock-leak class): a REAL
  // signed-in user (the `skycot-session=1` marker cookie is present) is
  // NEVER served the auth-bypass mock fixtures, even in a demo-flagged
  // deploy. The bypass short-circuit is for ANONYMOUS demo visitors only.
  if (hasRealSession()) return false;
  return (
    process.env.NODE_ENV !== "production" &&
    process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS === "1"
  );
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
  if (isAuthBypassed() && !isDemoLive()) {
    return mockPage(workerId, filters, cursor);
  }

  const qs = buildQueryString(filters, cursor);
  const path = `/api/v1/workers/${encodeURIComponent(workerId)}/activity?${qs}`;
  try {
    const raw = await apiGet<RawGatewayActivityPage>(path);
    return normalizeGatewayActivityPage(raw, workerId);
  } catch (err) {
    if (!hasRealSession() && !isDemoLive() && !isStrictProduction() && isMissingOrUnreachable(err)) {
      return mockPage(workerId, filters, cursor);
    }
    throw err;
  }
}

/** Raw per-worker activity row as the gateway emits it (audit shape). */
interface RawGatewayActivityRow {
  id?: string;
  action_type?: string;
  verb?: string;
  status?: string;
  cost?: number | null;
  timestamp?: string;
  worker?: { id?: string; name?: string };
}

/** Raw gateway page — `items` (audit feed) with snake_case cursor fields. */
interface RawGatewayActivityPage {
  items?: RawGatewayActivityRow[];
  rows?: RawGatewayActivityRow[];
  next_cursor?: string | null;
  nextCursor?: string | null;
  total_pages?: number;
  totalPages?: number;
}

/** Two-letter avatar fallback from a display name. */
function initialsFrom(name: string): string {
  const parts = name.trim().split(/\s+/).filter(Boolean);
  if (parts.length === 0) return "··";
  if (parts.length === 1) return parts[0].slice(0, 2).toUpperCase();
  return (parts[0][0] + parts[parts.length - 1][0]).toUpperCase();
}

/** Map a gateway audit-kind (`approval.asked`, `tool_result`, …) onto the
 *  SPA's curated action-type union. Unknown kinds fall back to the
 *  generic `message_sent` so the row still renders + filters cleanly. */
function mapActionType(raw: string | undefined): WorkerActivityActionType {
  const k = (raw ?? "").toLowerCase();
  if (k.includes("approval")) return "approval_asked";
  if (k.includes("refus")) return "refusal_issued";
  if (k.includes("escalat")) return "escalation";
  if (k.includes("skill")) return "skill_learned";
  if (k.includes("memory")) return "memory_written";
  if (k.includes("draft")) return "draft_queued";
  return "message_sent";
}

const KNOWN_STATUSES = new Set<string>(WORKER_ACTIVITY_STATUSES);

/** Normalize the gateway's audit-shaped activity response into the SPA's
 *  `WorkerActivityPage`. The gateway returns `{items:[…audit rows…]}`;
 *  the SPA renders `{rows: WorkerActivity[]}`. Without this, `page.rows`
 *  is `undefined` and the activity tab's `flatMap`/`map` dereferences
 *  `undefined` → the worker-detail error boundary swallows the whole tab. */
function normalizeGatewayActivityPage(
  raw: RawGatewayActivityPage,
  workerId: string,
): WorkerActivityPage {
  const items = raw.items ?? raw.rows ?? [];
  const rows: WorkerActivity[] = (Array.isArray(items) ? items : [])
    .filter((it): it is RawGatewayActivityRow => !!it && typeof it === "object")
    .map((it, i) => {
      const name = it.worker?.name ?? "";
      const status = (it.status ?? "").toLowerCase();
      return {
        id: it.id ?? `${it.timestamp ?? "row"}-${i}`,
        workerId: it.worker?.id ?? workerId,
        workerName: name,
        workerInitials: initialsFrom(name),
        timestamp: it.timestamp ?? "",
        verb: it.verb ?? it.action_type ?? "",
        actionType: mapActionType(it.action_type ?? it.verb),
        status: (KNOWN_STATUSES.has(status) ? status : "succeeded") as WorkerActivityStatus,
        costUsd: typeof it.cost === "number" ? it.cost : undefined,
      };
    });
  return {
    rows,
    nextCursor: raw.next_cursor ?? raw.nextCursor ?? null,
    totalPages: raw.total_pages ?? raw.totalPages,
  };
}
