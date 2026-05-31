"use client";

import {
  AlertTriangle,
  LoaderCircle,
  Mic,
  Square,
  Volume2,
} from "lucide-react";
import {
  forwardRef,
  useCallback,
  useEffect,
  useImperativeHandle,
  useMemo,
  useRef,
  useState,
} from "react";
import { cn } from "@/lib/utils";

export type VoiceChatState = "idle" | "recording" | "transcribing" | "speaking";

export type VoiceChatAudioSource = Blob | string;

export type VoiceChatOnAudioResult =
  | VoiceChatAudioSource
  | { audio?: VoiceChatAudioSource | null }
  | null
  | undefined
  | void;

export interface VoiceChatHandle {
  /** Play a backend TTS response and drive the UI through `speaking`. */
  playResponse: (audio: VoiceChatAudioSource) => Promise<void>;
  /** Stop the current recording, if one is active. */
  stopRecording: () => void;
  /** Clear errors and return to idle. */
  reset: () => void;
  /** Current talk-state snapshot. */
  state: VoiceChatState;
}

export interface VoiceChatProps {
  /** Called with the recorded mic blob. The consumer owns the backend POST. */
  onAudio: (blob: Blob) => Promise<VoiceChatOnAudioResult> | VoiceChatOnAudioResult;
  /** Optional className for console-specific placement. */
  className?: string;
  /** Disable the mic affordance while an outer surface is busy. */
  disabled?: boolean;
  /** Heading text next to the mic button. */
  label?: string;
  /** Status labels for console-specific copy. */
  stateLabels?: Partial<Record<VoiceChatState, string>>;
  /** Called whenever the component changes talk-state. */
  onStateChange?: (state: VoiceChatState) => void;
  /** Called with user-visible errors after the component records them. */
  onError?: (message: string) => void;
}

const DEFAULT_STATE_LABELS: Record<VoiceChatState, string> = {
  idle: "Tap to talk",
  recording: "Recording",
  transcribing: "Transcribing",
  speaking: "Speaking",
};

const STATE_HELP: Record<VoiceChatState, string> = {
  idle: "Ready for a spoken turn.",
  recording: "Listening through your microphone.",
  transcribing: "Audio captured. Waiting for the teammate response.",
  speaking: "Playing the teammate response.",
};

/** SSR-safe capability check for browser mic capture. */
export function voiceChatSupported(): boolean {
  return (
    typeof window !== "undefined" &&
    typeof navigator !== "undefined" &&
    !!navigator.mediaDevices &&
    typeof navigator.mediaDevices.getUserMedia === "function" &&
    typeof window.MediaRecorder !== "undefined"
  );
}

export const VoiceChat = forwardRef<VoiceChatHandle, VoiceChatProps>(
  (
    {
      onAudio,
      className,
      disabled = false,
      label = "Voice",
      stateLabels,
      onStateChange,
      onError,
    },
    ref,
  ) => {
    const labels = useMemo(
      () => ({ ...DEFAULT_STATE_LABELS, ...stateLabels }),
      [stateLabels],
    );
    const [state, setState] = useState<VoiceChatState>("idle");
    const [error, setError] = useState<string | null>(null);
    const recorderRef = useRef<MediaRecorder | null>(null);
    const streamRef = useRef<MediaStream | null>(null);
    const chunksRef = useRef<Blob[]>([]);
    const audioRef = useRef<HTMLAudioElement | null>(null);
    const mountedRef = useRef(true);

    useEffect(() => {
      mountedRef.current = true;
      return () => {
        mountedRef.current = false;
        stopRecordingInternal(recorderRef.current);
        recorderRef.current = null;
        stopTracks(streamRef.current);
        streamRef.current = null;
        stopAudio(audioRef.current);
        audioRef.current = null;
      };
    }, []);

    useEffect(() => {
      onStateChange?.(state);
    }, [onStateChange, state]);

    const reportError = useCallback(
      (message: string) => {
        if (!mountedRef.current) return;
        setError(message);
        onError?.(message);
      },
      [onError],
    );

    const safeSetState = useCallback((next: VoiceChatState) => {
      if (mountedRef.current) setState(next);
    }, []);

    const playResponse = useCallback(
      async (source: VoiceChatAudioSource) => {
        if (!mountedRef.current) return;
        setError(null);
        safeSetState("speaking");
        stopAudio(audioRef.current);
        try {
          await playAudioSource(source, (audio) => {
            audioRef.current = audio;
          });
          safeSetState("idle");
        } catch (err) {
          reportError(errorMessage(err, "Couldn't play the teammate response."));
          safeSetState("idle");
          throw err;
        }
      },
      [reportError, safeSetState],
    );

    const submitAudio = useCallback(
      async (blob: Blob) => {
        safeSetState("transcribing");
        try {
          const result = await onAudio(blob);
          const audio = extractAudioSource(result);
          if (audio) {
            await playResponse(audio);
          } else {
            safeSetState("idle");
          }
        } catch (err) {
          reportError(errorMessage(err, "Couldn't complete the voice turn."));
          safeSetState("idle");
        }
      },
      [onAudio, playResponse, reportError, safeSetState],
    );

    const startRecording = useCallback(async () => {
      if (disabled || state !== "idle") return;
      setError(null);
      if (!voiceChatSupported()) {
        reportError("This browser can't capture microphone audio.");
        return;
      }
      try {
        const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
        if (!mountedRef.current) {
          stopTracks(stream);
          return;
        }
        streamRef.current = stream;
        const recorder = new window.MediaRecorder(stream);
        recorderRef.current = recorder;
        chunksRef.current = [];
        recorder.ondataavailable = (event: BlobEvent) => {
          if (event.data && event.data.size > 0) chunksRef.current.push(event.data);
        };
        recorder.onerror = () => {
          stopTracks(streamRef.current);
          streamRef.current = null;
          recorderRef.current = null;
          reportError("Microphone recording failed. Try again.");
          safeSetState("idle");
        };
        recorder.onstop = () => {
          stopTracks(streamRef.current);
          streamRef.current = null;
          recorderRef.current = null;
          if (!mountedRef.current) return;
          const type = chunksRef.current[0]?.type || recorder.mimeType || "audio/webm";
          const blob = new Blob(chunksRef.current, { type });
          chunksRef.current = [];
          if (blob.size === 0) {
            reportError("No audio captured. Hold the mic, speak, then stop.");
            safeSetState("idle");
            return;
          }
          void submitAudio(blob);
        };
        recorder.start();
        safeSetState("recording");
      } catch (err) {
        stopTracks(streamRef.current);
        streamRef.current = null;
        recorderRef.current = null;
        reportError(errorMessage(err, "Couldn't access your microphone."));
        safeSetState("idle");
      }
    }, [disabled, reportError, safeSetState, state, submitAudio]);

    const stopRecording = useCallback(() => {
      stopRecordingInternal(recorderRef.current);
    }, []);

    const reset = useCallback(() => {
      setError(null);
      safeSetState("idle");
    }, [safeSetState]);

    useImperativeHandle(
      ref,
      () => ({
        playResponse,
        stopRecording,
        reset,
        state,
      }),
      [playResponse, reset, state, stopRecording],
    );

    const busy = state === "transcribing" || state === "speaking";
    const recording = state === "recording";
    const buttonDisabled = disabled || busy;
    const buttonLabel = recording ? "Stop recording" : labels[state];
    const Icon = recording
      ? Square
      : state === "transcribing"
        ? LoaderCircle
        : state === "speaking"
          ? Volume2
          : Mic;

    return (
      <section
        data-testid="voice-chat"
        data-state={state}
        aria-busy={busy || undefined}
        className={cn(
          "flex w-full max-w-md items-center gap-3 rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--card)] p-3 text-[var(--card-foreground)]",
          "shadow-sm",
          className,
        )}
      >
        <button
          type="button"
          aria-label={buttonLabel}
          aria-pressed={recording}
          disabled={buttonDisabled}
          onClick={recording ? stopRecording : startRecording}
          className={cn(
            "grid size-12 shrink-0 place-items-center rounded-full text-[var(--primary-foreground)] transition duration-[var(--duration-base)] ease-[var(--motion-ease)]",
            "bg-[var(--primary)] shadow-lg shadow-[var(--primary)]/20",
            "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--ring)] focus-visible:ring-offset-2 focus-visible:ring-offset-[var(--background)]",
            "disabled:cursor-not-allowed disabled:opacity-55",
            recording && "animate-pulse bg-[var(--destructive)] text-[var(--destructive-foreground)]",
            state === "speaking" && "bg-[image:var(--brand-gradient)]",
          )}
        >
          <Icon
            aria-hidden
            className={cn("size-5", state === "transcribing" && "animate-spin")}
          />
        </button>

        <div className="min-w-0 flex-1">
          <div className="flex flex-wrap items-center gap-2">
            <span className="truncate text-sm font-semibold text-[var(--foreground)]">
              {label}
            </span>
            <span
              data-testid="voice-chat-state"
              className="rounded-[var(--radius-sm)] border border-[var(--border)] bg-[var(--muted)] px-2 py-0.5 text-[11px] font-medium uppercase tracking-normal text-[var(--muted-foreground)]"
            >
              {labels[state]}
            </span>
          </div>
          <p
            data-testid="voice-chat-help"
            aria-live="polite"
            className="mt-1 text-xs leading-5 text-[var(--muted-foreground)]"
          >
            {STATE_HELP[state]}
          </p>
          {error ? (
            <p
              role="alert"
              data-testid="voice-chat-error"
              className="mt-2 flex items-start gap-1.5 text-xs leading-5 text-[var(--destructive)]"
            >
              <AlertTriangle aria-hidden className="mt-0.5 size-3.5 shrink-0" />
              <span>{error}</span>
            </p>
          ) : null}
        </div>
      </section>
    );
  },
);
VoiceChat.displayName = "VoiceChat";

async function playAudioSource(
  source: VoiceChatAudioSource,
  bindAudio: (audio: HTMLAudioElement) => void,
): Promise<void> {
  const src = isBlob(source) ? URL.createObjectURL(source) : source;
  const objectUrl = isBlob(source) ? src : null;
  const audio = new Audio(src);
  bindAudio(audio);
  await new Promise<void>((resolve, reject) => {
    let settled = false;
    const settle = (cb: () => void) => {
      if (settled) return;
      settled = true;
      audio.onended = null;
      audio.onerror = null;
      if (objectUrl) URL.revokeObjectURL(objectUrl);
      cb();
    };
    audio.onended = () => settle(resolve);
    audio.onerror = () => settle(() => reject(new Error("Audio playback failed.")));
    void audio.play().catch((err: unknown) => settle(() => reject(err)));
  });
}

function extractAudioSource(result: VoiceChatOnAudioResult): VoiceChatAudioSource | null {
  if (!result) return null;
  if (typeof result === "string" || isBlob(result)) return result;
  return result.audio ?? null;
}

function isBlob(value: unknown): value is Blob {
  return typeof Blob !== "undefined" && value instanceof Blob;
}

function stopTracks(stream: MediaStream | null): void {
  if (!stream) return;
  for (const track of stream.getTracks()) {
    track.stop();
  }
}

function stopRecordingInternal(recorder: MediaRecorder | null): void {
  if (recorder && recorder.state !== "inactive") recorder.stop();
}

function stopAudio(audio: HTMLAudioElement | null): void {
  if (!audio) return;
  audio.pause();
  audio.src = "";
}

function errorMessage(err: unknown, fallback: string): string {
  if (err instanceof DOMException && err.name === "NotAllowedError") {
    return "Microphone permission denied. Allow mic access and try again.";
  }
  if (err instanceof Error && err.message) return err.message;
  return fallback;
}
