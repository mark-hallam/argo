"use client";

import { useQuery } from "@tanstack/react-query";
import {
  fetchWorkerEscalations,
  type FetchWorkerEscalationsResult,
} from "@/lib/api/escalations";
import type { EscalationRule } from "@/components/skycot/workers/EscalationMatrix";

export const workerEscalationsQueryKey = (workerId: string) =>
  ["worker-escalations", workerId] as const;

/**
 * Read a worker's live escalation rule set for the Escalation tab.
 *
 * Delegates the NO-THEATER gating to {@link fetchWorkerEscalations}: a
 * real signed-in user (or a demo-live recording session) gets the live
 * `GET /api/v1/workers/<id>/escalations` rule set and an honest error on
 * failure; only anonymous dev/demo falls back to the bundled sample. The
 * `result.source` distinguishes the two so the page can honest-empty the
 * history/trust surfaces (which have no live feed) for the real path.
 *
 * `fallbackRules` supplies the page's richer per-worker sample rules for
 * the anonymous-dev/offline path; a real/demo-live operator never sees
 * it (see {@link fetchWorkerEscalations}).
 */
export function useWorkerEscalations(
  workerId: string,
  fallbackRules?: () => EscalationRule[],
) {
  return useQuery<FetchWorkerEscalationsResult, Error>({
    queryKey: workerEscalationsQueryKey(workerId),
    queryFn: () => fetchWorkerEscalations(workerId, fallbackRules),
    enabled: workerId.length > 0,
    retry: 1,
  });
}
