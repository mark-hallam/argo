"use client";

import { useQuery } from "@tanstack/react-query";
import {
  fetchWorkerOverview,
  WorkerOverviewNotFoundError,
  type FetchWorkerOverviewResult,
} from "@/lib/api/worker-overview";

export const workerOverviewQueryKey = (workerId: string) =>
  ["worker", workerId, "overview"] as const;

/**
 * Fetch the Overview-tab payload (charter / duties / KPIs / recent
 * activity) for a worker. Falls back to the bundled sample dataset
 * in dev when the gateway endpoint 404s — see `fetchWorkerOverview`.
 *
 * Cache key is shared across all consumers of the same workerId, so
 * the layout's WorkerHero (via `useWorker`) and the Overview body
 * (via this hook) never race-fetch the same data.
 */
export function useWorkerOverview(workerId: string) {
  return useQuery<FetchWorkerOverviewResult, Error>({
    queryKey: workerOverviewQueryKey(workerId),
    queryFn: () => fetchWorkerOverview(workerId),
    retry: (failureCount, err) => {
      if (err instanceof WorkerOverviewNotFoundError) return false;
      return failureCount < 2;
    },
  });
}
