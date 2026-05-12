"use client";

import { useCallback } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  archiveWorker as apiArchiveWorker,
  deleteWorker as apiDeleteWorker,
  pauseWorker as apiPauseWorker,
  getSettings,
  updateAutonomy as apiUpdateAutonomy,
  updateBudget as apiUpdateBudget,
  updateChannels as apiUpdateChannels,
  updateQuietHours as apiUpdateQuietHours,
  WorkerSettingsNotFoundError,
  type AutonomyConfig,
  type BudgetConfig,
  type ChannelsConfig,
  type FetchWorkerSettingsResult,
  type QuietHoursConfig,
  type SettingsMutationResult,
  type WorkerSettings,
} from "@/lib/api/worker-settings";

/**
 * Settings-tab data + mutation hooks.
 *
 * `useWorkerSettings(workerId)` reads the full payload (one query
 * key, no retry on 404) and the sibling action hooks return
 * mutation handles that optimistically patch the cached settings
 * before the network round-trip completes — with rollback on error.
 *
 * The optimistic transforms are colocated here so the section
 * components don't need to know about the cache shape.
 */

export const workerSettingsQueryKey = (workerId: string) =>
  ["worker", workerId, "settings"] as const;

export function useWorkerSettings(workerId: string) {
  return useQuery<FetchWorkerSettingsResult, Error>({
    queryKey: workerSettingsQueryKey(workerId),
    queryFn: () => getSettings(workerId),
    retry: (failureCount, err) => {
      if (err instanceof WorkerSettingsNotFoundError) return false;
      return failureCount < 2;
    },
  });
}

type Updater = (settings: WorkerSettings) => WorkerSettings;

function applyOptimistic(
  client: ReturnType<typeof useQueryClient>,
  workerId: string,
  updater: Updater,
): FetchWorkerSettingsResult | undefined {
  const key = workerSettingsQueryKey(workerId);
  const previous = client.getQueryData<FetchWorkerSettingsResult>(key);
  if (!previous) return undefined;
  client.setQueryData<FetchWorkerSettingsResult>(key, {
    ...previous,
    settings: updater(previous.settings),
  });
  return previous;
}

function rollback(
  client: ReturnType<typeof useQueryClient>,
  workerId: string,
  previous: FetchWorkerSettingsResult | undefined,
) {
  if (!previous) return;
  client.setQueryData(workerSettingsQueryKey(workerId), previous);
}

export function useUpdateChannels(workerId: string) {
  const client = useQueryClient();
  return useMutation<
    SettingsMutationResult,
    Error,
    ChannelsConfig,
    FetchWorkerSettingsResult | undefined
  >({
    mutationFn: (channels) => apiUpdateChannels(workerId, channels),
    onMutate: (channels) =>
      applyOptimistic(client, workerId, (s) => ({ ...s, channels })),
    onError: (_e, _v, prev) => rollback(client, workerId, prev),
  });
}

export function useUpdateQuietHours(workerId: string) {
  const client = useQueryClient();
  return useMutation<
    SettingsMutationResult,
    Error,
    QuietHoursConfig,
    FetchWorkerSettingsResult | undefined
  >({
    mutationFn: (quietHours) => apiUpdateQuietHours(workerId, quietHours),
    onMutate: (quietHours) =>
      applyOptimistic(client, workerId, (s) => ({ ...s, quietHours })),
    onError: (_e, _v, prev) => rollback(client, workerId, prev),
  });
}

export function useUpdateBudget(workerId: string) {
  const client = useQueryClient();
  return useMutation<
    SettingsMutationResult,
    Error,
    Pick<BudgetConfig, "dailyCapUsd" | "monthlyCapUsd">,
    FetchWorkerSettingsResult | undefined
  >({
    mutationFn: (budget) => apiUpdateBudget(workerId, budget),
    onMutate: (budget) =>
      applyOptimistic(client, workerId, (s) => ({
        ...s,
        budget: { ...s.budget, ...budget },
      })),
    onError: (_e, _v, prev) => rollback(client, workerId, prev),
  });
}

export function useUpdateAutonomy(workerId: string) {
  const client = useQueryClient();
  return useMutation<
    SettingsMutationResult,
    Error,
    AutonomyConfig,
    FetchWorkerSettingsResult | undefined
  >({
    mutationFn: (autonomy) => apiUpdateAutonomy(workerId, autonomy),
    onMutate: (autonomy) =>
      applyOptimistic(client, workerId, (s) => ({ ...s, autonomy })),
    onError: (_e, _v, prev) => rollback(client, workerId, prev),
  });
}

export function usePauseWorker(workerId: string) {
  return useMutation<SettingsMutationResult, Error, void>({
    mutationFn: () => apiPauseWorker(workerId),
  });
}

export function useArchiveWorker(workerId: string) {
  return useMutation<SettingsMutationResult, Error, void>({
    mutationFn: () => apiArchiveWorker(workerId),
  });
}

export function useDeleteWorker(workerId: string) {
  return useMutation<SettingsMutationResult, Error, void>({
    mutationFn: () => apiDeleteWorker(workerId),
  });
}

/** Convenience bundle for the page-level component. */
export function useSettingsActions(workerId: string) {
  const channels = useUpdateChannels(workerId);
  const quiet = useUpdateQuietHours(workerId);
  const budget = useUpdateBudget(workerId);
  const autonomy = useUpdateAutonomy(workerId);
  const pause = usePauseWorker(workerId);
  const archive = useArchiveWorker(workerId);
  const del = useDeleteWorker(workerId);

  const onUpdateChannels = useCallback(
    (next: ChannelsConfig) => channels.mutate(next),
    [channels],
  );
  const onUpdateQuietHours = useCallback(
    (next: QuietHoursConfig) => quiet.mutate(next),
    [quiet],
  );
  const onUpdateBudget = useCallback(
    (next: Pick<BudgetConfig, "dailyCapUsd" | "monthlyCapUsd">) => budget.mutate(next),
    [budget],
  );
  const onUpdateAutonomy = useCallback(
    (next: AutonomyConfig) => autonomy.mutate(next),
    [autonomy],
  );
  const onPause = useCallback(() => pause.mutate(), [pause]);
  const onArchive = useCallback(() => archive.mutate(), [archive]);
  const onDelete = useCallback(() => del.mutate(), [del]);

  return {
    onUpdateChannels,
    onUpdateQuietHours,
    onUpdateBudget,
    onUpdateAutonomy,
    onPause,
    onArchive,
    onDelete,
  };
}
