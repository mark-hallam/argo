// Pure filter logic for the approvals inbox (E1-b). Three dimensions
// (worker / channel / status); values OR within a dimension and AND
// across dimensions. State lives in URL search params.

import type { ApprovalItem, ChannelKind } from "@/lib/sample-approvals";

export const CHANNEL_KINDS: readonly ChannelKind[] = [
  "slack", "imessage", "sms", "push", "web", "voice", "whatsapp",
] as const;

export type StatusKind = "pending" | "approved" | "refused";

export const STATUS_KINDS: readonly StatusKind[] = [
  "pending", "approved", "refused",
] as const;

export interface ApprovalFilters {
  worker: string[];
  channel: ChannelKind[];
  status: StatusKind[];
}

export const EMPTY_FILTERS: ApprovalFilters = {
  worker: [],
  channel: [],
  status: [],
};

function splitCsv(value: string | null | undefined): string[] {
  if (!value) return [];
  return value
    .split(",")
    .map((v) => v.trim())
    .filter((v) => v.length > 0);
}

function uniqSorted<T extends string>(values: T[]): T[] {
  return Array.from(new Set(values)).sort();
}

// Unknown channel/status tokens are dropped silently so a hand-typed
// URL can't crash the page; canonical serialization rounds them out.
export function parseFilters(
  params: URLSearchParams | { get(name: string): string | null },
): ApprovalFilters {
  return {
    worker: uniqSorted(splitCsv(params.get("worker"))),
    channel: uniqSorted(splitCsv(params.get("channel"))).filter(
      (c): c is ChannelKind => (CHANNEL_KINDS as readonly string[]).includes(c),
    ),
    status: uniqSorted(splitCsv(params.get("status"))).filter(
      (s): s is StatusKind => (STATUS_KINDS as readonly string[]).includes(s),
    ),
  };
}

export function serializeFilters(filters: ApprovalFilters): string {
  const params = new URLSearchParams();
  if (filters.worker.length > 0) params.set("worker", uniqSorted(filters.worker).join(","));
  if (filters.channel.length > 0) params.set("channel", uniqSorted(filters.channel).join(","));
  if (filters.status.length > 0) params.set("status", uniqSorted(filters.status).join(","));
  return params.toString();
}

export function isFilterEmpty(filters: ApprovalFilters): boolean {
  return (
    filters.worker.length === 0 &&
    filters.channel.length === 0 &&
    filters.status.length === 0
  );
}

export function activeFilterCount(filters: ApprovalFilters): number {
  return (
    (filters.worker.length > 0 ? 1 : 0) +
    (filters.channel.length > 0 ? 1 : 0) +
    (filters.status.length > 0 ? 1 : 0)
  );
}

function itemStatus(item: ApprovalItem): StatusKind {
  if (item.decision === "approve") return "approved";
  if (item.decision === "refuse") return "refused";
  return "pending";
}

export function applyFilters(items: ApprovalItem[], filters: ApprovalFilters): ApprovalItem[] {
  if (isFilterEmpty(filters)) return items;
  return items.filter((item) => {
    if (filters.worker.length > 0 && !filters.worker.includes(item.worker.name)) return false;
    if (filters.channel.length > 0 && !filters.channel.includes(item.channel)) return false;
    if (filters.status.length > 0 && !filters.status.includes(itemStatus(item))) return false;
    return true;
  });
}

export function toggleFilterValue<K extends keyof ApprovalFilters>(
  filters: ApprovalFilters,
  dimension: K,
  value: ApprovalFilters[K][number],
): ApprovalFilters {
  const current = filters[dimension] as Array<ApprovalFilters[K][number]>;
  const next = current.includes(value)
    ? current.filter((v) => v !== value)
    : [...current, value];
  return { ...filters, [dimension]: next } as ApprovalFilters;
}

export function clearDimension<K extends keyof ApprovalFilters>(
  filters: ApprovalFilters,
  dimension: K,
): ApprovalFilters {
  return { ...filters, [dimension]: [] } as ApprovalFilters;
}

export function workerOptions(items: ApprovalItem[]): string[] {
  return uniqSorted(items.map((i) => i.worker.name));
}
