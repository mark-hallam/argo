/**
 * Audit-timeline type lock for E5 (CONTRACT). Locked here in `types/`
 * — not colocated with the audit API client — because E5-b..h and the
 * eventual sibling backend brief `mission-gateway-audit-feed-endpoint`
 * all re-import these types. Re-shaping any of them is a CONTRACT
 * break that requires re-cutting E5.
 *
 * v1 vs target (see streams/E5-a-audit-timeline-feed.md):
 * - Source: v1 reads from `GET /api/v1/audit?<filters>` with a
 *   dev-only mock fallback. Target: server endpoint mandatory in
 *   prod, RBAC-aware redaction, tenant-scoped.
 * - Cursor: opaque string; null → no more pages.
 * - Status set lifted from the WorkerActivity (4-state) primitive to
 *   include `autonomous` — for rows that completed without an
 *   approver gate. The ActivityRow primitive is locked at 4 statuses,
 *   so the page adapter maps `autonomous → succeeded` at render
 *   (the row carries the raw status in `data-status` for future
 *   styling).
 */
import type { ChannelKind } from "@/lib/sample-approvals";

/** Locked union of time-range short codes the audit feed accepts.
 *  Custom-range opens a date-range picker; v1 stubs that picker
 *  behind a toast (E5-b implements). */
export type AuditRangePreset =
  | "1h"
  | "today"
  | "yesterday"
  | "7d"
  | "30d"
  | "qtd"
  | "lq";

export const AUDIT_RANGE_PRESETS: readonly AuditRangePreset[] = [
  "1h",
  "today",
  "yesterday",
  "7d",
  "30d",
  "qtd",
  "lq",
] as const;

export const AUDIT_RANGE_LABELS: Record<AuditRangePreset, string> = {
  "1h": "Last hour",
  today: "Today",
  yesterday: "Yesterday",
  "7d": "Last 7 days",
  "30d": "Last 30 days",
  qtd: "This quarter",
  lq: "Last quarter",
};

/** A custom date range as ISO-8601 instants. */
export interface AuditCustomRange {
  from: string;
  to: string;
}

export type AuditRange = AuditRangePreset | AuditCustomRange;

export const DEFAULT_AUDIT_RANGE: AuditRangePreset = "today";

export type AuditStatus =
  | "succeeded"
  | "refused"
  | "pending"
  | "failed"
  | "autonomous";

export const AUDIT_STATUSES: readonly AuditStatus[] = [
  "succeeded",
  "refused",
  "pending",
  "failed",
  "autonomous",
] as const;

export const AUDIT_STATUS_LABELS: Record<AuditStatus, string> = {
  succeeded: "Succeeded",
  refused: "Refused",
  pending: "Pending",
  failed: "Failed",
  autonomous: "Autonomous",
};

export type DataClassification =
  | "public"
  | "internal"
  | "confidential"
  | "restricted";

/** One audit row in the cross-worker timeline. Plain English `verb`
 *  may carry PII — defense-in-depth via `<RedactedText>` lands in
 *  E1-e; the ActivityRow primitive inlines a byte-identical sentinel
 *  pass for now. */
export interface AuditRow {
  id: string;
  timestamp: string;
  worker: {
    id: string;
    name: string;
    avatarUrl?: string;
  };
  actionType: string;
  verb: string;
  status: AuditStatus;
  approver?: { id: string; name: string };
  channel?: ChannelKind;
  cost?: { usd: number };
  dataClassification?: DataClassification;
}

/** Opaque cursor token — server-authoritative. Null → no more pages. */
export type AuditCursor = string | null;

/** Filter shape consumed by `fetchAuditFeed` and threaded through the
 *  URL on the `/audit` route. `range` defaults to `today` when absent. */
export interface AuditFilters {
  range?: AuditRange;
  q?: string;
  worker?: string;
  actionType?: string;
  status?: string;
  cursor?: AuditCursor;
}

export interface AuditFeedPage {
  rows: AuditRow[];
  nextCursor: AuditCursor;
  /** Total page count for the load-more label; absent if unknown. */
  totalPages?: number;
}

export type AuditFeedSource = "gateway" | "mock-offline" | "mock-bypass";

/** Type-guard: discriminates a custom range from a preset. */
export function isCustomRange(range: AuditRange | undefined): range is AuditCustomRange {
  return typeof range === "object" && range !== null && "from" in range && "to" in range;
}

/** Day-group bucket: a calendar-day header + the rows that fall under it. */
export interface AuditDayBucket {
  /** Render label — "TODAY", "YESTERDAY", "MAY 9", or "MAY 9, 2024". */
  label: string;
  /** Calendar-day key in YYYY-MM-DD for stable React keys. */
  key: string;
  rows: AuditRow[];
}
