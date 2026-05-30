/**
 * `useVoiceTurn` — the browser push-to-talk seam for "Talk to Portia".
 *
 * Records mic audio with `MediaRecorder`, posts it to the gateway's
 * `POST /api/v1/voice/pitch-turn` (via `pitchTurn`), then plays Portia's
 * spoken answer back through an `Audio` element and surfaces the heard
 * transcript + answer text + truth-boundary disposition to the caller.
 *
 * The truth boundary is enforced entirely server-side (the pitch-turn
 * route reuses `/api/v1/pitch/ask`), so this hook never inspects or
 * rewrites the answer — it just records, sends, and plays.
 *
 * NB: real mic capture + audio playback can only be fully exercised in a
 * live browser with microphone permission. This hook is structured so the
 * MediaRecorder / getUserMedia / Audio touchpoints are mockable, and the
 * vitest suite drives them with fakes; the live-browser confirmation is a
 * separate manual step.
 */

import { useCallback, useEffect, useRef, useState } from "react";
import { ApiError } from "@/lib/api/types";
import type { PitchChannel } from "@/lib/api/pitch";
import { blobToBase64, pitchTurn, type PitchTurnResponse } from "@/lib/api/voice";

/** Phase of the push-to-talk lifecycle, for driving button affordances. */
export type VoiceTurnPhase = "idle" | "recording" | "thinking" | "speaking";

export interface UseVoiceTurnOptions {
  /** Pitch channel context forwarded to the gateway. Default `WebEmbed`. */
  context?: PitchChannel;
  /** Called with the decoded turn once the gateway answers. */
  onTurn?: (turn: PitchTurnResponse) => void;
}

export interface UseVoiceTurnResult {
  /** Current lifecycle phase. */
  phase: VoiceTurnPhase;
  /** True while recording — convenience for toggling the mic button. */
  isRecording: boolean;
  /** True while waiting on the gateway or playing the answer. */
  isBusy: boolean;
  /** Last error message (mic denied, gateway 503, …) or null. */
  error: string | null;
  /** Begin capturing mic audio. No-op unless idle. */
  start: () => Promise<void>;
  /** Stop capturing; triggers the gateway turn + playback. */
  stop: () => void;
  /** Convenience: start if idle, else stop. */
  toggle: () => void;
}

/** Whether the browser exposes the APIs this hook needs. SSR-safe. */
export function voiceCaptureSupported(): boolean {
  return (
    typeof window !== "undefined" &&
    typeof navigator !== "undefined" &&
    !!navigator.mediaDevices &&
    typeof navigator.mediaDevices.getUserMedia === "function" &&
    typeof window.MediaRecorder !== "undefined"
  );
}

function errorMessage(err: unknown): string {
  if (err instanceof ApiError && err.message) return err.message;
  if (err instanceof DOMException && err.name === "NotAllowedError") {
    return "Microphone permission denied. Allow mic access and try again.";
  }
  if (err instanceof Error && err.message) return err.message;
  return "Couldn't complete the voice turn.";
}

export function useVoiceTurn(options: UseVoiceTurnOptions = {}): UseVoiceTurnResult {
  const { context = "WebEmbed", onTurn } = options;
  const [phase, setPhase] = useState<VoiceTurnPhase>("idle");
  const [error, setError] = useState<string | null>(null);

  const recorderRef = useRef<MediaRecorder | null>(null);
  const chunksRef = useRef<Blob[]>([]);
  const streamRef = useRef<MediaStream | null>(null);
  const audioRef = useRef<HTMLAudioElement | null>(null);
  // Guards against a late callback flipping state after unmount.
  const mountedRef = useRef(true);

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      stopTracks(streamRef.current);
      streamRef.current = null;
      const audio = audioRef.current;
      if (audio) {
        audio.pause();
        audioRef.current = null;
      }
    };
  }, []);

  const playAnswer = useCallback((audioBase64: string, format: string) => {
    const src = `data:${format || "audio/mpeg"};base64,${audioBase64}`;
    const audio = new Audio(src);
    audioRef.current = audio;
    const done = () => {
      if (mountedRef.current) setPhase("idle");
    };
    audio.onended = done;
    audio.onerror = done;
    // Autoplay can reject without a user gesture; the stop() click is the
    // gesture, so this normally resolves. If it rejects we still leave the
    // transcript + answer text on screen and return to idle.
    void audio.play().catch(() => done());
  }, []);

  const runTurn = useCallback(
    async (audio: Blob) => {
      try {
        setPhase("thinking");
        const audioBase64 = await blobToBase64(audio);
        const turn = await pitchTurn({ audioBase64, context });
        if (!mountedRef.current) return;
        onTurn?.(turn);
        setPhase("speaking");
        playAnswer(turn.audio_base64, turn.format);
      } catch (err) {
        if (!mountedRef.current) return;
        setError(errorMessage(err));
        setPhase("idle");
      }
    },
    [context, onTurn, playAnswer],
  );

  const start = useCallback(async () => {
    if (phase !== "idle") return;
    setError(null);
    if (!voiceCaptureSupported()) {
      setError("This browser can't capture microphone audio.");
      return;
    }
    try {
      const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
      streamRef.current = stream;
      const recorder = new MediaRecorder(stream);
      chunksRef.current = [];
      recorder.ondataavailable = (e: BlobEvent) => {
        if (e.data && e.data.size > 0) chunksRef.current.push(e.data);
      };
      recorder.onstop = () => {
        stopTracks(streamRef.current);
        streamRef.current = null;
        const type = chunksRef.current[0]?.type || recorder.mimeType || "audio/webm";
        const blob = new Blob(chunksRef.current, { type });
        chunksRef.current = [];
        if (blob.size === 0) {
          if (mountedRef.current) {
            setError("No audio captured. Hold the mic and speak, then release.");
            setPhase("idle");
          }
          return;
        }
        void runTurn(blob);
      };
      recorderRef.current = recorder;
      recorder.start();
      setPhase("recording");
    } catch (err) {
      stopTracks(streamRef.current);
      streamRef.current = null;
      setError(errorMessage(err));
      setPhase("idle");
    }
  }, [phase, runTurn]);

  const stop = useCallback(() => {
    const recorder = recorderRef.current;
    if (recorder && recorder.state !== "inactive") {
      recorder.stop();
    }
    recorderRef.current = null;
  }, []);

  const toggle = useCallback(() => {
    if (phase === "recording") {
      stop();
    } else if (phase === "idle") {
      void start();
    }
  }, [phase, start, stop]);

  return {
    phase,
    isRecording: phase === "recording",
    isBusy: phase === "thinking" || phase === "speaking",
    error,
    start,
    stop,
    toggle,
  };
}

/** Stop every track on a stream so the OS mic indicator clears. */
function stopTracks(stream: MediaStream | null): void {
  if (!stream) return;
  for (const track of stream.getTracks()) {
    track.stop();
  }
}
