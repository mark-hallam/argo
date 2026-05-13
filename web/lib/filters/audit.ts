/**
 * Pure filter logic for the cross-worker audit timeline (E5-b).
 *
 * Seven dimensions — worker, channel, action type, status, approver,
 * cost band, data classification. Values OR within a dimension and
 * AND across dimensions. URL is the source of truth; the page threads
 * only `range` + `q` through `useAuditFeed` and applies these 7
 * dimensions client-side. Server-side push for scopes > 5000 rows is
 * tracked under `mission-enterprise-audit-server-filter`.
 */
import type {
  AuditRow,
  AuditStatus,
  DataClassification,
} from "@/lib/types/audit";
import { AUDIT_STATUSES } from "@/lib/types/audit";
import type { ChannelKind } from "@/lib/sample-approvals";
import { CHANNEL_KINDS } from "@/lib/filters/approvals";

export type CostBand = "free" | "lt_0_01" | "0_01_to_10" | "gt_0_10";

export const COST_BANDS: readonly CostBand[] = [
  "free", "lt_0_01", "0_01_to_10", "gt_0_10",
] as const;

export const COST_BAND_LABELS: Record<CostBand, string> = {
  free: "Free", lt_0_01: "< $0.01", "0_01_to_10": "$0.01 – $0.10", gt_0_10: "> $0.10",
};

export const DATA_CLASSIFICATIONS: readonly DataClassification[] = [
  "public", "internal", "confidential", "restricted",
] as const;

export const CLASSIFICATION_LABELS: Record<DataClassification, string> = {
  public: "Public", internal: "Internal", confidential: "Confidential", restricted: "Restricted",
};

export const CHANNEL_LABELS: Record<ChannelKind, string> = {
  slack: "Slack", imessage: "iMessage", sms: "SMS", push: "Push",
  web: "Web inbox", voice: "Voice", whatsapp: "WhatsApp",
};

export const STATUS_LABELS: Record<AuditStatus, string> = {
  succeeded: "Succeeded", refused: "Refused", pending: "Pending",
  failed: "Failed", autonomous: "Autonomous",
};

/** Suggested action-type checkboxes; merged with values observed in
 *  the loaded audit set so unfamiliar types still appear. */
export const SUGGESTED_ACTION_TYPES: readonly string[] = [
  "message_sent", "message_drafted", "approval_asked", "approval_decision",
  "refund_issued", "invoice_processed", "candidate_screened", "interview_booked",
  "rfp_drafted", "skill_promoted", "memory_written", "duty_fired",
] as const;

export interface AuditMultiFilters {
  worker: string[];
  channel: ChannelKind[];
  actionType: string[];
  status: AuditStatus[];
  approver: string[];
  costBand: CostBand[];
  classification: DataClassification[];
}

export const EMPTY_AUDIT_FILTERS: AuditMultiFilters = {
  worker: [], channel: [], actionType: [], status: [],
  approver: [], costBand: [], classification: [],
};

export const AUDIT_FILTER_DIMENSIONS: readonly (keyof AuditMultiFilters)[] = [
  "worker", "channel", "actionType", "status",
  "approver", "costBand", "classification",
] as const;

const URL_KEY: Record<keyof AuditMultiFilters, string> = {
  worker: "worker", channel: "channel", actionType: "action_type",
  status: "status", approver: "approver", costBand: "cost_band",
  classification: "classification",
};

function splitCsv(value: string | null | undefined): string[] {
  if (!value) return [];
  return value.split(",").map((v) => v.trim()).filter((v) => v.length > 0);
}

function uniqSorted<T extends string>(values: T[]): T[] {
  return Array.from(new Set(values)).sort();
}

/** Parse a URL search-param bag. Unknown tokens for closed
 *  enumerations are dropped silently; open-enum dimensions (worker /
 *  actionType / approver) accept any non-empty string. */
export function parseAuditMultiFilters(
  params: URLSearchParams | { get(name: string): string | null },
): AuditMultiFilters {
  return {
    worker: uniqSorted(splitCsv(params.get(URL_KEY.worker))),
    channel: uniqSorted(splitCsv(params.get(URL_KEY.channel))).filter(
      (c): c is ChannelKind => (CHANNEL_KINDS as readonly string[]).includes(c),
    ),
    actionType: uniqSorted(splitCsv(params.get(URL_KEY.actionType))),
    status: uniqSorted(splitCsv(params.get(URL_KEY.status))).filter(
      (s): s is AuditStatus => (AUDIT_STATUSES as readonly string[]).includes(s),
    ),
    approver: uniqSorted(splitCsv(params.get(URL_KEY.approver))),
    costBand: uniqSorted(splitCsv(params.get(URL_KEY.costBand))).filter(
      (b): b is CostBand => (COST_BANDS as readonly string[]).includes(b),
    ),
    classification: uniqSorted(splitCsv(params.get(URL_KEY.classification))).filter(
      (c): c is DataClassification =>
        (DATA_CLASSIFICATIONS as readonly string[]).includes(c),
    ),
  };
}

export function serializeAuditMultiFilters(filters: AuditMultiFilters): string {
  const params = new URLSearchParams();
  for (const dim of AUDIT_FILTER_DIMENSIONS) {
    const values = filters[dim];
    if (values.length > 0) {
      params.set(URL_KEY[dim], uniqSorted(values as string[]).join(","));
    }
  }
  return params.toString();
}

export function isAuditFilterEmpty(filters: AuditMultiFilters): boolean {
  return AUDIT_FILTER_DIMENSIONS.every((d) => filters[d].length === 0);
}

export function activeAuditFilterCount(filters: AuditMultiFilters): number {
  return AUDIT_FILTER_DIMENSIONS.reduce(
    (n, d) => n + (filters[d].length > 0 ? 1 : 0), 0,
  );
}

export function inCostBand(row: AuditRow, band: CostBand): boolean {
  const c = row.cost?.usd ?? 0;
  switch (band) {
    case "free":       return c === 0;
    case "lt_0_01":    return c > 0 && c < 0.01;
    case "0_01_to_10": return c >= 0.01 && c <= 0.1;
    case "gt_0_10":    return c > 0.1;
  }
}

export function applyAuditMultiFilters(
  rows: AuditRow[],
  filters: AuditMultiFilters,
): AuditRow[] {
  if (isAuditFilterEmpty(filters)) return rows;
  return rows.filter((row) => {
    if (filters.worker.length > 0 && !filters.worker.includes(row.worker.id)) return false;
    if (filters.channel.length > 0) {
      if (!row.channel || !filters.channel.includes(row.channel)) return false;
    }
    if (filters.actionType.length > 0 && !filters.actionType.includes(row.actionType)) return false;
    if (filters.status.length > 0 && !filters.status.includes(row.status)) return false;
    if (filters.approver.length > 0) {
      if (!row.approver || !filters.approver.includes(row.approver.id)) return false;
    }
    if (filters.costBand.length > 0 && !filters.costBand.some((b) => inCostBand(row, b))) return false;
    if (filters.classification.length > 0) {
      if (!row.dataClassification) return false;
      if (!filters.classification.includes(row.dataClassification)) return false;
    }
    return true;
  });
}

export function toggleAuditFilterValue<K extends keyof AuditMultiFilters>(
  filters: AuditMultiFilters,
  dimension: K,
  value: AuditMultiFilters[K][number],
): AuditMultiFilters {
  const current = filters[dimension] as Array<AuditMultiFilters[K][number]>;
  const next = current.includes(value)
    ? current.filter((v) => v !== value)
    : [...current, value];
  return { ...filters, [dimension]: next } as AuditMultiFilters;
}

export function clearAuditDimension<K extends keyof AuditMultiFilters>(
  filters: AuditMultiFilters,
  dimension: K,
): AuditMultiFilters {
  return { ...filters, [dimension]: [] } as AuditMultiFilters;
}

export interface AuditWorkerOption { id: string; name: string }
export interface AuditApproverOption { id: string; name: string }

export function auditWorkerOptions(rows: AuditRow[]): AuditWorkerOption[] {
  const seen = new Map<string, string>();
  for (const r of rows) if (!seen.has(r.worker.id)) seen.set(r.worker.id, r.worker.name);
  return Array.from(seen, ([id, name]) => ({ id, name })).sort((a, b) => a.name.localeCompare(b.name));
}

export function auditApproverOptions(rows: AuditRow[]): AuditApproverOption[] {
  const seen = new Map<string, string>();
  for (const r of rows) if (r.approver && !seen.has(r.approver.id)) seen.set(r.approver.id, r.approver.name);
  return Array.from(seen, ([id, name]) => ({ id, name })).sort((a, b) => a.name.localeCompare(b.name));
}

export function auditActionTypeOptions(rows: AuditRow[]): string[] {
  const set = new Set<string>(SUGGESTED_ACTION_TYPES);
  for (const r of rows) set.add(r.actionType);
  return Array.from(set).sort();
}

/** Sentence-case a snake_case action-type identifier for display. */
export function actionTypeLabel(actionType: string): string {
  const spaced = actionType.replace(/_/g, " ").trim();
  if (spaced.length === 0) return actionType;
  return spaced.charAt(0).toUpperCase() + spaced.slice(1);
}
