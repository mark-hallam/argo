"use client";

import { useQuery } from "@tanstack/react-query";
import { fetchWorker, WorkerNotFoundError } from "@/lib/api/workers";
import type { Worker } from "@/lib/types/worker";

export const workerQueryKey = (workerId: string) => ["worker", workerId] as const;

/**
 * Fetch a single worker by id. v1 falls back to the bundled mock
 * dataset on gateway 404 in dev (see `fetchWorker`). Production
 * propagates 404s as `WorkerNotFoundError` so the page can render
 * the not-found state.
 *
 * E4-b..i tabs read the cached worker via this same hook — sharing
 * the query key avoids per-tab refetches.
 */
export function useWorker(workerId: string) {
  return useQuery<Worker, Error>({
    queryKey: workerQueryKey(workerId),
    queryFn: () => fetchWorker(workerId),
    retry: (failureCount, err) => {
      if (err instanceof WorkerNotFoundError) return false;
      return failureCount < 2;
    },
  });
}
