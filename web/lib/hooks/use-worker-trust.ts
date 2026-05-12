"use client";

import { useQuery } from "@tanstack/react-query";
import {
  fetchWorkerTrustGrade,
  type FetchTrustGradeResult,
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
