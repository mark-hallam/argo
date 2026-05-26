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
  | { kind: "unavailable"; reason: "unrecognized-envelope"; details: string }
  | { kind: "failed"; reason: string }
  | { kind: "leak-detected"; details: string };

type GatewayReplayBody = {
  audit_id?: unknown;
  seq?: unknown;
  kind?: unknown;
  verdict?: unknown;
  expected_self_hash?: unknown;
  actual_self_hash?: unknown;
  diff?: unknown;
};

type ProvenanceAxis = keyof ReplayResult["provenanceDiff"];

const PROVENANCE_AXES: ProvenanceAxis[] = [
  "model",
  "promptTemplate",
  "skill",
  "memorySnapshot",
  "policy",
];

const UNCHANGED_PROVENANCE: ReplayResult["provenanceDiff"] = {
  model: { changed: false },
  promptTemplate: { changed: false },
  skill: { changed: false },
  memorySnapshot: { changed: false },
  policy: { changed: false },
};

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

function isObject(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function isReplayOutcome(raw: unknown): raw is ReplayOutcome {
  if (!isObject(raw) || typeof raw.kind !== "string") return false;
  if (raw.kind === "success") {
    return (
      typeof raw.verdict === "string" &&
      ["reproducible", "drifted-aligned", "drifted-misaligned"].includes(raw.verdict) &&
      isObject(raw.result)
    );
  }
  if (raw.kind === "unavailable") {
    if (raw.reason === "model-decommissioned") {
      return typeof raw.modelName === "string";
    }
    if (raw.reason === "unrecognized-envelope") {
      return typeof raw.details === "string";
    }
    return false;
  }
  if (raw.kind === "failed") return typeof raw.reason === "string";
  if (raw.kind === "leak-detected") return typeof raw.details === "string";
  return false;
}

function isGatewayReplayBody(raw: unknown): raw is GatewayReplayBody {
  return (
    isObject(raw) &&
    typeof raw.verdict === "string" &&
    "expected_self_hash" in raw &&
    "actual_self_hash" in raw
  );
}

function asDisplayString(value: unknown, fallback: string): string {
  return typeof value === "string" && value.trim() ? value : fallback;
}

function diffRecord(diff: unknown): Record<string, unknown> {
  return isObject(diff) ? diff : {};
}

function provenanceDiffFromGatewayDiff(
  diff: unknown,
): ReplayResult["provenanceDiff"] {
  const record = diffRecord(diff);
  const next: ReplayResult["provenanceDiff"] = {
    model: { changed: false },
    promptTemplate: { changed: false },
    skill: { changed: false },
    memorySnapshot: { changed: false },
    policy: { changed: false },
  };
  for (const axis of PROVENANCE_AXES) {
    const entry = record[axis];
    if (!entry) continue;
    if (isObject(entry)) {
      next[axis] = {
        changed: true,
        from: typeof entry.from === "string" ? entry.from : undefined,
        to: typeof entry.to === "string" ? entry.to : undefined,
      };
    } else {
      next[axis] = { changed: true };
    }
  }
  return next;
}

function gatewayDiffTouchesPolicy(diff: unknown): boolean {
  const record = diffRecord(diff);
  return "policy" in record || "policy_id" in record || "policy_hash" in record;
}

function hashBody(label: string, raw: GatewayReplayBody): string {
  const hashKey = label === "Original" ? raw.expected_self_hash : raw.actual_self_hash;
  const selfHash = asDisplayString(hashKey, "unavailable");
  const auditId = asDisplayString(raw.audit_id, "unknown");
  const seq = typeof raw.seq === "number" ? String(raw.seq) : "unknown";
  const eventKind = asDisplayString(raw.kind, "unknown");
  return [
    `${label} audit self_hash: ${selfHash}`,
    `Audit id: ${auditId}`,
    `Sequence: ${seq}`,
    `Event kind: ${eventKind}`,
  ].join("\n");
}

function resultFromGatewayHash(
  raw: GatewayReplayBody,
  provenanceDiff: ReplayResult["provenanceDiff"],
): ReplayResult {
  return {
    originalOutput: { kind: "text", body: hashBody("Original", raw) },
    replayedOutput: { kind: "text", body: hashBody("Replayed", raw) },
    provenanceDiff,
  };
}

function unavailableReplay(reason: string): ReplayOutcome {
  return {
    kind: "unavailable",
    reason: "unrecognized-envelope",
    details: reason,
  };
}

// TEMPORARY: gateway returns hash-recompute envelope. Once gateway
// ships the SPA-shaped response per fix-gateway-replay-envelope-spa-contract,
// delete this adapter and trust the canonical ReplayOutcome contract directly.
export function adaptGatewayReplay(raw: unknown): ReplayOutcome {
  if (isReplayOutcome(raw)) return raw;
  if (!isGatewayReplayBody(raw)) {
    return unavailableReplay("Replay returned an unrecognized envelope.");
  }
  if (raw.verdict === "same") {
    return {
      kind: "success",
      verdict: "reproducible",
      result: resultFromGatewayHash(raw, UNCHANGED_PROVENANCE),
    };
  }
  if (raw.verdict === "drifted") {
    const provenanceDiff = provenanceDiffFromGatewayDiff(raw.diff);
    return {
      kind: "success",
      verdict: gatewayDiffTouchesPolicy(raw.diff) ? "drifted-misaligned" : "drifted-aligned",
      result: resultFromGatewayHash(raw, provenanceDiff),
    };
  }
  return unavailableReplay(`Replay returned an unknown verdict: ${String(raw.verdict)}.`);
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
    return adaptGatewayReplay(await apiPost<unknown>(path));
  } catch (err) {
    if (!isStrictProduction() && isMissingOrUnreachable(err)) {
      return sampleOrFailed(actionId);
    }
    throw err;
  }
}
