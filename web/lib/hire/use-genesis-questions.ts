"use client";

import { useQuery } from "@tanstack/react-query";
import { ApiError } from "@/lib/api/types";
import { classifyRole, type ClassifyResult } from "@/lib/api/genesis";
import { STUB_QUESTIONS } from "./genesis-stub";
import type { Question } from "./genesis-types";

/**
 * Step 2 "Listen" question source.
 *
 * Wraps `classifyRole` in a TanStack Query with bounded retry (3
 * attempts at 1s / 2s / 4s backoff per the brief). After the 3rd
 * retry exhausts, the hook surfaces `status="error"` so the page can
 * swap in the skip-to-template panel. 404 (gateway not yet wired) is
 * NOT retried — it falls through to the stub on the first call.
 */
export interface UseGenesisQuestionsResult {
  /** Final question set: gateway response, stub, or `[]` while loading. */
  questions: Question[];
  /** Coarse-grained status for UI rendering. */
  status: "loading" | "ready" | "error" | "fallback";
  /** Where the questions came from once we settle. */
  source?: "gateway" | "stub";
  /** Final error after all retries (only set when status === "error"). */
  error: Error | null;
  /** Imperative re-run; clears the cache and re-fires the classifier. */
  refetch: () => void;
  /** Total fetch attempts made so far (1 = the initial). Useful for telemetry. */
  attemptCount: number;
}

const MAX_RETRIES = 3;
const BACKOFF_MS = [1_000, 2_000, 4_000] as const;

/** Decides whether a failure is retryable. 4xx is not; ApiError 5xx + network are. */
function shouldRetry(err: unknown): boolean {
  if (err instanceof ApiError) {
    if (err.status === 404) return false;
    return err.status >= 500;
  }
  return err instanceof TypeError;
}

export function useGenesisQuestions(
  problem: string | undefined,
): UseGenesisQuestionsResult {
  const enabled = typeof problem === "string" && problem.trim().length > 0;

  const query = useQuery<ClassifyResult, Error>({
    queryKey: ["genesis", "classify", problem ?? ""] as const,
    enabled,
    staleTime: 5 * 60_000,
    retry: (failureCount, err) => {
      if (!shouldRetry(err)) return false;
      return failureCount < MAX_RETRIES;
    },
    retryDelay: (failureCount) => BACKOFF_MS[Math.min(failureCount, BACKOFF_MS.length - 1)] ?? 4_000,
    queryFn: () => classifyRole({ problem: problem ?? "" }),
  });

  if (query.data) {
    return {
      questions: query.data.questions,
      status: query.data.source === "stub" ? "fallback" : "ready",
      source: query.data.source,
      error: null,
      refetch: () => void query.refetch(),
      attemptCount: query.failureCount + 1,
    };
  }

  if (query.isError) {
    return {
      questions: [...STUB_QUESTIONS],
      status: "error",
      source: undefined,
      error: query.error,
      refetch: () => void query.refetch(),
      attemptCount: query.failureCount,
    };
  }

  return {
    questions: [],
    status: "loading",
    source: undefined,
    error: null,
    refetch: () => void query.refetch(),
    attemptCount: query.failureCount,
  };
}
