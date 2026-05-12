"use client";

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
