"use client";

import { useMutation, type UseMutationResult } from "@tanstack/react-query";
import { useCallback } from "react";
import {
  hireWorker,
  type HireWorkerRequest,
  type HireWorkerResult,
} from "@/lib/api/workers";

/**
 * Step 4 "Hire commit" mutation hook.
 *
 * Thin wrapper around `useMutation` that fires the hire endpoint and
 * exposes the full mutation surface (data / error / isPending / mutate)
 * plus a typed `commit()` helper. Optimistic step advancement is the
 * caller's responsibility — Step 4 patches draft state to
 * `{ step: 5, workerId: <draft.workerId> }` in `onMutate` and rolls
 * back to `step: 4` on `onError`. Keeping the hook retry-less + side-
 * effect-free here lets the caller stack its `onMutate` /
 * `onError` handlers without re-implementing them in the hook.
 *
 * On unmount the caller should call `reset()` if a stale response
 * could otherwise write to a draft state the user has navigated away
 * from.
 */
export type UseHireWorkerOptions = Parameters<
  typeof useMutation<HireWorkerResult, Error, HireWorkerRequest>
>[0];

export interface UseHireWorkerResult {
  commit: (req: HireWorkerRequest) => void;
  reset: () => void;
  data: HireWorkerResult | undefined;
  error: Error | null;
  isPending: boolean;
  isSuccess: boolean;
  isError: boolean;
  mutation: UseMutationResult<HireWorkerResult, Error, HireWorkerRequest>;
}

export function useHireWorker(
  options?: Omit<UseHireWorkerOptions, "mutationFn" | "retry">,
): UseHireWorkerResult {
  const mutation = useMutation<HireWorkerResult, Error, HireWorkerRequest>({
    ...options,
    mutationFn: hireWorker,
    retry: false,
  });

  const commit = useCallback(
    (req: HireWorkerRequest) => {
      mutation.mutate(req);
    },
    [mutation],
  );

  const reset = useCallback(() => {
    mutation.reset();
  }, [mutation]);

  return {
    commit,
    reset,
    data: mutation.data,
    error: mutation.error,
    isPending: mutation.isPending,
    isSuccess: mutation.isSuccess,
    isError: mutation.isError,
    mutation,
  };
}
