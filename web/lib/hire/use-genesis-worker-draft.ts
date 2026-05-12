"use client";

import { useMutation } from "@tanstack/react-query";
import { useCallback } from "react";
import {
  draftWorker,
  type DraftWorkerResult,
  type DraftWorkerRequest,
} from "@/lib/api/genesis";

/**
 * Step 3 "Drafting" worker-draft source.
 *
 * Thin wrapper around `useMutation` that fires
 * `POST /api/v1/genesis/draft-worker`. Retry semantics are managed by
 * the caller (Step3Drafting tracks a retry count + 30s timeout), so
 * this hook is intentionally retry-less.
 *
 * The hook returns the mutation's full surface (data / error /
 * isPending / mutate) plus a typed `start()` helper that the Step 3
 * component fires on mount.
 *
 * On unmount the caller should call `reset()` to invalidate any
 * in-flight result so a slow server response can't write into a
 * draft state the user has already navigated away from.
 */
export interface UseGenesisWorkerDraftResult {
  start: (req: DraftWorkerRequest) => void;
  reset: () => void;
  data: DraftWorkerResult | undefined;
  error: Error | null;
  isPending: boolean;
  isSuccess: boolean;
  isError: boolean;
}

export function useGenesisWorkerDraft(): UseGenesisWorkerDraftResult {
  const mutation = useMutation<DraftWorkerResult, Error, DraftWorkerRequest>({
    mutationFn: draftWorker,
    retry: false,
  });

  const start = useCallback(
    (req: DraftWorkerRequest) => {
      mutation.mutate(req);
    },
    [mutation],
  );

  const reset = useCallback(() => {
    mutation.reset();
  }, [mutation]);

  return {
    start,
    reset,
    data: mutation.data,
    error: mutation.error,
    isPending: mutation.isPending,
    isSuccess: mutation.isSuccess,
    isError: mutation.isError,
  };
}
