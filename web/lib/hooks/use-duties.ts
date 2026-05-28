"use client";

import { useMutation, useQueryClient } from "@tanstack/react-query";
import {
  toggleDuty,
  updateDuty,
  type ToggleDutyRequest,
  type UpdateDutyRequest,
  type UpdateDutyResult,
} from "@/lib/api/duties";
import { workerOverviewQueryKey } from "@/lib/hooks/use-worker-overview";
import type {
  FetchWorkerOverviewResult,
  WorkerDuty,
} from "@/lib/api/worker-overview";

/**
 * Per-duty mutation hooks consumed by `WorkerOverviewContent` /
 * `EditDutyDialog`. Both write the gateway-returned (or mock-derived)
 * `WorkerDuty` straight back into the cached overview so the row
 * re-renders with persisted state — no extra refetch round-trip.
 */

function findCachedDuty(
  cached: FetchWorkerOverviewResult | undefined,
  dutyId: string,
): WorkerDuty | null {
  if (!cached) return null;
  return cached.overview.duties.find((d) => d.id === dutyId) ?? null;
}

function applyDutyToCache(
  cached: FetchWorkerOverviewResult | undefined,
  next: WorkerDuty,
): FetchWorkerOverviewResult | undefined {
  if (!cached) return cached;
  const replaced = cached.overview.duties.map((d) =>
    d.id === next.id ? next : d,
  );
  return {
    ...cached,
    overview: { ...cached.overview, duties: replaced },
  };
}

export function useUpdateDuty(workerId: string) {
  const queryClient = useQueryClient();
  const overviewKey = workerOverviewQueryKey(workerId);

  return useMutation<UpdateDutyResult, Error, Omit<UpdateDutyRequest, "workerId">>({
    mutationFn: (req) => {
      const cached =
        queryClient.getQueryData<FetchWorkerOverviewResult>(overviewKey);
      const previous = findCachedDuty(cached, req.dutyId);
      return updateDuty({ ...req, workerId }, previous);
    },
    onSuccess: (result) => {
      queryClient.setQueryData<FetchWorkerOverviewResult>(overviewKey, (cached) =>
        applyDutyToCache(cached, result.duty),
      );
    },
  });
}

export function useToggleDuty(workerId: string) {
  const queryClient = useQueryClient();
  const overviewKey = workerOverviewQueryKey(workerId);

  return useMutation<
    UpdateDutyResult,
    Error,
    Omit<ToggleDutyRequest, "workerId">,
    { previous?: FetchWorkerOverviewResult }
  >({
    mutationFn: (req) => {
      const cached =
        queryClient.getQueryData<FetchWorkerOverviewResult>(overviewKey);
      const previous = findCachedDuty(cached, req.dutyId);
      return toggleDuty({ ...req, workerId }, previous);
    },
    onMutate: (req) => {
      const previous =
        queryClient.getQueryData<FetchWorkerOverviewResult>(overviewKey);
      if (previous) {
        const optimistic = previous.overview.duties.map((d) =>
          d.id === req.dutyId ? { ...d, enabled: req.enabled } : d,
        );
        queryClient.setQueryData<FetchWorkerOverviewResult>(overviewKey, {
          ...previous,
          overview: { ...previous.overview, duties: optimistic },
        });
      }
      return { previous };
    },
    onError: (_err, _req, ctx) => {
      if (ctx?.previous) {
        queryClient.setQueryData<FetchWorkerOverviewResult>(overviewKey, ctx.previous);
      }
    },
    onSuccess: (result) => {
      queryClient.setQueryData<FetchWorkerOverviewResult>(overviewKey, (cached) =>
        applyDutyToCache(cached, result.duty),
      );
    },
  });
}
