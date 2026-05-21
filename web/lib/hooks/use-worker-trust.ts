"use client";

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  fetchWorkerTrustGrade,
  recomputeWorkerTrustGrade,
  type FetchTrustGradeResult,
  type TrustRecomputeResult,
} from "@/lib/api/worker-trust";

export const workerTrustGradeQueryKey = (workerId: string) =>
  ["worker-trust-grade", workerId] as const;

/** Read-only snapshot of a worker's trust grade. The endpoint is
 *  intentionally cheap (one row + 3 derived fields) so we let
 *  TanStack's defaults handle refetch / focus revalidation. */
export function useWorkerTrustGrade(workerId: string, options?: { enabled?: boolean }) {
  return useQuery<FetchTrustGradeResult>({
    queryKey: workerTrustGradeQueryKey(workerId),
    queryFn: () => fetchWorkerTrustGrade(workerId),
    enabled: options?.enabled ?? Boolean(workerId),
  });
}

/** Admin/owner-only mutation that asks the gateway to run the
 *  trust-engine synchronously for one worker. On success the
 *  trust-grade query is invalidated so the badge refreshes
 *  immediately (the engine writes the grade column inside the same
 *  request handler). */
export function useRecomputeWorkerTrustGrade(workerId: string) {
  const client = useQueryClient();
  return useMutation<TrustRecomputeResult, Error>({
    mutationFn: () => recomputeWorkerTrustGrade(workerId),
    onSuccess: () => {
      void client.invalidateQueries({ queryKey: workerTrustGradeQueryKey(workerId) });
    },
  });
}
