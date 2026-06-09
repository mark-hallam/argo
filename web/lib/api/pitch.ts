/**
 * Pitch Agent API client.
 *
 * Calls `POST /api/v1/pitch/ask` — the canonical-Q&A lookup +
 * truth-boundary evaluator the gateway wraps around the existing
 * `argo-skill::canonical_qa` matcher and
 * `argo-capability::evaluate_claim`. On a canonical hit (hot path)
 * the response is deterministic and sub-millisecond. On a canonical
 * miss the gateway falls through to an LLM-fallback path (per the
 * `pitch-agent-llm-with-truth-boundary-gating` brief): the LLM
 * drafts an answer using the truth-boundary policy + operator-
 * reviewed company facts as system context, then the draft is
 * re-evaluated against the policy and a canonical refusal is
 * swapped in if Block fires.
 *
 * Wire-shape mirror of `crates/argo-cli/src/handlers/gateway/
 * routes_pitch_ask.rs`. Any change to the Rust response struct must
 * land here in the same PR.
 */

import { apiPost } from "@/lib/api/client";
import { ApiError } from "@/lib/api/types";

export type PitchChannel = "Rehearsal" | "WebEmbed" | "LiveMeeting" | "Chat";
export type PitchDisposition = "Allow" | "Escalate" | "Block";

/**
 * Where the answer was sourced.
 * - `canonical` — hot-path canonical-Q&A match (sub-millisecond, no LLM cost).
 * - `llm` — LLM fallback fired; truth-boundary check Allowed/Escalated.
 * - `llm_blocked_canonical_refused` — LLM fired but truth-boundary
 *   check Blocked the draft and the gateway swapped in a canonical
 *   refusal. The `draft` field carries the LLM's actual unsafe attempt
 *   for the rehearsal surface.
 */
export type PitchSource = "canonical" | "llm" | "llm_blocked_canonical_refused";

export interface PitchAskRequest {
  question: string;
  channel?: PitchChannel;
  audience?: string;
}

/** Provider-reported token counts for the LLM-fallback path. */
export interface PitchTokens {
  input?: number | null;
  output?: number | null;
}

export interface PitchAskResponse {
  matched: boolean;
  /**
   * Where the answer came from. Optional for forward-compat — gateways
   * that predate the LLM-fallback brief omit it; the SPA treats
   * `undefined` as `canonical`.
   */
  source?: PitchSource;
  entry_id?: string;
  answer: string;
  /**
   * On the LLM-blocked path the gateway echoes the LLM's actual
   * (unsafe) draft so the rehearsal surface can show the swap.
   * Not present on canonical or LLM-allow paths.
   */
  draft?: string;
  declared_claim_classes?: string[];
  disposition: PitchDisposition;
  block_reason?: string | null;
  channel: PitchChannel;
  latency_ms: number;
  /** Model id used by the LLM-fallback path. */
  model?: string;
  /** Provider-reported token counts for the LLM-fallback path. */
  tokens?: PitchTokens;
}

const DISPOSITIONS: ReadonlySet<PitchDisposition> = new Set([
  "Allow",
  "Escalate",
  "Block",
]);

const SOURCES: ReadonlySet<PitchSource> = new Set([
  "canonical",
  "llm",
  "llm_blocked_canonical_refused",
]);

function isPitchAskResponse(raw: unknown): raw is PitchAskResponse {
  if (!raw || typeof raw !== "object") return false;
  const v = raw as Record<string, unknown>;
  if (typeof v.matched !== "boolean") return false;
  if (typeof v.answer !== "string") return false;
  if (typeof v.disposition !== "string") return false;
  if (!DISPOSITIONS.has(v.disposition as PitchDisposition)) return false;
  if (typeof v.channel !== "string") return false;
  if (typeof v.latency_ms !== "number") return false;
  // `source` is optional for back-compat; reject only if present + invalid.
  if (v.source !== undefined && (typeof v.source !== "string" || !SOURCES.has(v.source as PitchSource))) {
    return false;
  }
  return true;
}

/**
 * Ask the Pitch Agent one question. Returns the gateway's full
 * response on success. The caller is expected to render the
 * disposition badge (Allow / Escalate / Block) directly so the
 * operator can see when a refusal fired.
 *
 * Throws `ApiError` on any non-2xx, malformed payload, or network
 * failure so the chat surface can surface a single error banner
 * without ambiguity.
 */
export async function askPitch(req: PitchAskRequest): Promise<PitchAskResponse> {
  const payload: PitchAskRequest = {
    question: req.question,
    channel: req.channel ?? "Rehearsal",
  };
  if (req.audience !== undefined) payload.audience = req.audience;
  const raw = await apiPost<unknown>("/api/v1/pitch/ask", payload);
  if (!isPitchAskResponse(raw)) {
    throw new ApiError(
      502,
      "Pitch endpoint returned an unexpected payload",
      "INVALID_PITCH_RESPONSE",
    );
  }
  return raw;
}

/**
 * Ask the PUBLIC, anonymous, rate-limited Pitch Agent one question —
 * `POST /api/v1/pitch/ask/public`. This is the un-authenticated front door
 * behind the landing-page "Talk to Portia" widget: no bearer is required,
 * the gateway forces the strict public truth-boundary channel server-side,
 * and only `question` is sent (channel/audience are NOT client-controlled).
 *
 * Throws `ApiError` on any non-2xx (notably `429` when the per-IP cap
 * fires) or a malformed payload, so the embed can fall back gracefully to
 * its canned offline answer.
 */
export async function askPitchPublic(question: string): Promise<PitchAskResponse> {
  const raw = await apiPost<unknown>("/api/v1/pitch/ask/public", { question });
  if (!isPitchAskResponse(raw)) {
    throw new ApiError(
      502,
      "Public pitch endpoint returned an unexpected payload",
      "INVALID_PITCH_RESPONSE",
    );
  }
  return raw;
}
