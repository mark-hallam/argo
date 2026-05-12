"use client";

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  createRetro,
  fetchRetrosForWorker,
  fetchRetrosIndex,
  finalizeRetro,
  updateRetroSections,
  type CreateRetroInput,
  type DataSource,
  type FinalizeRetroInput,
  type Retro,
  type RetrosListResult,
  type UpdateRetroInput,
} from "@/lib/api/retros";

const RETROS_WORKER_KEY = (workerId: string) =>
  ["retros", "worker", workerId] as const;
const RETROS_INDEX_KEY = (q: string) => ["retros", "index", q] as const;

/** List retros for one worker. Cache key is per-worker so navigating
 *  between profiles doesn't blow the global retro cache. */
export function useWorkerRetros(workerId: string | null) {
  const query = useQuery({
    queryKey: workerId ? RETROS_WORKER_KEY(workerId) : (["retros", "worker", "_none"] as const),
    queryFn: () => fetchRetrosForWorker(workerId ?? ""),
    enabled: Boolean(workerId),
  });
  return {
    ...query,
    data: query.data?.items as Retro[] | undefined,
    dataSource: query.data?.source as DataSource | undefined,
  };
}

/** Tenant-scoped retro index. The `q` argument is folded into the
 *  cache key so theme searches don't collide. */
export function useRetrosIndex(q: string) {
  const query = useQuery({
    queryKey: RETROS_INDEX_KEY(q),
    queryFn: () => fetchRetrosIndex(q || undefined),
  });
  return {
    ...query,
    data: query.data?.items as Retro[] | undefined,
    dataSource: query.data?.source as DataSource | undefined,
  };
}

/** Create a fresh retro. Invalidates the worker-scoped list so the
 *  dialog sees the new row on close. */
export function useCreateRetro() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (input: CreateRetroInput) => createRetro(input),
    onSuccess: (_retro, vars) => {
      qc.invalidateQueries({ queryKey: RETROS_WORKER_KEY(vars.workerId) });
      qc.invalidateQueries({ queryKey: ["retros", "index"] });
    },
  });
}

/** PATCH the sections of a draft retro. Optimistically updates the
 *  per-worker cache so the dialog stepper feels responsive; rolls
 *  back on error. */
export function useUpdateRetroSections() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (input: UpdateRetroInput) => updateRetroSections(input),
    onMutate: async (vars) => {
      await qc.cancelQueries({ queryKey: RETROS_WORKER_KEY(vars.workerId) });
      const prev = qc.getQueryData<RetrosListResult>(RETROS_WORKER_KEY(vars.workerId));
      if (prev) {
        qc.setQueryData<RetrosListResult>(RETROS_WORKER_KEY(vars.workerId), {
          ...prev,
          items: prev.items.map((r) =>
            r.retroId === vars.retroId
              ? { ...r, sections: { ...r.sections, ...vars.sections } }
              : r,
          ),
        });
      }
      return { prev };
    },
    onError: (_e, vars, ctx) => {
      if (ctx?.prev) {
        qc.setQueryData(RETROS_WORKER_KEY(vars.workerId), ctx.prev);
      }
    },
    onSuccess: (_r, vars) => {
      qc.invalidateQueries({ queryKey: RETROS_WORKER_KEY(vars.workerId) });
    },
  });
}

/** POST .../finalize. Locks the retro in the cache before the round
 *  trip so the dialog Finalize button can collapse immediately. */
export function useFinalizeRetro() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (input: FinalizeRetroInput) => finalizeRetro(input),
    onMutate: async (vars) => {
      await qc.cancelQueries({ queryKey: RETROS_WORKER_KEY(vars.workerId) });
      const prev = qc.getQueryData<RetrosListResult>(RETROS_WORKER_KEY(vars.workerId));
      if (prev) {
        const now = Math.floor(Date.now() / 1000);
        qc.setQueryData<RetrosListResult>(RETROS_WORKER_KEY(vars.workerId), {
          ...prev,
          items: prev.items.map((r) =>
            r.retroId === vars.retroId
              ? { ...r, finalized: true, finalizedAt: now }
              : r,
          ),
        });
      }
      return { prev };
    },
    onError: (_e, vars, ctx) => {
      if (ctx?.prev) {
        qc.setQueryData(RETROS_WORKER_KEY(vars.workerId), ctx.prev);
      }
    },
    onSuccess: (_r, vars) => {
      qc.invalidateQueries({ queryKey: RETROS_WORKER_KEY(vars.workerId) });
      qc.invalidateQueries({ queryKey: ["retros", "index"] });
    },
  });
}
