"use client";

import { useCallback } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  getWorkerPolicies,
  updateWorkerPolicy,
  type ApprovalPolicy,
  type UpdatePolicyResult,
  type WorkerPoliciesResult,
} from "@/lib/api/worker-policies";

/**
 * Per-worker approval-policy hooks.
 *
 * `useWorkerPolicies(workerId)` reads the full list (one query
 * key). `useUpdateWorkerPolicy(workerId)` returns a mutation
 * handle that optimistically updates the cached row and refetches
 * the policies query on success so the source-of-truth
 * `inferred_trust_grade` from the server stays canonical.
 */

export const workerPoliciesQueryKey = (workerId: string) =>
  ["worker", workerId, "policies"] as const;

export function useWorkerPolicies(workerId: string) {
  return useQuery<WorkerPoliciesResult, Error>({
    queryKey: workerPoliciesQueryKey(workerId),
    queryFn: () => getWorkerPolicies(workerId),
    enabled: Boolean(workerId),
  });
}

export interface UpdatePolicyVars {
  actionType: string;
  policy: ApprovalPolicy;
  operatorNote: string;
}

export function useUpdateWorkerPolicy(workerId: string) {
  const client = useQueryClient();
  return useMutation<
    UpdatePolicyResult,
    Error,
    UpdatePolicyVars,
    WorkerPoliciesResult | undefined
  >({
    mutationFn: (vars) =>
      updateWorkerPolicy({
        workerId,
        actionType: vars.actionType,
        policy: vars.policy,
        operatorNote: vars.operatorNote,
      }),
    onMutate: (vars) => {
      const key = workerPoliciesQueryKey(workerId);
      const previous = client.getQueryData<WorkerPoliciesResult>(key);
      if (!previous) return undefined;
      client.setQueryData<WorkerPoliciesResult>(key, {
        ...previous,
        rows: previous.rows.map((row) =>
          row.actionType === vars.actionType
            ? { ...row, currentPolicy: vars.policy, isDefault: false }
            : row,
        ),
      });
      return previous;
    },
    onError: (_e, _v, previous) => {
      if (previous) {
        client.setQueryData(workerPoliciesQueryKey(workerId), previous);
      }
    },
    onSettled: () => {
      void client.invalidateQueries({
        queryKey: workerPoliciesQueryKey(workerId),
      });
    },
  });
}

/** Convenience bundle for the page-level component. */
export function usePolicyActions(workerId: string) {
  const update = useUpdateWorkerPolicy(workerId);
  const onUpdatePolicy = useCallback(
    (vars: UpdatePolicyVars) => update.mutate(vars),
    [update],
  );
  return { onUpdatePolicy, isUpdating: update.isPending };
}
