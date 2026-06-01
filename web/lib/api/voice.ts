/**
 * Conversational-voice API client for the Pitch Agent.
 *
 * Wraps `POST /api/v1/voice/pitch-turn` — the gateway route that runs one
 * full spoken turn: speech-to-text (whisper-1) → the **same**
 * truth-bounded `/api/v1/pitch/ask` answer → text-to-speech (tts-1). See
 * `crates/argo-cli/src/handlers/gateway/routes_voice_io.rs`.
 *
 * The truth boundary lives entirely server-side: a spoken over-claim
 * question (e.g. "what's your valuation?") defers exactly as the typed
 * pitch does — this client never inspects or rewrites the answer.
 *
 * Wire-shape mirror of `pitch_turn_json` in the Rust handler. Any change to
 * that JSON envelope must land here in the same PR.
 */

import { apiPost } from "@/lib/api/client";
import { ApiError } from "@/lib/api/types";
import type { PitchChannel, PitchDisposition } from "@/lib/api/pitch";

/** Truth-boundary block echoed from the reused pitch handler. */
export interface VoiceTruthBoundary {
  /** `Allow` | `Escalate` | `Block` — or null if the handler omitted it. */
  disposition: PitchDisposition | null;
  /** `canonical` | `llm` | `llm_blocked_canonical_refused` | null. */
  source: string | null;
  /** Why a Block fired, when one did. */
  block_reason: string | null;
  /** Echoed pitch channel. */
  channel: string | null;
}

/** Decoded response of one spoken pitch turn. */
export interface PitchTurnResponse {
  /** What Portia heard — the whisper-1 transcript of the question. */
  transcript: string;
  /** The truth-bounded answer text (same gate as the typed pitch). */
  answer_text: string;
  /** Base64-encoded mp3 of the spoken answer. */
  audio_base64: string;
  /** MIME of the spoken audio (`audio/mpeg`). */
  format: string;
  /** STT confidence, when the provider reports one. */
  confidence?: number;
  /** Truth-boundary disposition + source, when present. */
  truth_boundary?: VoiceTruthBoundary;
}

export interface PitchTurnRequest {
  /** Base64-encoded audio of the spoken question (no data-URL prefix). */
  audioBase64: string;
  /** Provider voice id for the answer; defaults to Portia server-side. */
  voiceId?: string;
  /** Pitch channel context; defaults to `Rehearsal` server-side. */
  context?: PitchChannel;
}

function isPitchTurnResponse(raw: unknown): raw is PitchTurnResponse {
  if (!raw || typeof raw !== "object") return false;
  const v = raw as Record<string, unknown>;
  return (
    typeof v.transcript === "string" &&
    typeof v.answer_text === "string" &&
    typeof v.audio_base64 === "string" &&
    v.audio_base64.length > 0 &&
    typeof v.format === "string"
  );
}

/**
 * Run one spoken pitch turn. POSTs the recorded audio to the gateway and
 * returns the heard transcript, the truth-bounded answer text, and the
 * base64 mp3 of Portia's spoken reply.
 *
 * Throws `ApiError` on any non-2xx (e.g. a 503 when the voice provider is
 * unavailable — never faked audio), a malformed payload, or a network
 * failure, so the caller can surface a single error banner.
 */
export async function pitchTurn(req: PitchTurnRequest): Promise<PitchTurnResponse> {
  const payload: Record<string, unknown> = { audio_base64: req.audioBase64 };
  if (req.voiceId !== undefined) payload.voice_id = req.voiceId;
  if (req.context !== undefined) payload.context = req.context;
  const raw = await apiPost<unknown>("/api/v1/voice/pitch-turn", payload);
  if (!isPitchTurnResponse(raw)) {
    throw new ApiError(
      502,
      "Voice endpoint returned an unexpected payload",
      "INVALID_VOICE_RESPONSE",
    );
  }
  return raw;
}

/**
 * Convert a recorded audio `Blob` to a bare base64 string (no `data:`
 * prefix), the shape the gateway's `audio_base64` field expects.
 *
 * Uses `FileReader` so it works in the browser; the `data:<mime>;base64,`
 * prefix the reader prepends is stripped. Exposed (and unit-tested) so the
 * /pitch mic flow and tests share one encoder.
 */
export function blobToBase64(blob: Blob): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onerror = () => reject(reader.error ?? new Error("FileReader failed"));
    reader.onloadend = () => {
      const result = reader.result;
      if (typeof result !== "string") {
        reject(new Error("FileReader did not return a string"));
        return;
      }
      const comma = result.indexOf(",");
      resolve(comma >= 0 ? result.slice(comma + 1) : result);
    };
    reader.readAsDataURL(blob);
  });
}

/** Decoded response of the live-voice (WebRTC) session mint. */
export interface RealtimeSession {
  /** Short-lived OpenAI Realtime ephemeral secret for the WebRTC handshake. */
  clientSecret: string;
  /** Realtime model the session is bound to (e.g. `gpt-realtime-2`). */
  model: string;
  /** Voice the session speaks in. */
  voice: string;
}

/**
 * Mint a live-voice (WebRTC) session. POSTs to the gateway, which creates a
 * short-lived OpenAI Realtime ephemeral secret server-side — the real
 * `OPENAI_API_KEY` never reaches the browser — carrying Portia's
 * truth-bounded `instructions`. The browser then opens a WebRTC peer
 * connection straight to OpenAI with the returned secret.
 *
 * Throws `ApiError` on a non-2xx (e.g. a 503 when `OPENAI_API_KEY` is absent —
 * never a faked token) or a malformed payload.
 */
export async function createRealtimeSession(
  context: PitchChannel = "Rehearsal",
): Promise<RealtimeSession> {
  const raw = await apiPost<unknown>("/api/v1/voice/realtime-session", { context });
  if (!raw || typeof raw !== "object") {
    throw new ApiError(
      502,
      "Live-voice session returned an unexpected payload",
      "INVALID_REALTIME_SESSION",
    );
  }
  const v = raw as Record<string, unknown>;
  const clientSecret = typeof v.client_secret === "string" ? v.client_secret : "";
  if (!clientSecret) {
    throw new ApiError(502, "Live-voice session missing client_secret", "INVALID_REALTIME_SESSION");
  }
  return {
    clientSecret,
    model: typeof v.model === "string" ? v.model : "gpt-realtime-2",
    voice: typeof v.voice === "string" ? v.voice : "marin",
  };
}
