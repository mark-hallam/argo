"use client";

import { useCallback } from "react";
import {
  useMutation,
  useQuery,
  useQueryClient,
} from "@tanstack/react-query";
import {
  addFact,
  deleteFact,
  editFact,
  fetchMemory,
  WorkerMemoryNotFoundError,
  type FetchWorkerMemoryResult,
  type WorkerFact,
} from "@/lib/api/worker-memory";

export const workerMemoryQueryKey = (workerId: string) =>
  ["worker", workerId, "memory"] as const;

/**
 * Fetch the Memory tab payload (facts + active conversations +
 * recent decisions + voice profile) for a worker. Mirrors the
 * `useWorkerOverview` pattern: shares a stable query key with any
 * other consumer that wants to read the same dataset, and skips
 * retry on the typed `WorkerMemoryNotFoundError`.
 */
export function useWorkerMemory(workerId: string) {
  return useQuery<FetchWorkerMemoryResult, Error>({
    queryKey: workerMemoryQueryKey(workerId),
    queryFn: () => fetchMemory(workerId),
    retry: (failureCount, err) => {
      if (err instanceof WorkerMemoryNotFoundError) return false;
      return failureCount < 2;
    },
  });
}

/**
 * Optimistic-add mutation. Inserts the new fact at the top of the
 * cached list immediately, fires the POST, and reconciles the
 * server-issued id on success (rolls back on error). Matches the
 * approvals-page optimistic-write pattern used elsewhere in the
 * SPA.
 */
export function useAddFact(workerId: string) {
  const queryClient = useQueryClient();
  const queryKey = workerMemoryQueryKey(workerId);

  return useMutation<
    WorkerFact,
    Error,
    { text: string },
    { previous?: FetchWorkerMemoryResult; tempId: string }
  >({
    mutationFn: ({ text }) => addFact(workerId, { text }),
    onMutate: async ({ text }) => {
      await queryClient.cancelQueries({ queryKey });
      const previous = queryClient.getQueryData<FetchWorkerMemoryResult>(queryKey);
      const tempId = `f_tmp_${Date.now()}`;
      if (previous) {
        const optimistic: WorkerFact = {
          factId: tempId,
          text,
          createdAt: new Date().toISOString(),
        };
        queryClient.setQueryData<FetchWorkerMemoryResult>(queryKey, {
          ...previous,
          memory: {
            ...previous.memory,
            facts: [optimistic, ...previous.memory.facts],
          },
        });
      }
      return { previous, tempId };
    },
    onError: (_err, _vars, ctx) => {
      if (ctx?.previous) queryClient.setQueryData(queryKey, ctx.previous);
    },
    onSuccess: (created, _vars, ctx) => {
      const current = queryClient.getQueryData<FetchWorkerMemoryResult>(queryKey);
      if (!current || !ctx) return;
      const reconciled = current.memory.facts.map((f) =>
        f.factId === ctx.tempId ? created : f,
      );
      queryClient.setQueryData<FetchWorkerMemoryResult>(queryKey, {
        ...current,
        memory: { ...current.memory, facts: reconciled },
      });
    },
  });
}

export function useEditFact(workerId: string) {
  const queryClient = useQueryClient();
  const queryKey = workerMemoryQueryKey(workerId);

  return useMutation<
    WorkerFact,
    Error,
    { factId: string; text: string },
    { previous?: FetchWorkerMemoryResult }
  >({
    mutationFn: ({ factId, text }) => editFact(workerId, factId, { text }),
    onMutate: async ({ factId, text }) => {
      await queryClient.cancelQueries({ queryKey });
      const previous = queryClient.getQueryData<FetchWorkerMemoryResult>(queryKey);
      if (previous) {
        const next = previous.memory.facts.map((f) =>
          f.factId === factId ? { ...f, text } : f,
        );
        queryClient.setQueryData<FetchWorkerMemoryResult>(queryKey, {
          ...previous,
          memory: { ...previous.memory, facts: next },
        });
      }
      return { previous };
    },
    onError: (_err, _vars, ctx) => {
      if (ctx?.previous) queryClient.setQueryData(queryKey, ctx.previous);
    },
  });
}

export function useDeleteFact(workerId: string) {
  const queryClient = useQueryClient();
  const queryKey = workerMemoryQueryKey(workerId);

  return useMutation<
    void,
    Error,
    { factId: string },
    { previous?: FetchWorkerMemoryResult }
  >({
    mutationFn: ({ factId }) => deleteFact(workerId, factId),
    onMutate: async ({ factId }) => {
      await queryClient.cancelQueries({ queryKey });
      const previous = queryClient.getQueryData<FetchWorkerMemoryResult>(queryKey);
      if (previous) {
        queryClient.setQueryData<FetchWorkerMemoryResult>(queryKey, {
          ...previous,
          memory: {
            ...previous.memory,
            facts: previous.memory.facts.filter((f) => f.factId !== factId),
          },
        });
      }
      return { previous };
    },
    onError: (_err, _vars, ctx) => {
      if (ctx?.previous) queryClient.setQueryData(queryKey, ctx.previous);
    },
  });
}

/**
 * Convenience composite — bundles the query + three mutation hooks
 * behind a single call site so the page component doesn't have to
 * repeat the workerId four times. Returns plain callbacks (not the
 * raw mutation objects) for the common case; consumers needing
 * granular state can call the individual hooks directly.
 */
export function useWorkerMemoryActions(workerId: string) {
  const add = useAddFact(workerId);
  const edit = useEditFact(workerId);
  const remove = useDeleteFact(workerId);

  const onAdd = useCallback((text: string) => add.mutateAsync({ text }), [add]);
  const onEdit = useCallback(
    (factId: string, text: string) => edit.mutateAsync({ factId, text }),
    [edit],
  );
  const onDelete = useCallback(
    (factId: string) => remove.mutateAsync({ factId }),
    [remove],
  );

  return {
    onAdd,
    onEdit,
    onDelete,
    isAdding: add.isPending,
    isEditing: edit.isPending,
    isDeleting: remove.isPending,
    error: add.error ?? edit.error ?? remove.error ?? null,
  };
}
