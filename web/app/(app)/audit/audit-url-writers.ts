"use client";

import type { useRouter } from "next/navigation";
import {
  AUDIT_RANGE_PRESETS,
  DEFAULT_AUDIT_RANGE,
  type AuditRangePreset,
} from "@/lib/types/audit";
import {
  AUDIT_FILTER_DIMENSIONS,
  isAuditFilterEmpty,
  serializeAuditMultiFilters,
  type AuditMultiFilters,
} from "@/lib/filters/audit";
import { lookupWorkerIdFromSlug } from "@/lib/sample-personas";
import { notifyAuditUrlChange } from "./use-audit-url-state";

/**
 * Pure URL read/write helpers for the `/audit` route, extracted from
 * `page.tsx` to keep that file under the 500-LOC ceiling. The write
 * helpers notify the non-suspending feed-filter store
 * (`notifyAuditUrlChange`) so the hoisted `useAuditFeed` re-keys after a
 * soft `router.replace` — see `use-audit-url-state.ts` for why the feed
 * fetch lives above the `useSearchParams()` Suspense boundary.
 */
export function isPristineState(
  range: AuditRangePreset,
  q: string | undefined,
  multi: AuditMultiFilters,
): boolean {
  return range === DEFAULT_AUDIT_RANGE && !q && isAuditFilterEmpty(multi);
}

export function parseRange(params: URLSearchParams | null): AuditRangePreset {
  const raw = params?.get("range") ?? "";
  return (AUDIT_RANGE_PRESETS as readonly string[]).includes(raw)
    ? (raw as AuditRangePreset)
    : DEFAULT_AUDIT_RANGE;
}

// Deep links from the worker Activity tab arrive with a human-readable
// slug (`?worker=mariah`) so the address bar doesn't leak the UUID.
// Translate any known slugs back to UUIDs before the rest of the
// filter machinery — which keys on `row.worker.id` — sees the value.
// Unknown tokens (non-sample workers, raw UUIDs) pass through.
export function resolveWorkerSlugs(filters: AuditMultiFilters): AuditMultiFilters {
  if (filters.worker.length === 0) return filters;
  const resolved = filters.worker.map((token) => lookupWorkerIdFromSlug(token) ?? token);
  return { ...filters, worker: resolved };
}

export function writeSingleParam(
  router: ReturnType<typeof useRouter>,
  pathname: string,
  searchParams: URLSearchParams | null,
  key: string,
  value: string | null,
) {
  const params = new URLSearchParams(searchParams?.toString() ?? "");
  if (value && value.length > 0) params.set(key, value);
  else params.delete(key);
  const qs = params.toString();
  router.replace(qs ? `${pathname}?${qs}` : pathname, { scroll: false });
  // Soft `router.replace` does not emit `popstate`; notify the hoisted feed's
  // non-suspending URL reader so a range/q change re-keys the fetch.
  notifyAuditUrlChange();
}

export function writeMultiFilters(
  router: ReturnType<typeof useRouter>,
  pathname: string,
  searchParams: URLSearchParams | null,
  filters: AuditMultiFilters,
) {
  const params = new URLSearchParams(searchParams?.toString() ?? "");
  for (const dim of AUDIT_FILTER_DIMENSIONS) {
    params.delete(urlKey(dim));
  }
  const next = new URLSearchParams(serializeAuditMultiFilters(filters));
  next.forEach((value, key) => params.set(key, value));
  const qs = params.toString();
  router.replace(qs ? `${pathname}?${qs}` : pathname, { scroll: false });
  // Dimension filters don't affect the feed key, but range/q live in the
  // same URL — keep the reader in sync on every write for consistency.
  notifyAuditUrlChange();
}

export function urlKey(dim: keyof AuditMultiFilters): string {
  switch (dim) {
    case "actionType":
      return "action_type";
    case "costBand":
      return "cost_band";
    default:
      return dim;
  }
}
