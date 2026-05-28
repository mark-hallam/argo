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
 *
 * Streaming model (v1.0 demo): when `live` is on (default), the hook
 * polls the gateway every `WORKER_ACTIVITY_POLL_MS` while the tab is
 * visible. Background polling is disabled — `refetchIntervalInBackground:
 * false` pauses the timer when the document is hidden, so a backgrounded
 * tab doesn't spam audit-feed reads. SSE/WebSocket is the v1.x credible
 * streaming path; polling is enough for the demo MP4's "watch this —
 * give it a second" cadence.
 */

/** Polling cadence (ms) for the Activity tab when `live` is enabled. */
export const WORKER_ACTIVITY_POLL_MS = 3000;

/** Stale time (ms) — focus/mount re-render uses cached rows without an immediate refetch. */
export const WORKER_ACTIVITY_STALE_MS = 1000;

export const workerActivityQueryKey = (
  workerId: string,
  filters: WorkerActivityFilters,
) => ["worker-activity", workerId, filters] as const;

export function useWorkerActivity(
  workerId: string,
  filters: WorkerActivityFilters,
  options?: { enabled?: boolean; live?: boolean },
) {
  const live = options?.live ?? true;
  return useInfiniteQuery<WorkerActivityPage, Error>({
    queryKey: workerActivityQueryKey(workerId, filters),
    initialPageParam: null as string | null,
    queryFn: ({ pageParam }) =>
      fetchWorkerActivity(workerId, filters, (pageParam as string | null) ?? null),
    getNextPageParam: (lastPage) => lastPage.nextCursor,
    enabled: options?.enabled ?? true,
    refetchInterval: live ? WORKER_ACTIVITY_POLL_MS : false,
    refetchIntervalInBackground: false,
    staleTime: WORKER_ACTIVITY_STALE_MS,
  });
}
