/**
 * Replay sandbox API client. POSTs to
 * `/api/v1/audit/<actionId>/replay` and returns a discriminated
 * `ReplayOutcome`. In dev (auth-bypass or gateway 404) returns a
 * deterministic mock outcome derived from the action id so the
 * `/audit/action/<id>/replay` route is usable without a backend.
 *
 * The discriminated union is the CONTRACT shape locked by the E5-d
 * brief — server-side replay execution is owned by the sibling
 * `claude-audit-replay` brief and must return the same envelope.
 *
 * Side effects: the client never executes anything. Replay is
 * server-side; this module just renders the verdict.
 */
import { apiPost } from "@/lib/api/client";
import { ApiError } from "@/lib/api/types";
import { findSampleReplay, mockOutcomeForActionId } from "@/lib/sample-replay";

export type Verdict = "reproducible" | "drifted-aligned" | "drifted-misaligned";

export interface ReplayResult {
  originalOutput: { kind: "text" | "json" | "computer-use"; body: unknown };
  replayedOutput: { kind: "text" | "json" | "computer-use"; body: unknown };
  provenanceDiff: {
    model: { changed: boolean; from?: string; to?: string };
    promptTemplate: { changed: boolean; from?: string; to?: string };
    skill: { changed: boolean; from?: string; to?: string };
    memorySnapshot: { changed: boolean };
    policy: { changed: boolean; from?: string; to?: string };
  };
}

export type ReplayOutcome =
  | { kind: "success"; verdict: Verdict; result: ReplayResult }
  | { kind: "unavailable"; reason: "model-decommissioned"; modelName: string }
  | { kind: "failed"; reason: string }
  | { kind: "leak-detected"; details: string };

function isStrictProduction(): boolean {
  return (
    process.env.NODE_ENV === "production" &&
    process.env.NEXT_PUBLIC_ENV === "production"
  );
}

function isAuthBypassed(): boolean {
  return (
    process.env.NODE_ENV !== "production" &&
    process.env.NEXT_PUBLIC_GATEWAY_AUTH_BYPASS === "1"
  );
}

function isMissingOrUnreachable(err: unknown): boolean {
  if (err instanceof ApiError) return err.status === 404;
  return err instanceof TypeError;
}

function mockOutcome(actionId: string): ReplayOutcome {
  const explicit = findSampleReplay(actionId);
  if (explicit) return explicit;
  return mockOutcomeForActionId(actionId);
}

/**
 * Kick off a replay. Returns the outcome envelope; never throws on a
 * 404 in dev (falls through to the mock). Surfaces network / 5xx
 * errors to the caller, which the mutation hook maps to the
 * `{ kind: "failed" }` UI state via its `onError` handler.
 */
export async function replayAction(actionId: string): Promise<ReplayOutcome> {
  if (!actionId) {
    return { kind: "failed", reason: "Missing action id" };
  }

  if (isAuthBypassed()) {
    return mockOutcome(actionId);
  }

  const path = `/api/v1/audit/${encodeURIComponent(actionId)}/replay`;
  try {
    return await apiPost<ReplayOutcome>(path);
  } catch (err) {
    if (!isStrictProduction() && isMissingOrUnreachable(err)) {
      return mockOutcome(actionId);
    }
    throw err;
  }
}

/**
 * Deterministic verdict picker — used by the mock fallback and the
 * page tests. Hashes the action id to one of the three verdicts so
 * the same id always returns the same result.
 */
export function mockVerdict(actionId: string): Verdict {
  if (actionId.length === 0) return "reproducible";
  const h = actionId.charCodeAt(actionId.length - 1) % 3;
  return (["reproducible", "drifted-aligned", "drifted-misaligned"] as const)[h];
}
