"use client";

import { useCallback } from "react";
import {
  useMutation,
  useQueryClient,
} from "@tanstack/react-query";
import {
  classifyDuty,
  commitDuty,
  previewDuty,
  type ClassifyDutyRequest,
  type ClassifyDutyResult,
  type CommitDutyRequest,
  type CommitDutyResult,
  type PreviewDutyRequest,
  type PreviewDutyResult,
} from "@/lib/api/duties";
import { retrainRecentQueryKey } from "@/lib/hooks/use-retrain";
import type {
  ListRecentRetrainsResult,
  RetrainRecord,
} from "@/lib/api/retrain";
import {
  workerOverviewQueryKey,
} from "@/lib/hooks/use-worker-overview";
import type { FetchWorkerOverviewResult } from "@/lib/api/worker-overview";

/**
 * Mutations powering AddResponsibilityDialog (E4-h).
 *
 *   - classify → Genesis-lite shape classification (step 2).
 *   - preview  → 3 sample outputs the duty would produce (step 3).
 *   - commit   → persist on the worker (step 5), optimistically
 *                inserting the new duty into the cached overview AND
 *                prepending a marker record into RecentChangesPanel's
 *                source cache so the addition surfaces alongside
 *                retrains (the panel doesn't yet know about duties —
 *                this is a v1 bridge until a unified change-feed
 *                lands).
 */

function dutyAddedRecord(
  workerId: string,
  result: CommitDutyResult,
): RetrainRecord {
  const now = new Date();
  return {
    retrainId: `dty_${result.duty.id}`,
    workerId,
    classification: "policy",
    oneLineSummary: `Duty added: ${result.duty.label}`,
    committedAt: now.toISOString(),
    // Duties don't carry an undo window — we surface them in the
    // panel for visibility, but the row is read-only.
    undoExpiresAt: now.toISOString(),
  };
}

export function useClassifyDuty() {
  return useMutation<ClassifyDutyResult, Error, ClassifyDutyRequest>({
    mutationFn: (req) => classifyDuty(req),
  });
}

export function usePreviewDuty() {
  return useMutation<PreviewDutyResult, Error, PreviewDutyRequest>({
    mutationFn: (req) => previewDuty(req),
  });
}

export function useCommitDuty(workerId: string) {
  const queryClient = useQueryClient();
  const overviewKey = workerOverviewQueryKey(workerId);
  const recentKey = retrainRecentQueryKey(workerId);

  return useMutation<CommitDutyResult, Error, CommitDutyRequest>({
    mutationFn: (req) => commitDuty(req),
    onSuccess: (result) => {
      const overview =
        queryClient.getQueryData<FetchWorkerOverviewResult>(overviewKey);
      if (overview) {
        const filtered = result.duty
          ? overview.overview.duties.filter((d) => d.id !== result.duty.id)
          : overview.overview.duties;
        queryClient.setQueryData<FetchWorkerOverviewResult>(overviewKey, {
          ...overview,
          overview: {
            ...overview.overview,
            duties: [...filtered, result.duty],
          },
        });
      }
      const recent = queryClient.getQueryData<ListRecentRetrainsResult>(recentKey) ?? {
        records: [],
        source: "mock" as const,
      };
      queryClient.setQueryData<ListRecentRetrainsResult>(recentKey, {
        ...recent,
        records: [dutyAddedRecord(workerId, result), ...recent.records],
      });
    },
  });
}

export interface DutyAddActions {
  classify: (req: Omit<ClassifyDutyRequest, "workerId">) => Promise<ClassifyDutyResult>;
  preview: (req: Omit<PreviewDutyRequest, "workerId">) => Promise<PreviewDutyResult>;
  commit: (req: Omit<CommitDutyRequest, "workerId">) => Promise<CommitDutyResult>;
  isClassifying: boolean;
  isCommitting: boolean;
}

/**
 * Convenience composite — bundles the three duty mutations behind a
 * single hook call. Mirrors the `useRetrainActions` pattern.
 */
export function useDutyAdd(workerId: string): DutyAddActions {
  const classify = useClassifyDuty();
  const preview = usePreviewDuty();
  const commit = useCommitDuty(workerId);

  const doClassify = useCallback(
    (req: Omit<ClassifyDutyRequest, "workerId">) =>
      classify.mutateAsync({ ...req, workerId }),
    [classify, workerId],
  );
  const doPreview = useCallback(
    (req: Omit<PreviewDutyRequest, "workerId">) =>
      preview.mutateAsync({ ...req, workerId }),
    [preview, workerId],
  );
  const doCommit = useCallback(
    (req: Omit<CommitDutyRequest, "workerId">) =>
      commit.mutateAsync({ ...req, workerId }),
    [commit, workerId],
  );

  return {
    classify: doClassify,
    preview: doPreview,
    commit: doCommit,
    isClassifying: classify.isPending,
    isCommitting: commit.isPending,
  };
}
