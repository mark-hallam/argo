/**
 * Replay sandbox API client. POSTs to
 * `/api/v1/audit/<actionId>/replay` and returns a discriminated
 * `ReplayOutcome`.
 *
 * Mock outcomes are returned ONLY for action ids in the canonical
 * sample-replay TABLE, AND ONLY when `isDemoMode()` is true OR the
 * gateway is auth-bypassed. Any other action id — including unknown
 * 32-char-hex ids that the gateway 404s on — returns a
 * `{ kind: "failed" }` envelope so the page renders `<FailedView>`
 * with a real error instead of a synthesised forensic report. This
 * closes the Scene-6 demo-gate (an unknown id used to fabricate a
 * "Policy regression detected" banner derived from the URL).
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
import { findSampleReplay } from "@/lib/sample-replay";
import { isDemoMode } from "@/lib/skycot/demo-mode";

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
    process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS === "1"
  );
}

function isMissingOrUnreachable(err: unknown): boolean {
  if (err instanceof ApiError) return err.status === 404;
  return err instanceof TypeError;
}

const UNKNOWN_ACTION_FAILED: ReplayOutcome = {
  kind: "failed",
  reason:
    "Action not found — the replay sandbox only covers the seeded demo personas.",
};

/**
 * Returns a canonical sample-replay outcome for `actionId` IF demo
 * mode or auth-bypass is enabled AND the id is in the hand-crafted
 * TABLE. Otherwise returns a real `failed` envelope so the page
 * renders `<FailedView>` instead of fabricating content from the URL.
 */
function sampleOrFailed(actionId: string): ReplayOutcome {
  if (!isDemoMode() && !isAuthBypassed()) return UNKNOWN_ACTION_FAILED;
  return findSampleReplay(actionId) ?? UNKNOWN_ACTION_FAILED;
}

/**
 * Kick off a replay. Returns the outcome envelope; never throws on a
 * 404 (falls through to a `failed` envelope, or to a canonical sample
 * when one exists in demo / auth-bypass mode). Surfaces network /
 * 5xx errors to the caller, which the mutation hook maps to the
 * `{ kind: "failed" }` UI state via its `onError` handler.
 */
export async function replayAction(actionId: string): Promise<ReplayOutcome> {
  if (!actionId) {
    return { kind: "failed", reason: "Missing action id" };
  }

  if (isAuthBypassed()) {
    return sampleOrFailed(actionId);
  }

  const path = `/api/v1/audit/${encodeURIComponent(actionId)}/replay`;
  try {
    return await apiPost<ReplayOutcome>(path);
  } catch (err) {
    if (!isStrictProduction() && isMissingOrUnreachable(err)) {
      return sampleOrFailed(actionId);
    }
    throw err;
  }
}

