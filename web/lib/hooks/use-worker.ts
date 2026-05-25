"use client";

import { useQuery } from "@tanstack/react-query";
import {
  AuthNotReadyError,
  fetchWorker,
  WorkerNotFoundError,
} from "@/lib/api/workers";
import type { Worker } from "@/lib/types/worker";

export const workerQueryKey = (workerId: string) => ["worker", workerId] as const;

/**
 * Fetch a single worker by id. v1 falls back to the bundled mock
 * dataset on gateway 404 in dev (see `fetchWorker`). Production
 * propagates 404s as `WorkerNotFoundError` so the page can render
 * the not-found state. 401s during the session-token race become
 * `AuthNotReadyError`; the short retry loop below gives
 * `/api/auth/session` time to resolve before either the fetch
 * succeeds or the error propagates as a real failure.
 *
 * E4-b..i tabs read the cached worker via this same hook — sharing
 * the query key avoids per-tab refetches.
 */
export function useWorker(workerId: string) {
  return useQuery<Worker, Error>({
    queryKey: workerQueryKey(workerId),
    queryFn: () => fetchWorker(workerId),
    retry: (failureCount, err) => {
      // Real 404 → render not-found immediately, don't burn retries.
      if (err instanceof WorkerNotFoundError) return false;
      // Session-token race → retry up to 3 times. The bearer token
      // peek resolves on the first `/api/auth/session` round-trip
      // (typically <150ms), so 3 short retries handle the cold-nav
      // window without making genuine auth failures linger.
      if (err instanceof AuthNotReadyError) return failureCount < 3;
      return failureCount < 2;
    },
    retryDelay: (attempt, err) => {
      if (err instanceof AuthNotReadyError) {
        // 120ms → 240ms → 480ms. Worst-case ~840ms total — under the
        // brief's 1-2s acceptance budget for cold navigation.
        return Math.min(120 * 2 ** attempt, 600);
      }
      return Math.min(1000 * 2 ** attempt, 30000);
    },
  });
}
