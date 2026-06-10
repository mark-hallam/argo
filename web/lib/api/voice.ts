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

import { apiGet, apiPost } from "@/lib/api/client";
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

/** Decoded response of one speech-to-text clip transcription. */
export interface SttResponse {
  /** The transcribed words for the posted audio clip. */
  transcript: string;
  /** STT confidence, when the provider reports one. */
  confidence?: number;
}

/**
 * Transcribe one recorded audio clip via `POST /api/v1/voice/stt` — the
 * gateway's vendor-neutral speech-to-text route (base64 audio in →
 * transcript out). Used by the `/genesis` mic-dictation flow to turn a
 * spoken role description into editable text.
 *
 * This is dictation only: the route is clip-request/response (real provider
 * in prod, `MockStt`-backed offline), NOT a streaming socket and NOT the
 * truth-bounded pitch/conversation path — it never runs an LLM answer.
 *
 * Goes through `apiPost` so tenant headers + cookie auth are inherited; the
 * server-side tenant scoping is pinned by `routes_voice_io_tests.rs`. Throws
 * `ApiError` on any non-2xx (e.g. a 503 when the STT provider is
 * unavailable — never a faked transcript) or a malformed payload.
 */
export async function sttTranscribe(audioBase64: string): Promise<SttResponse> {
  const raw = await apiPost<unknown>("/api/v1/voice/stt", {
    audio_base64: audioBase64,
  });
  const v = raw && typeof raw === "object" ? (raw as Record<string, unknown>) : null;
  if (!v || typeof v.transcript !== "string") {
    throw new ApiError(
      502,
      "STT endpoint returned an unexpected payload",
      "INVALID_STT_RESPONSE",
    );
  }
  return {
    transcript: v.transcript,
    ...(typeof v.confidence === "number" ? { confidence: v.confidence } : {}),
  };
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

/** Persona the live session adopts — Portia's pitch, or Cora's coach / founder-story interview voice. */
export type RealtimeMode = "pitch" | "coach" | "interview";

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
  mode: RealtimeMode = "pitch",
): Promise<RealtimeSession> {
  const body: Record<string, unknown> = { context };
  // The gateway defaults to Portia's pitch persona when `mode` is absent; only
  // send it for Cora's coach / interview voices so the default path is unchanged.
  if (mode !== "pitch") body.mode = mode;
  const raw = await apiPost<unknown>("/api/v1/voice/realtime-session", body);
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

/** One spoken turn within a past live-voice session. */
export interface VoiceSessionTurn {
  /** `"user"` (you) or `"agent"` (the teammate). */
  role: "user" | "agent";
  /** The transcribed words. */
  text: string;
  /** Epoch milliseconds when the turn completed. */
  ts: number;
}

/**
 * A past live-voice session: every spoken turn (both sides) grouped under one
 * `session_id`, read back from the hash-chained audit log, newest first.
 * Wire-shape mirror of `group_sessions` in `routes_voice_realtime.rs` — any
 * change to that JSON envelope must land here in the same change.
 */
export interface VoiceSession {
  /** Client-generated id that ties one call's turns together. */
  session_id: string;
  /** `"pitch"` | `"coach"` | `"interview"` (or `""` when unset). */
  mode: string;
  /** Epoch ms of the first turn. */
  started_at: number;
  /** Epoch ms of the last turn. */
  ended_at: number;
  /** How many turns the session holds. */
  turn_count: number;
  /** First ~120 chars of the opening turn, for the collapsed row. */
  preview: string;
  /** Every turn, oldest first. */
  turns: VoiceSessionTurn[];
}

/**
 * Persist one spoken turn as a hash-chained, tamper-evident audit row
 * (`POST /api/v1/voice/transcript`). **Best-effort by design** — a failed
 * write (network, CSRF, 503) is swallowed so it can never interrupt a live
 * call. The audit moat applied to conversation: every word, logged + provable.
 *
 * Empty/whitespace text is dropped client-side (the gateway also rejects it).
 */
export async function postVoiceTranscriptTurn(turn: {
  sessionId: string;
  role: "user" | "agent";
  text: string;
  mode?: RealtimeMode;
  tsMillis?: number;
}): Promise<void> {
  const text = turn.text.trim();
  if (!text) return;
  const payload: Record<string, unknown> = {
    session_id: turn.sessionId,
    role: turn.role,
    text,
    ts_millis: turn.tsMillis ?? Date.now(),
  };
  if (turn.mode !== undefined) payload.mode = turn.mode;
  try {
    await apiPost("/api/v1/voice/transcript", payload);
  } catch (err) {
    // Best-effort: log for debugging but never propagate — the live call wins.
    console.warn("voice transcript turn not persisted (best-effort)", err);
  }
}

/**
 * List past live-voice sessions (newest first), optionally filtered to one
 * `mode`. Reads `GET /api/v1/voice/sessions`, which groups the tenant's
 * `voice.transcript_turn` audit rows by `session_id`. Returns `[]` on any
 * malformed payload so the history panel degrades to empty rather than throws.
 */
export async function listVoiceSessions(mode?: RealtimeMode): Promise<VoiceSession[]> {
  const path = mode
    ? `/api/v1/voice/sessions?mode=${encodeURIComponent(mode)}`
    : "/api/v1/voice/sessions";
  const raw = await apiGet<unknown>(path);
  if (!raw || typeof raw !== "object") return [];
  const sessions = (raw as Record<string, unknown>).sessions;
  return Array.isArray(sessions) ? (sessions as VoiceSession[]) : [];
}
