"use client";

import { useMutation, useQueryClient } from "@tanstack/react-query";
import { pauseWorker, resumeWorker } from "@/lib/api/workers";
import type { FetchTeamResult } from "@/lib/api/team";
import type { Worker, WorkerStatus } from "@/lib/types/worker";

const TEAM_KEY = ["team"] as const;

type TeamCache = FetchTeamResult | undefined;

function applyStatus(
  cache: TeamCache,
  workerId: string,
  status: WorkerStatus,
): TeamCache {
  if (!cache) return cache;
  return {
    ...cache,
    workers: cache.workers.map((w: Worker) =>
      w.id === workerId ? { ...w, status } : w,
    ),
  };
}

interface UseWorkerActionsApi {
  pauseWorker: (workerId: string) => Promise<void>;
  resumeWorker: (workerId: string) => Promise<void>;
  /** True while either mutation is in-flight for this worker. */
  isPending: boolean;
}

/**
 * Pause/resume mutations with optimistic update of the team query
 * cache. The card flips status immediately; on error we restore the
 * snapshot we took on `onMutate`.
 *
 * The hook is keyed per worker so multiple cards on the page each
 * track their own pending state — a pause-in-progress on Mariah
 * doesn't disable the menu on Priya.
 */
export function useWorkerActions(workerId: string): UseWorkerActionsApi {
  const qc = useQueryClient();

  const pauseMutation = useMutation({
    mutationFn: () => pauseWorker(workerId),
    onMutate: async () => {
      await qc.cancelQueries({ queryKey: TEAM_KEY });
      const prev = qc.getQueryData<FetchTeamResult>(TEAM_KEY);
      qc.setQueryData<FetchTeamResult>(TEAM_KEY, (cache) =>
        applyStatus(cache, workerId, "paused"),
      );
      return { prev };
    },
    onError: (_err, _vars, ctx) => {
      if (ctx?.prev) qc.setQueryData(TEAM_KEY, ctx.prev);
    },
  });

  const resumeMutation = useMutation({
    mutationFn: () => resumeWorker(workerId),
    onMutate: async () => {
      await qc.cancelQueries({ queryKey: TEAM_KEY });
      const prev = qc.getQueryData<FetchTeamResult>(TEAM_KEY);
      qc.setQueryData<FetchTeamResult>(TEAM_KEY, (cache) =>
        applyStatus(cache, workerId, "idle"),
      );
      return { prev };
    },
    onError: (_err, _vars, ctx) => {
      if (ctx?.prev) qc.setQueryData(TEAM_KEY, ctx.prev);
    },
  });

  return {
    pauseWorker: async () => {
      await pauseMutation.mutateAsync();
    },
    resumeWorker: async () => {
      await resumeMutation.mutateAsync();
    },
    isPending: pauseMutation.isPending || resumeMutation.isPending,
  };
}
