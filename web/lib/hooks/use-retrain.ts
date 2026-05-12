"use client";

import { useCallback } from "react";
import {
  useMutation,
  useQuery,
  useQueryClient,
} from "@tanstack/react-query";
import {
  classifyRetrain,
  commitRetrain,
  listRecentRetrains,
  previewReverseTest,
  undoRetrain,
  type ClassifyRetrainRequest,
  type ClassifyRetrainResult,
  type CommitRetrainRequest,
  type CommitRetrainResult,
  type ListRecentRetrainsResult,
  type RetrainRecord,
  type ReverseTestResult,
} from "@/lib/api/retrain";

export const retrainRecentQueryKey = (workerId: string) =>
  ["worker", workerId, "retrain", "recent"] as const;

/**
 * Recent-retrains query backing the RecentChangesPanel. Falls back
 * to an empty list when the gateway endpoint isn't wired (E4-g v1
 * ships ahead of `mission-gateway-retrain-endpoints`).
 */
export function useRecentRetrains(workerId: string) {
  return useQuery<ListRecentRetrainsResult, Error>({
    queryKey: retrainRecentQueryKey(workerId),
    queryFn: () => listRecentRetrains(workerId),
    retry: (failureCount) => failureCount < 1,
  });
}

/** Classify the user's free-text correction into a structured proposal. */
export function useClassifyRetrain() {
  return useMutation<ClassifyRetrainResult, Error, ClassifyRetrainRequest>({
    mutationFn: (req) => classifyRetrain(req),
  });
}

/**
 * Commit the proposal. Optimistically prepends the new record to the
 * recent-changes cache so the panel surfaces it immediately; rolls
 * back on error.
 */
export function useCommitRetrain(workerId: string) {
  const queryClient = useQueryClient();
  const queryKey = retrainRecentQueryKey(workerId);

  return useMutation<
    CommitRetrainResult,
    Error,
    CommitRetrainRequest,
    { previous?: ListRecentRetrainsResult }
  >({
    mutationFn: (req) => commitRetrain(req),
    onMutate: async () => {
      await queryClient.cancelQueries({ queryKey });
      const previous = queryClient.getQueryData<ListRecentRetrainsResult>(queryKey);
      return { previous };
    },
    onError: (_err, _vars, ctx) => {
      if (ctx?.previous) queryClient.setQueryData(queryKey, ctx.previous);
    },
    onSuccess: (result) => {
      const current = queryClient.getQueryData<ListRecentRetrainsResult>(queryKey) ?? {
        records: [],
        source: "mock" as const,
      };
      queryClient.setQueryData<ListRecentRetrainsResult>(queryKey, {
        ...current,
        records: [result.record, ...current.records],
      });
    },
  });
}

/** Undo a recent retrain. Marks the record `undoneAt` in cache. */
export function useUndoRetrain(workerId: string) {
  const queryClient = useQueryClient();
  const queryKey = retrainRecentQueryKey(workerId);

  return useMutation<RetrainRecord, Error, { retrainId: string }>({
    mutationFn: ({ retrainId }) => undoRetrain(workerId, retrainId),
    onSuccess: (record) => {
      const current = queryClient.getQueryData<ListRecentRetrainsResult>(queryKey);
      if (!current) return;
      const next = current.records.map((r) =>
        r.retrainId === record.retrainId
          ? { ...r, undoneAt: record.undoneAt ?? new Date().toISOString() }
          : r,
      );
      queryClient.setQueryData<ListRecentRetrainsResult>(queryKey, {
        ...current,
        records: next,
      });
    },
  });
}

/**
 * Trigger a reverse-test preview — re-handles 3 prior conversations
 * with the new rule applied. Read-only; nothing is sent.
 */
export function useReverseTest(workerId: string) {
  return useMutation<ReverseTestResult, Error, { retrainId: string }>({
    mutationFn: ({ retrainId }) => previewReverseTest(workerId, retrainId),
  });
}

export interface RetrainActions {
  classify: (req: Omit<ClassifyRetrainRequest, "workerId">) => Promise<ClassifyRetrainResult>;
  commit: (req: Omit<CommitRetrainRequest, "workerId">) => Promise<CommitRetrainResult>;
  undo: (retrainId: string) => Promise<RetrainRecord>;
  reverseTest: (retrainId: string) => Promise<ReverseTestResult>;
  isClassifying: boolean;
  isCommitting: boolean;
}

/**
 * Convenience composite — bundles the four retrain mutations behind
 * a single hook call. Mirrors the `useWorkerMemoryActions` pattern.
 */
export function useRetrainActions(workerId: string): RetrainActions {
  const classify = useClassifyRetrain();
  const commit = useCommitRetrain(workerId);
  const undo = useUndoRetrain(workerId);
  const reverseTest = useReverseTest(workerId);

  const doClassify = useCallback(
    (req: Omit<ClassifyRetrainRequest, "workerId">) =>
      classify.mutateAsync({ ...req, workerId }),
    [classify, workerId],
  );
  const doCommit = useCallback(
    (req: Omit<CommitRetrainRequest, "workerId">) =>
      commit.mutateAsync({ ...req, workerId }),
    [commit, workerId],
  );
  const doUndo = useCallback(
    (retrainId: string) => undo.mutateAsync({ retrainId }),
    [undo],
  );
  const doReverse = useCallback(
    (retrainId: string) => reverseTest.mutateAsync({ retrainId }),
    [reverseTest],
  );

  return {
    classify: doClassify,
    commit: doCommit,
    undo: doUndo,
    reverseTest: doReverse,
    isClassifying: classify.isPending,
    isCommitting: commit.isPending,
  };
}
