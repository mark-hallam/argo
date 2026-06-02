"use client";

/**
 * `useRealtimeVoice` — live, two-way voice with Portia over WebRTC.
 *
 * The streaming counterpart to `useVoiceTurn` (tap-to-talk). The gateway
 * mints a short-lived OpenAI Realtime ephemeral secret (with Portia's
 * truth-boundary baked into the session instructions); this hook opens a
 * WebRTC peer connection straight to OpenAI with that secret, streams the
 * mic up and Portia's audio back, and surfaces the live transcripts.
 *
 * IMPORTANT — truth boundary in this mode is **LLM-primary**: there is no
 * server-side `evaluate_claim` post-check on a token-streamed answer (you
 * can't check a sentence before it's spoken). The model is *instructed* to
 * defer on forbidden classes. So the UI must label live answers honestly
 * (not the green "Allowed" pill that implies the deterministic check ran).
 *
 * Live audio + mic capture can only be exercised in a real browser with mic
 * permission; the WebRTC touchpoints are kept thin so the surface is easy to
 * reason about.
 */

import { useCallback, useEffect, useRef, useState } from "react";
import { ApiError } from "@/lib/api/types";
import type { PitchChannel } from "@/lib/api/pitch";
import { createRealtimeSession, type RealtimeMode } from "@/lib/api/voice";

/** Lifecycle phase of a live-voice session. */
export type RealtimePhase = "idle" | "connecting" | "live" | "error";

/** One completed transcript line in the live conversation. */
export interface RealtimeTranscript {
  role: "user" | "agent";
  text: string;
}

export interface UseRealtimeVoiceOptions {
  /** Pitch channel context forwarded to the mint endpoint. Default `Rehearsal`. */
  context?: PitchChannel;
  /** Which persona the live session adopts (Portia pitch, or Cora coach/interview). Default `pitch`. */
  mode?: RealtimeMode;
}

export interface UseRealtimeVoiceResult {
  phase: RealtimePhase;
  error: string | null;
  /** True once the WebRTC connection is live. */
  isLive: boolean;
  /** Completed transcript lines, oldest first. */
  transcripts: RealtimeTranscript[];
  /** Portia's in-progress spoken text (streaming), for a live caption. */
  liveCaption: string;
  /** Open the live session (mic + WebRTC). No-op unless idle/error. */
  start: () => Promise<void>;
  /** Tear down the live session and return to idle. */
  stop: () => void;
}

/** OpenAI's WebRTC SDP-exchange endpoint. The ephemeral secret authorizes it. */
const REALTIME_CALLS_URL = "https://api.openai.com/v1/realtime/calls";

/** SSR-safe check that the browser can do live voice (mic + WebRTC). */
export function realtimeVoiceSupported(): boolean {
  return (
    typeof window !== "undefined" &&
    typeof navigator !== "undefined" &&
    !!navigator.mediaDevices &&
    typeof navigator.mediaDevices.getUserMedia === "function" &&
    typeof window.RTCPeerConnection !== "undefined"
  );
}

export function useRealtimeVoice(
  options: UseRealtimeVoiceOptions = {},
): UseRealtimeVoiceResult {
  const { context = "Rehearsal", mode = "pitch" } = options;
  const [phase, setPhase] = useState<RealtimePhase>("idle");
  const [error, setError] = useState<string | null>(null);
  const [transcripts, setTranscripts] = useState<RealtimeTranscript[]>([]);
  const [liveCaption, setLiveCaption] = useState("");

  const pcRef = useRef<RTCPeerConnection | null>(null);
  const streamRef = useRef<MediaStream | null>(null);
  const audioRef = useRef<HTMLAudioElement | null>(null);
  const dcRef = useRef<RTCDataChannel | null>(null);
  const mountedRef = useRef(true);
  const agentBufRef = useRef("");

  const teardown = useCallback(() => {
    if (dcRef.current) {
      dcRef.current.close();
      dcRef.current = null;
    }
    if (pcRef.current) {
      pcRef.current.getSenders().forEach((s) => s.track?.stop());
      pcRef.current.close();
      pcRef.current = null;
    }
    if (streamRef.current) {
      streamRef.current.getTracks().forEach((t) => t.stop());
      streamRef.current = null;
    }
    if (audioRef.current) {
      audioRef.current.pause();
      audioRef.current.srcObject = null;
      audioRef.current = null;
    }
    agentBufRef.current = "";
  }, []);

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      teardown();
    };
  }, [teardown]);

  // Decode a single Realtime server event off the data channel. We care about
  // the user-speech transcript and Portia's spoken-text deltas/done; other
  // events (VAD, audio buffers) are ignored — the audio plays via the track.
  const handleEvent = useCallback((evt: Record<string, unknown>) => {
    const type = typeof evt.type === "string" ? evt.type : "";
    const transcript = typeof evt.transcript === "string" ? evt.transcript : undefined;
    const delta = typeof evt.delta === "string" ? evt.delta : undefined;

    if (type === "conversation.item.input_audio_transcription.completed" && transcript) {
      const text = transcript.trim();
      if (text && mountedRef.current) {
        setTranscripts((prev) => [...prev, { role: "user", text }]);
      }
      return;
    }
    if (
      type === "response.output_audio_transcript.delta" ||
      type === "response.audio_transcript.delta"
    ) {
      if (delta) {
        agentBufRef.current += delta;
        if (mountedRef.current) setLiveCaption(agentBufRef.current);
      }
      return;
    }
    if (
      type === "response.output_audio_transcript.done" ||
      type === "response.audio_transcript.done"
    ) {
      const text = (transcript ?? agentBufRef.current).trim();
      agentBufRef.current = "";
      if (mountedRef.current) {
        setLiveCaption("");
        if (text) setTranscripts((prev) => [...prev, { role: "agent", text }]);
      }
    }
  }, []);

  const start = useCallback(async () => {
    if (phase === "connecting" || phase === "live") return;
    setError(null);
    setTranscripts([]);
    setLiveCaption("");
    if (!realtimeVoiceSupported()) {
      setError("This browser can't do live voice (needs microphone + WebRTC).");
      setPhase("error");
      return;
    }
    setPhase("connecting");
    try {
      // Capture the mic FIRST — synchronously after the click — so the user
      // activation is still live when Chrome shows the permission prompt.
      // Awaiting the session mint first consumes the click gesture, after
      // which Chrome suppresses the getUserMedia prompt and rejects with
      // NotAllowedError (no popup). That was why /pitch "Go Live" silently
      // "denied" while a direct mic request (e.g. mictests) prompts fine.
      const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
      if (!mountedRef.current) {
        stream.getTracks().forEach((t) => t.stop());
        return;
      }
      streamRef.current = stream;

      const session = await createRealtimeSession(context, mode);

      const pc = new RTCPeerConnection();
      pcRef.current = pc;

      const audio = document.createElement("audio");
      audio.autoplay = true;
      audioRef.current = audio;
      pc.ontrack = (e) => {
        if (audioRef.current && e.streams[0]) audioRef.current.srcObject = e.streams[0];
      };

      stream.getTracks().forEach((t) => pc.addTrack(t, stream));

      const dc = pc.createDataChannel("oai-events");
      dcRef.current = dc;
      dc.addEventListener("message", (e: MessageEvent) => {
        try {
          handleEvent(JSON.parse(e.data as string) as Record<string, unknown>);
        } catch {
          /* ignore non-JSON frames */
        }
      });
      dc.addEventListener("open", () => {
        // Ask the session to transcribe the user's speech so we can show what
        // was asked. Best-effort — the audio path works regardless.
        try {
          dc.send(
            JSON.stringify({
              type: "session.update",
              session: { audio: { input: { transcription: { model: "whisper-1" } } } },
            }),
          );
        } catch {
          /* best-effort */
        }
      });

      const offer = await pc.createOffer();
      await pc.setLocalDescription(offer);

      const sdpResp = await fetch(REALTIME_CALLS_URL, {
        method: "POST",
        body: offer.sdp ?? "",
        headers: {
          Authorization: `Bearer ${session.clientSecret}`,
          "Content-Type": "application/sdp",
        },
      });
      if (!sdpResp.ok) {
        const detail = await sdpResp.text().catch(() => "");
        throw new Error(
          `Live-voice handshake failed (${sdpResp.status}). ${detail.slice(0, 180)}`,
        );
      }
      const answerSdp = await sdpResp.text();
      await pc.setRemoteDescription({ type: "answer", sdp: answerSdp });

      if (!mountedRef.current) {
        teardown();
        return;
      }
      setPhase("live");
    } catch (err) {
      teardown();
      if (!mountedRef.current) return;
      setError(errMsg(err));
      setPhase("error");
    }
  }, [context, mode, handleEvent, phase, teardown]);

  const stop = useCallback(() => {
    teardown();
    if (mountedRef.current) {
      setPhase("idle");
      setLiveCaption("");
    }
  }, [teardown]);

  return {
    phase,
    error,
    isLive: phase === "live",
    transcripts,
    liveCaption,
    start,
    stop,
  };
}

function errMsg(err: unknown): string {
  if (err instanceof ApiError && err.message) return err.message;
  if (err instanceof DOMException && err.name === "NotAllowedError") {
    return "Microphone permission denied. Allow mic access and try again.";
  }
  if (err instanceof Error && err.message) return err.message;
  return "Couldn't start live voice.";
}
