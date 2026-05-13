"use client";

import { useMutation, type UseMutationResult } from "@tanstack/react-query";
import { replayAction, type ReplayOutcome } from "@/lib/api/replay";

/** Mutation key used by tests and the page; namespaced by actionId. */
export const replayMutationKey = (actionId: string) =>
  ["audit-replay", actionId] as const;

/**
 * Drives the `/audit/action/<id>/replay` route. The mutation is fired
 * on mount by the page; transient progress is owned by the page
 * itself (the gateway streaming-progress contract lands with the
 * sibling backend brief).
 *
 * Network / 5xx errors map to a synthetic `{ kind: "failed" }`
 * outcome at the page level via `data ?? { kind: "failed", ... }`;
 * the hook surface itself stays a TanStack mutation so retry / status
 * semantics match the rest of the app.
 */
export function useReplay(
  actionId: string,
): UseMutationResult<ReplayOutcome, Error, void> {
  return useMutation<ReplayOutcome, Error, void>({
    mutationKey: replayMutationKey(actionId),
    mutationFn: () => replayAction(actionId),
    retry: false,
  });
}
