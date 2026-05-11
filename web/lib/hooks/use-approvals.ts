"use client";

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  fetchApprovals,
  submitDecision,
  submitUndo,
  type ApprovalDecisionRequest,
} from "@/lib/api/approvals";
import type { ApprovalItem } from "@/lib/sample-approvals";

const APPROVALS_KEY = ["approvals"] as const;

/** The approvals inbox. Server-cached + revalidated on window focus. */
export function useApprovals() {
  return useQuery({
    queryKey: APPROVALS_KEY,
    queryFn: fetchApprovals,
  });
}

/**
 * Submit a decision with optimistic update. The card flips state
 * immediately; on error we roll back. On success we leave the
 * cached state in place — no refetch needed unless TTL expires.
 */
export function useSubmitDecision() {
  const qc = useQueryClient();

  return useMutation({
    mutationFn: submitDecision,
    onMutate: async ({ approvalId, decision }: ApprovalDecisionRequest) => {
      await qc.cancelQueries({ queryKey: APPROVALS_KEY });
      const prev = qc.getQueryData<ApprovalItem[]>(APPROVALS_KEY);
      qc.setQueryData<ApprovalItem[]>(APPROVALS_KEY, (items) =>
        (items ?? []).map((i) => (i.id === approvalId ? { ...i, decision } : i)),
      );
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
      const prev = qc.getQueryData<ApprovalItem[]>(APPROVALS_KEY);
      qc.setQueryData<ApprovalItem[]>(APPROVALS_KEY, (items) =>
        (items ?? []).map((i) => (i.id === approvalId ? { ...i, decision: null } : i)),
      );
      return { prev };
    },
    onError: (_err, _vars, ctx) => {
      if (ctx?.prev) qc.setQueryData(APPROVALS_KEY, ctx.prev);
    },
  });
}
