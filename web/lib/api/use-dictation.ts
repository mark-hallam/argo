/**
 * `useDictation` — near-live mic dictation for the `/genesis` role textarea.
 *
 * Records mic audio and, every few seconds, flushes a COMPLETE, standalone
 * audio clip to the gateway's `POST /api/v1/voice/stt` (via `sttTranscribe`),
 * emitting each clip's transcript through `onChunk` so the caller can append
 * it to a textarea. Pressing stop flushes the final clip.
 *
 * Why stop/restart per clip, not a single timesliced recorder: `MediaRecorder`
 * timeslice fragments after the first are NOT independently decodable (only the
 * first fragment carries the container header), so an STT provider can't
 * transcribe them in isolation. Cutting a fresh, self-contained clip each
 * interval — while keeping the same mic `MediaStream` open the whole time, so
 * the gap between clips is only a few ms of JS — gives valid clips that append
 * cleanly. This is deliberately the deterministic clip-request/response STT
 * route, not the OpenAI Realtime token-stream (that path is /pitch + /coach's
 * conversation surface, with no server-side claim post-check — out of scope
 * for dictation).
 *
 * NB: real mic capture can only be fully exercised in a live browser with
 * microphone permission. This hook is structured so the MediaRecorder /
 * getUserMedia touchpoints are mockable, and the vitest suite drives them with
 * fakes; the live-browser confirmation is a separate manual step. Mirrors the
 * shape of `use-voice-turn.ts`.
 */

import { useCallback, useEffect, useRef, useState } from "react";
import { ApiError } from "@/lib/api/types";
import { blobToBase64, sttTranscribe } from "@/lib/api/voice";

/** Phase of the dictation lifecycle. */
export type DictationState = "idle" | "recording" | "denied" | "unavailable";

/** Default interval between clip flushes (ms). Each flush is one STT round-trip. */
const DEFAULT_INTERVAL_MS = 4_000;

export interface UseDictationOptions {
  /**
   * Called once per successfully-transcribed clip with the trimmed,
   * non-empty transcript text. The caller appends it to its own state.
   */
  onChunk: (text: string) => void;
  /** Milliseconds between clip flushes. Defaults to {@link DEFAULT_INTERVAL_MS}. */
  intervalMs?: number;
}

export interface UseDictationResult {
  /** Current lifecycle state. */
  state: DictationState;
  /** True while recording — convenience for toggling the mic button. */
  isRecording: boolean;
  /** Last error message (mic denied / no device / unsupported), or null. */
  error: string | null;
  /** Begin capturing mic audio. No-op unless idle. */
  start: () => Promise<void>;
  /** Stop capturing; flushes the final clip. */
  stop: () => void;
  /** Convenience: start if idle, else stop. */
  toggle: () => void;
}

/** Whether the browser exposes the APIs this hook needs. SSR-safe. */
export function dictationSupported(): boolean {
  return (
    typeof window !== "undefined" &&
    typeof navigator !== "undefined" &&
    !!navigator.mediaDevices &&
    typeof navigator.mediaDevices.getUserMedia === "function" &&
    typeof window.MediaRecorder !== "undefined"
  );
}

function deniedMessage(err: unknown): string {
  if (err instanceof DOMException && err.name === "NotAllowedError") {
    return "Mic unavailable — type instead.";
  }
  if (err instanceof DOMException && err.name === "NotFoundError") {
    return "Mic unavailable — type instead.";
  }
  return "Mic unavailable — type instead.";
}

export function useDictation(options: UseDictationOptions): UseDictationResult {
  const { onChunk, intervalMs = DEFAULT_INTERVAL_MS } = options;
  const [state, setState] = useState<DictationState>("idle");
  const [error, setError] = useState<string | null>(null);

  const streamRef = useRef<MediaStream | null>(null);
  const recorderRef = useRef<MediaRecorder | null>(null);
  const chunksRef = useRef<Blob[]>([]);
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  // True between start() and stop(): drives the clip-cycle loop.
  const activeRef = useRef(false);
  // Latest onChunk, so a re-render with a new closure is always honoured.
  const onChunkRef = useRef(onChunk);
  onChunkRef.current = onChunk;
  // Guards against a late callback flipping state after unmount.
  const mountedRef = useRef(true);

  const clearTimer = useCallback(() => {
    if (timerRef.current !== null) {
      clearTimeout(timerRef.current);
      timerRef.current = null;
    }
  }, []);

  /** Transcribe one finished clip and emit its text. Failures are skipped. */
  const transcribeClip = useCallback(async (blob: Blob) => {
    if (blob.size === 0) return;
    try {
      const audioBase64 = await blobToBase64(blob);
      const { transcript } = await sttTranscribe(audioBase64);
      const text = transcript.trim();
      if (text.length > 0) onChunkRef.current(text);
    } catch (err) {
      // A failed chunk is non-blocking: prior chunks stay, recording continues.
      console.warn("dictation chunk transcription skipped", err);
    }
  }, []);

  // `beginCycle` records one standalone clip, then (if still active) starts the
  // next — declared via a ref so `onstop` can re-enter it without a TS cycle.
  const beginCycleRef = useRef<() => void>(() => {});

  const beginCycle = useCallback(() => {
    const stream = streamRef.current;
    if (!stream || !activeRef.current) return;
    let recorder: MediaRecorder;
    try {
      recorder = new MediaRecorder(stream);
    } catch (err) {
      activeRef.current = false;
      stopTracks(streamRef.current);
      streamRef.current = null;
      if (mountedRef.current) {
        setError(deniedMessage(err));
        setState("idle");
      }
      return;
    }
    chunksRef.current = [];
    recorder.ondataavailable = (e: BlobEvent) => {
      if (e.data && e.data.size > 0) chunksRef.current.push(e.data);
    };
    recorder.onstop = () => {
      const type = chunksRef.current[0]?.type || recorder.mimeType || "audio/webm";
      const blob = new Blob(chunksRef.current, { type });
      chunksRef.current = [];
      void transcribeClip(blob);
      // Same mic stream stays open; cut the next clip immediately.
      if (activeRef.current) beginCycleRef.current();
    };
    recorderRef.current = recorder;
    recorder.start();
    clearTimer();
    timerRef.current = setTimeout(() => {
      const r = recorderRef.current;
      if (r && r.state !== "inactive") r.stop();
    }, intervalMs);
  }, [intervalMs, clearTimer, transcribeClip]);
  beginCycleRef.current = beginCycle;

  const start = useCallback(async () => {
    if (state !== "idle") return;
    setError(null);
    if (!dictationSupported()) {
      setState("unavailable");
      setError("Mic unavailable — type instead.");
      return;
    }
    try {
      const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
      streamRef.current = stream;
      activeRef.current = true;
      setState("recording");
      beginCycleRef.current();
    } catch (err) {
      stopTracks(streamRef.current);
      streamRef.current = null;
      activeRef.current = false;
      if (mountedRef.current) {
        setState("denied");
        setError(deniedMessage(err));
      }
    }
  }, [state]);

  const stop = useCallback(() => {
    activeRef.current = false;
    clearTimer();
    const recorder = recorderRef.current;
    if (recorder && recorder.state !== "inactive") {
      // Final clip flushes through onstop; the cycle won't re-arm (active=false).
      recorder.stop();
    }
    recorderRef.current = null;
    stopTracks(streamRef.current);
    streamRef.current = null;
    if (mountedRef.current) setState("idle");
  }, [clearTimer]);

  const toggle = useCallback(() => {
    if (state === "recording") {
      stop();
    } else {
      void start();
    }
  }, [state, start, stop]);

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      activeRef.current = false;
      clearTimer();
      const recorder = recorderRef.current;
      if (recorder && recorder.state !== "inactive") recorder.stop();
      recorderRef.current = null;
      stopTracks(streamRef.current);
      streamRef.current = null;
    };
  }, [clearTimer]);

  return {
    state,
    isRecording: state === "recording",
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
