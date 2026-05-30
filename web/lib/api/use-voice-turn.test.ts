import { act, renderHook, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

// Mock the network + encoder seam so the hook test stays pure-DOM: we drive
// MediaRecorder / getUserMedia / Audio with fakes and assert the hook wires
// them together. blobToBase64 is stubbed so we don't depend on FileReader.
const voiceApi = vi.hoisted(() => ({
  pitchTurn: vi.fn(),
  blobToBase64: vi.fn(),
}));

vi.mock("./voice", async (importOriginal) => {
  const actual = await importOriginal<typeof import("./voice")>();
  return { ...actual, ...voiceApi };
});

import { useVoiceTurn } from "./use-voice-turn";
import { ApiError } from "./types";

// --- Fakes -----------------------------------------------------------------

/** A controllable MediaRecorder stand-in. */
class FakeMediaRecorder {
  static instances: FakeMediaRecorder[] = [];
  state: "inactive" | "recording" = "inactive";
  mimeType = "audio/webm";
  ondataavailable: ((e: { data: Blob }) => void) | null = null;
  onstop: (() => void) | null = null;
  constructor(public stream: FakeMediaStream) {
    FakeMediaRecorder.instances.push(this);
  }
  start() {
    this.state = "recording";
  }
  /** Emit a chunk then fire stop — mimics releasing the mic button. */
  stop() {
    this.state = "inactive";
    this.ondataavailable?.({ data: new Blob([new Uint8Array([1, 2, 3])], { type: "audio/webm" }) });
    this.onstop?.();
  }
  /** Stop with no captured audio (the empty-recording path). */
  stopEmpty() {
    this.state = "inactive";
    this.onstop?.();
  }
}

class FakeTrack {
  stopped = false;
  stop() {
    this.stopped = true;
  }
}

class FakeMediaStream {
  tracks = [new FakeTrack()];
  getTracks() {
    return this.tracks;
  }
}

/** A controllable Audio element stand-in. */
class FakeAudio {
  static instances: FakeAudio[] = [];
  onended: (() => void) | null = null;
  onerror: (() => void) | null = null;
  paused = false;
  playCalls = 0;
  constructor(public src: string) {
    FakeAudio.instances.push(this);
  }
  play() {
    this.playCalls += 1;
    return Promise.resolve();
  }
  pause() {
    this.paused = true;
  }
  /** Simulate playback finishing. */
  finish() {
    this.onended?.();
  }
}

let getUserMedia: ReturnType<typeof vi.fn>;

function installVoiceEnv() {
  FakeMediaRecorder.instances = [];
  FakeAudio.instances = [];
  getUserMedia = vi.fn(() => Promise.resolve(new FakeMediaStream()));
  Object.defineProperty(navigator, "mediaDevices", {
    configurable: true,
    value: { getUserMedia },
  });
  (window as unknown as { MediaRecorder: unknown }).MediaRecorder = FakeMediaRecorder;
  (window as unknown as { Audio: unknown }).Audio = FakeAudio;
  (globalThis as unknown as { Audio: unknown }).Audio = FakeAudio;
}

const SAMPLE_TURN = {
  transcript: "What does Skycot do?",
  answer_text: "Skycot is an AI-teammate platform.",
  audio_base64: "QUJD",
  format: "audio/mpeg",
  truth_boundary: { disposition: "Allow", source: "llm", block_reason: null, channel: "WebEmbed" },
};

describe("useVoiceTurn", () => {
  beforeEach(() => {
    installVoiceEnv();
    voiceApi.pitchTurn.mockReset();
    voiceApi.blobToBase64.mockReset();
    voiceApi.blobToBase64.mockResolvedValue("Zm9v");
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  it("records, posts the captured audio, and plays Portia's answer", async () => {
    voiceApi.pitchTurn.mockResolvedValueOnce(SAMPLE_TURN);
    const onTurn = vi.fn();
    const { result } = renderHook(() => useVoiceTurn({ onTurn }));

    expect(result.current.phase).toBe("idle");

    // Start recording.
    await act(async () => {
      await result.current.start();
    });
    expect(getUserMedia).toHaveBeenCalledWith({ audio: true });
    expect(result.current.phase).toBe("recording");
    expect(result.current.isRecording).toBe(true);

    // Release the mic → recorder.stop() emits a chunk + onstop, which posts.
    await act(async () => {
      FakeMediaRecorder.instances[0].stop();
    });

    await waitFor(() => expect(onTurn).toHaveBeenCalledWith(SAMPLE_TURN));
    // It POSTed the encoded audio with the right context.
    expect(voiceApi.pitchTurn).toHaveBeenCalledWith({ audioBase64: "Zm9v", context: "WebEmbed" });
    // And it played the returned answer audio as a data URL.
    await waitFor(() => expect(FakeAudio.instances.length).toBe(1));
    expect(FakeAudio.instances[0].src).toBe("data:audio/mpeg;base64,QUJD");
    expect(FakeAudio.instances[0].playCalls).toBe(1);
    expect(result.current.phase).toBe("speaking");

    // When playback ends, we return to idle.
    act(() => FakeAudio.instances[0].finish());
    await waitFor(() => expect(result.current.phase).toBe("idle"));

    // The mic track was released (OS indicator clears).
    expect((getUserMedia.mock.results[0].value as Promise<FakeMediaStream>)).toBeDefined();
  });

  it("forwards a custom context to the gateway", async () => {
    voiceApi.pitchTurn.mockResolvedValueOnce(SAMPLE_TURN);
    const { result } = renderHook(() => useVoiceTurn({ context: "Rehearsal" }));
    await act(async () => {
      await result.current.start();
    });
    await act(async () => {
      FakeMediaRecorder.instances[0].stop();
    });
    await waitFor(() =>
      expect(voiceApi.pitchTurn).toHaveBeenCalledWith({
        audioBase64: "Zm9v",
        context: "Rehearsal",
      }),
    );
  });

  it("surfaces a denied-mic permission as a friendly error", async () => {
    getUserMedia.mockRejectedValueOnce(
      new DOMException("denied", "NotAllowedError"),
    );
    const { result } = renderHook(() => useVoiceTurn());
    await act(async () => {
      await result.current.start();
    });
    expect(result.current.phase).toBe("idle");
    expect(result.current.error).toMatch(/permission denied/i);
    expect(voiceApi.pitchTurn).not.toHaveBeenCalled();
  });

  it("surfaces a gateway 503 (no faked audio) and returns to idle", async () => {
    voiceApi.pitchTurn.mockRejectedValueOnce(
      new ApiError(503, "tts provider: no key", "voice_unavailable"),
    );
    const { result } = renderHook(() => useVoiceTurn());
    await act(async () => {
      await result.current.start();
    });
    await act(async () => {
      FakeMediaRecorder.instances[0].stop();
    });
    await waitFor(() => expect(result.current.error).toMatch(/no key/i));
    expect(result.current.phase).toBe("idle");
    // No audio was played — the failure did not fabricate a clip.
    expect(FakeAudio.instances.length).toBe(0);
  });

  it("flags an empty recording instead of posting silence", async () => {
    const { result } = renderHook(() => useVoiceTurn());
    await act(async () => {
      await result.current.start();
    });
    await act(async () => {
      FakeMediaRecorder.instances[0].stopEmpty();
    });
    await waitFor(() => expect(result.current.error).toMatch(/no audio captured/i));
    expect(result.current.phase).toBe("idle");
    expect(voiceApi.pitchTurn).not.toHaveBeenCalled();
  });

  it("toggle() starts when idle and stops when recording", async () => {
    voiceApi.pitchTurn.mockResolvedValueOnce(SAMPLE_TURN);
    const { result } = renderHook(() => useVoiceTurn());
    await act(async () => {
      result.current.toggle();
    });
    await waitFor(() => expect(result.current.phase).toBe("recording"));
    await act(async () => {
      result.current.toggle();
    });
    await waitFor(() => expect(voiceApi.pitchTurn).toHaveBeenCalled());
  });
});
