"use client";

import { useInfiniteQuery, useQuery } from "@tanstack/react-query";
import { fetchAuditFeed } from "@/lib/api/audit";
import type { AuditFilters, AuditFeedPage } from "@/lib/types/audit";

/**
 * Infinite-query hook for the cross-worker audit timeline. Cursor
 * pagination flows through TanStack `useInfiniteQuery`:
 *
 * - `pageParam: null` → first page (matches the gateway contract
 *   `fetchAuditFeed({ ...filters, cursor: null })`).
 * - `getNextPageParam` reads `nextCursor` from the response. The
 *   server is authoritative about "no more pages" (cursor `null`).
 *
 * The query key includes the filters object (sans cursor) so that
 * TanStack hashes it deterministically — changing any filter
 * triggers a fresh pagination cycle (a refetch, not a "load more").
 * The cursor is threaded through `pageParam` instead so all pages
 * under one filter set share the same cache key.
 */
export const auditFeedQueryKey = (filters: AuditFilters) => {
  // Drop the cursor from the key — TanStack manages it via pageParam.
  const { cursor: _cursor, ...rest } = filters;
  return ["audit-feed", rest] as const;
};

export function useAuditFeed(
  filters: AuditFilters,
  options?: { enabled?: boolean },
) {
  return useInfiniteQuery<AuditFeedPage, Error>({
    queryKey: auditFeedQueryKey(filters),
    initialPageParam: null as string | null,
    queryFn: ({ pageParam }) =>
      fetchAuditFeed({ ...filters, cursor: (pageParam as string | null) ?? null }),
    getNextPageParam: (lastPage) => lastPage.nextCursor,
    enabled: options?.enabled ?? true,
  });
}

export type AuditWorkspaceState = "loading" | "fresh" | "active";

/**
 * Look-back probe used by `<AuditEmpty>` to decide whether an empty
 * primary feed means "fresh workspace with no activity ever" vs
 * "established workspace that's just quiet in the selected range."
 *
 * Fires a single-page `?range=30d` query with no q/filters; the result
 * count alone determines the verdict (we don't even need to inspect
 * the rows). `enabled` is wired by the caller so this only runs when
 * the primary feed has settled to a no-rows state — otherwise we'd
 * double-query on every initial mount.
 */
export function useAuditWorkspaceFreshness(options?: {
  enabled?: boolean;
}): AuditWorkspaceState {
  const probeFilters: AuditFilters = { range: "30d" };
  const query = useQuery<AuditFeedPage, Error>({
    queryKey: ["audit-workspace-freshness"],
    queryFn: () => fetchAuditFeed({ ...probeFilters, cursor: null }),
    enabled: options?.enabled ?? true,
    staleTime: 60_000,
  });
  if (query.isPending || query.isFetching) return "loading";
  if (query.isError) return "loading";
  return (query.data?.rows.length ?? 0) > 0 ? "active" : "fresh";
}
