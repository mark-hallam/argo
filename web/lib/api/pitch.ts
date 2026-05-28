/**
 * Pitch Agent API client.
 *
 * Calls `POST /api/v1/pitch/ask` — the deterministic canonical-Q&A
 * lookup + truth-boundary evaluator the gateway wraps around the
 * existing `argo-skill::canonical_qa` matcher and
 * `argo-capability::evaluate_claim`. There is NO LLM hop on this
 * path; the response shape carries the matcher's verdict plus the
 * truth-boundary disposition so the SPA can render the metadata
 * badge alongside the answer.
 *
 * Wire-shape mirror of `crates/argo-cli/src/handlers/gateway/
 * routes_pitch_ask.rs`. Any change to the Rust response struct must
 * land here in the same PR.
 */

import { apiPost } from "@/lib/api/client";
import { ApiError } from "@/lib/api/types";

export type PitchChannel = "Rehearsal" | "WebEmbed" | "LiveMeeting" | "Chat";
export type PitchDisposition = "Allow" | "Escalate" | "Block";

export interface PitchAskRequest {
  question: string;
  channel?: PitchChannel;
  audience?: string;
}

export interface PitchAskResponse {
  matched: boolean;
  entry_id?: string;
  answer: string;
  declared_claim_classes?: string[];
  disposition: PitchDisposition;
  block_reason?: string | null;
  channel: PitchChannel;
  latency_ms: number;
}

const DISPOSITIONS: ReadonlySet<PitchDisposition> = new Set([
  "Allow",
  "Escalate",
  "Block",
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
