"use client";

import { useInfiniteQuery } from "@tanstack/react-query";
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
