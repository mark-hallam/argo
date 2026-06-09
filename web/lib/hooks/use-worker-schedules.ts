"use client";

import { useQuery } from "@tanstack/react-query";
import { listDuties, type ListDutiesResult } from "@/lib/api/duties";
import type { WorkerDuty } from "@/lib/api/worker-overview";

export const workerSchedulesQueryKey = (workerId: string) =>
  ["worker", workerId, "schedules"] as const;

/**
 * Fetch the worker's persisted schedules (real `role_duty` rows) for
 * the Schedules tab via the live `listDuties` reader shipped in #2742.
 *
 * `listDuties` carries the NO-THEATER gate (DEC-007 / DEC-029): with a
 * real session OR demo-live, a gateway 404 / malformed body throws an
 * honest error instead of leaking the bundled sample. The optional
 * `fallback` (the bundled sample duties for this worker) is consumed
 * ONLY on the anonymous-dev path, so a signed-in / demo-live operator
 * sees their actual schedules or an honest empty state — never fixtures.
 *
 * Distinct query key from `useWorkerOverview` so the Schedules tab owns
 * its own cache slot and never re-renders off the overview's fabricated
 * `duties` array.
 */
export function useWorkerSchedules(
  workerId: string,
  fallback?: () => WorkerDuty[],
) {
  return useQuery<ListDutiesResult, Error>({
    queryKey: workerSchedulesQueryKey(workerId),
    queryFn: () => listDuties(workerId, fallback),
    retry: (failureCount) => failureCount < 2,
  });
}
