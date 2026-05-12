"use client";

import { useInfiniteQuery } from "@tanstack/react-query";
import {
  fetchWorkerActivity,
  type WorkerActivityFilters,
  type WorkerActivityPage,
} from "@/lib/api/worker-activity";

/**
 * Infinite-query hook for the per-worker Activity feed. Cursor
 * pagination flows through TanStack `useInfiniteQuery`:
 *
 * - `pageParam: null` → first page (matches the gateway contract
 *   `fetchWorkerActivity(id, filters, null)`).
 * - `getNextPageParam` reads `nextCursor` from the response. The
 *   server is authoritative about "no more pages" (cursor `null`).
 *
 * The query key includes the filters object — TanStack hashes it
 * deterministically, so changing any filter triggers a fresh
 * pagination cycle (a refetch, not a "load more"). The per-worker
 * Activity component re-instantiates the hook with the new filters
 * and gets a clean infinite-list under a new cache key.
 */
export const workerActivityQueryKey = (
  workerId: string,
  filters: WorkerActivityFilters,
) => ["worker-activity", workerId, filters] as const;

export function useWorkerActivity(
  workerId: string,
  filters: WorkerActivityFilters,
  options?: { enabled?: boolean },
) {
  return useInfiniteQuery<WorkerActivityPage, Error>({
    queryKey: workerActivityQueryKey(workerId, filters),
    initialPageParam: null as string | null,
    queryFn: ({ pageParam }) =>
      fetchWorkerActivity(workerId, filters, (pageParam as string | null) ?? null),
    getNextPageParam: (lastPage) => lastPage.nextCursor,
    enabled: options?.enabled ?? true,
  });
}
