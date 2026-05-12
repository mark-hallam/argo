"use client";

import { useCallback, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  fetchApprovals,
  submitDecision,
  submitUndo,
  type ApprovalDecisionRequest,
  type ApprovalsResult,
  type DataSource,
} from "@/lib/api/approvals";
import type { ApprovalItem } from "@/lib/sample-approvals";

const APPROVALS_KEY = ["approvals"] as const;

/**
 * The approvals inbox. The cache stores the full `ApprovalsResult`
 * (`{ items, source }`); we splay `data` back into `items` so the
 * page's existing `query.data ?? []` access pattern keeps working,
 * and expose `dataSource` separately for the DataSourceChip.
 *
 * `dataSource` is `undefined` while the first fetch is in flight;
 * the chip should treat that as "no chip yet" rather than guessing
 * a source.
 */
export function useApprovals() {
  const query = useQuery({
    queryKey: APPROVALS_KEY,
    queryFn: fetchApprovals,
  });
  return {
    ...query,
    data: query.data?.items,
    dataSource: query.data?.source as DataSource | undefined,
  };
}

/** Optimistic decision submit. The card flips immediately; on error we roll back. */
export function useSubmitDecision() {
  const qc = useQueryClient();

  return useMutation({
    mutationFn: submitDecision,
    onMutate: async ({ approvalId, decision }: ApprovalDecisionRequest) => {
      await qc.cancelQueries({ queryKey: APPROVALS_KEY });
      const prev = qc.getQueryData<ApprovalsResult>(APPROVALS_KEY);
      if (prev) {
        qc.setQueryData<ApprovalsResult>(APPROVALS_KEY, {
          ...prev,
          items: prev.items.map((i: ApprovalItem) =>
            i.id === approvalId ? { ...i, decision } : i,
          ),
        });
      }
      return { prev };
    },
    onError: (_err, _vars, ctx) => {
      if (ctx?.prev) qc.setQueryData(APPROVALS_KEY, ctx.prev);
    },
  });
}

export type BatchOutcome = {
  approvalId: string;
  ok: boolean;
  error?: Error;
};

export type BatchSubmitState = {
  /** True while at least one parallel mutation is in flight. */
  isPending: boolean;
  /** Total in the in-flight batch (resets on each `submit`). */
  total: number;
  /** Mutations that resolved (success or failure). */
  completed: number;
  /** Mutations that rejected. */
  failed: number;
  /** Per-approval results once the batch settles. */
  outcomes: BatchOutcome[];
};

const IDLE_BATCH: BatchSubmitState = {
  isPending: false,
  total: 0,
  completed: 0,
  failed: 0,
  outcomes: [],
};

/**
 * Batch decision submit. Fires N parallel `submitDecision` POSTs
 * via the same optimistic-update path as `useSubmitDecision` — each
 * card flips independently, each rolls back on its own failure, and
 * the caller gets a single Promise that resolves once all settle.
 *
 * Returns a per-call progress snapshot (`state.completed`,
 * `state.failed`) so the batch bar can render a "Approving 5..."
 * → "Approved 4 — 1 failed" countdown.
 */
export function useBatchSubmitDecisions() {
  const submitOne = useSubmitDecision();
  const [state, setState] = useState<BatchSubmitState>(IDLE_BATCH);

  const submit = useCallback(
    async (requests: ApprovalDecisionRequest[]): Promise<BatchOutcome[]> => {
      if (requests.length === 0) {
        setState(IDLE_BATCH);
        return [];
      }
      let completed = 0;
      let failed = 0;
      setState({
        isPending: true,
        total: requests.length,
        completed: 0,
        failed: 0,
        outcomes: [],
      });
      const settled = await Promise.all(
        requests.map(async (req): Promise<BatchOutcome> => {
          try {
            await submitOne.mutateAsync(req);
            completed += 1;
            setState((s) => ({ ...s, completed }));
            return { approvalId: req.approvalId, ok: true };
          } catch (err) {
            failed += 1;
            setState((s) => ({ ...s, failed }));
            return {
              approvalId: req.approvalId,
              ok: false,
              error: err instanceof Error ? err : new Error(String(err)),
            };
          }
        }),
      );
      setState({
        isPending: false,
        total: requests.length,
        completed,
        failed,
        outcomes: settled,
      });
      return settled;
    },
    [submitOne],
  );

  const reset = useCallback(() => setState(IDLE_BATCH), []);

  return { submit, reset, state };
}

/** Undo a recent decision. Same optimistic-update shape. */
export function useUndoDecision() {
  const qc = useQueryClient();

  return useMutation({
    mutationFn: (approvalId: string) => submitUndo(approvalId),
    onMutate: async (approvalId: string) => {
      await qc.cancelQueries({ queryKey: APPROVALS_KEY });
      const prev = qc.getQueryData<ApprovalsResult>(APPROVALS_KEY);
      if (prev) {
        qc.setQueryData<ApprovalsResult>(APPROVALS_KEY, {
          ...prev,
          items: prev.items.map((i: ApprovalItem) =>
            i.id === approvalId ? { ...i, decision: null } : i,
          ),
        });
      }
      return { prev };
    },
    onError: (_err, _vars, ctx) => {
      if (ctx?.prev) qc.setQueryData(APPROVALS_KEY, ctx.prev);
    },
  });
}
