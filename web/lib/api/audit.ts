/**
 * Audit-timeline API client. Mirrors the `worker-activity.ts` /
 * `team.ts` 404→mock-fallback contract; the gateway endpoint
 * `GET /api/v1/audit` arrives via `mission-gateway-audit-feed-endpoint`.
 *
 * In strict-production mode (NODE_ENV=production AND
 * NEXT_PUBLIC_ENV=production) the mock fallback is suppressed — a
 * 404 or network error surfaces to the caller so SREs see real
 * gateway breakage rather than mocks that pretend to work.
 */
import { apiGet } from "@/lib/api/client";
import { ApiError } from "@/lib/api/types";
import { SAMPLE_AUDIT_ROWS } from "@/lib/sample-audit";
import {
  AUDIT_RANGE_PRESETS,
  DEFAULT_AUDIT_RANGE,
  isCustomRange,
  type AuditCursor,
  type AuditDayBucket,
  type AuditFilters,
  type AuditFeedPage,
  type AuditRange,
  type AuditRangePreset,
  type AuditRow,
} from "@/lib/types/audit";

export const PAGE_SIZE = 50;

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

function effectiveRange(range: AuditRange | undefined): AuditRange {
  if (!range) return DEFAULT_AUDIT_RANGE;
  if (isCustomRange(range)) return range;
  return (AUDIT_RANGE_PRESETS as readonly string[]).includes(range)
    ? range
    : DEFAULT_AUDIT_RANGE;
}

/** Lower bound (inclusive) for a range — rows older than this are dropped. */
export function rangeFloor(range: AuditRange, now: number = Date.now()): number {
  if (isCustomRange(range)) return Date.parse(range.from);
  return presetFloor(range, now);
}

/** Upper bound (inclusive) for a range — rows newer than this are dropped. */
export function rangeCeiling(range: AuditRange, now: number = Date.now()): number {
  if (isCustomRange(range)) return Date.parse(range.to);
  if (range === "yesterday") {
    const d = new Date(now);
    d.setHours(0, 0, 0, 0);
    return d.getTime();
  }
  if (range === "lq") {
    const d = new Date(now);
    const q = Math.floor(d.getMonth() / 3);
    d.setMonth(q * 3, 1);
    d.setHours(0, 0, 0, 0);
    return d.getTime();
  }
  return now;
}

function presetFloor(range: AuditRangePreset, now: number): number {
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
    case "qtd": {
      const d = new Date(now);
      const q = Math.floor(d.getMonth() / 3);
      d.setMonth(q * 3, 1);
      d.setHours(0, 0, 0, 0);
      return d.getTime();
    }
    case "lq": {
      const d = new Date(now);
      const q = Math.floor(d.getMonth() / 3);
      d.setMonth(q * 3 - 3, 1);
      d.setHours(0, 0, 0, 0);
      return d.getTime();
    }
  }
}

function matchesQuery(row: AuditRow, q: string): boolean {
  if (!q) return true;
  const needle = q.toLowerCase();
  return (
    row.verb.toLowerCase().includes(needle) ||
    row.worker.name.toLowerCase().includes(needle) ||
    row.id.toLowerCase().includes(needle) ||
    row.actionType.toLowerCase().includes(needle)
  );
}

function applyFilters(rows: AuditRow[], filters: AuditFilters): AuditRow[] {
  const now = Date.now();
  const range = effectiveRange(filters.range);
  const floor = rangeFloor(range, now);
  const ceiling = rangeCeiling(range, now);
  const q = filters.q?.trim() ?? "";
  return rows.filter((row) => {
    const t = Date.parse(row.timestamp);
    if (Number.isNaN(t) || t < floor || t > ceiling) return false;
    if (filters.worker && row.worker.id !== filters.worker) return false;
    if (filters.actionType && row.actionType !== filters.actionType) return false;
    if (filters.status && row.status !== filters.status) return false;
    if (!matchesQuery(row, q)) return false;
    return true;
  });
}

function paginate(rows: AuditRow[], cursor: AuditCursor): AuditFeedPage {
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

function mockPage(filters: AuditFilters): AuditFeedPage {
  const filtered = applyFilters(SAMPLE_AUDIT_ROWS, filters);
  return paginate(filtered, filters.cursor ?? null);
}

function buildQueryString(filters: AuditFilters): string {
  const params = new URLSearchParams();
  const range = effectiveRange(filters.range);
  if (isCustomRange(range)) {
    params.set("from", range.from);
    params.set("to", range.to);
  } else {
    params.set("range", range);
  }
  if (filters.q) params.set("q", filters.q);
  if (filters.worker) params.set("worker", filters.worker);
  if (filters.actionType) params.set("action_type", filters.actionType);
  if (filters.status) params.set("status", filters.status);
  if (filters.cursor) params.set("cursor", filters.cursor);
  return params.toString();
}

/** Group audit rows into day buckets in chronological-descending
 *  order. "TODAY" / "YESTERDAY" labels resolve against `now`; older
 *  days render as `MAY 9` (locale-formatted short date, year omitted
 *  if it matches the current year). */
export function groupByDay(rows: AuditRow[], now: number = Date.now()): AuditDayBucket[] {
  const today = startOfDay(now);
  const yesterday = today - 24 * 60 * 60_000;
  const thisYear = new Date(now).getFullYear();
  const buckets = new Map<string, AuditDayBucket>();
  for (const row of rows) {
    const t = Date.parse(row.timestamp);
    if (Number.isNaN(t)) continue;
    const dayStart = startOfDay(t);
    const key = dayKey(dayStart);
    const existing = buckets.get(key);
    if (existing) {
      existing.rows.push(row);
      continue;
    }
    buckets.set(key, {
      key,
      label: labelFor(dayStart, today, yesterday, thisYear),
      rows: [row],
    });
  }
  return Array.from(buckets.values()).sort((a, b) => b.key.localeCompare(a.key));
}

function startOfDay(ms: number): number {
  const d = new Date(ms);
  d.setHours(0, 0, 0, 0);
  return d.getTime();
}

function dayKey(dayStart: number): string {
  const d = new Date(dayStart);
  const y = d.getFullYear();
  const m = String(d.getMonth() + 1).padStart(2, "0");
  const dd = String(d.getDate()).padStart(2, "0");
  return `${y}-${m}-${dd}`;
}

function labelFor(dayStart: number, today: number, yesterday: number, thisYear: number): string {
  if (dayStart === today) return "TODAY";
  if (dayStart === yesterday) return "YESTERDAY";
  const d = new Date(dayStart);
  const month = d.toLocaleString("en-US", { month: "short" }).toUpperCase();
  const day = d.getDate();
  return d.getFullYear() === thisYear ? `${month} ${day}` : `${month} ${day}, ${d.getFullYear()}`;
}

/**
 * Fetch one page of audit rows. Cursor `null` (or absent) → first
 * page. The server is authoritative about "no more pages" via a
 * `nextCursor: null` response.
 */
export async function fetchAuditFeed(filters: AuditFilters): Promise<AuditFeedPage> {
  if (isAuthBypassed()) {
    return mockPage(filters);
  }

  const qs = buildQueryString(filters);
  const path = `/api/v1/audit?${qs}`;
  try {
    return await apiGet<AuditFeedPage>(path);
  } catch (err) {
    if (!isStrictProduction() && isMissingOrUnreachable(err)) {
      return mockPage(filters);
    }
    throw err;
  }
}
