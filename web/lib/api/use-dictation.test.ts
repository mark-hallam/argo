import { act, renderHook, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

// Mock the network + encoder seam so the hook test stays pure-DOM: we drive
// MediaRecorder / getUserMedia with fakes and assert the hook wires them to the
// STT route. blobToBase64 is stubbed so we don't depend on FileReader.
const voiceApi = vi.hoisted(() => ({
  sttTranscribe: vi.fn(),
  blobToBase64: vi.fn(),
}));

vi.mock("./voice", async (importOriginal) => {
  const actual = await importOriginal<typeof import("./voice")>();
  return { ...actual, ...voiceApi };
});

import { useDictation } from "./use-dictation";
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
  /** Emit a chunk then fire stop — mimics one clip flush. */
  stop() {
    this.state = "inactive";
    this.ondataavailable?.({ data: new Blob([new Uint8Array([1, 2, 3])], { type: "audio/webm" }) });
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

let getUserMedia: ReturnType<typeof vi.fn>;

function installVoiceEnv() {
  FakeMediaRecorder.instances = [];
  getUserMedia = vi.fn(() => Promise.resolve(new FakeMediaStream()));
  Object.defineProperty(navigator, "mediaDevices", {
    configurable: true,
    value: { getUserMedia },
  });
  (window as unknown as { MediaRecorder: unknown }).MediaRecorder = FakeMediaRecorder;
}

describe("useDictation", () => {
  beforeEach(() => {
    installVoiceEnv();
    voiceApi.sttTranscribe.mockReset();
    voiceApi.blobToBase64.mockReset();
    voiceApi.blobToBase64.mockResolvedValue("Zm9v");
    voiceApi.sttTranscribe.mockResolvedValue({ transcript: "tier one tickets" });
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  it("starts recording and flushes a clip to /voice/stt, appending each transcript", async () => {
    const onChunk = vi.fn();
    const { result } = renderHook(() => useDictation({ onChunk }));

    expect(result.current.state).toBe("idle");

    await act(async () => {
      await result.current.start();
    });
    expect(getUserMedia).toHaveBeenCalledWith({ audio: true });
    expect(result.current.state).toBe("recording");
    expect(result.current.isRecording).toBe(true);

    // One clip flush (interval boundary): recorder.stop() → STT → onChunk.
    await act(async () => {
      FakeMediaRecorder.instances[0].stop();
    });

    await waitFor(() => expect(onChunk).toHaveBeenCalledWith("tier one tickets"));
    expect(voiceApi.sttTranscribe).toHaveBeenCalledWith("Zm9v");
    // Still recording: a fresh standalone clip recorder was cut for the next span.
    expect(result.current.state).toBe("recording");
    expect(FakeMediaRecorder.instances.length).toBe(2);
  });

  it("stop() flushes the final clip and returns to idle", async () => {
    const onChunk = vi.fn();
    const { result } = renderHook(() => useDictation({ onChunk }));
    await act(async () => {
      await result.current.start();
    });
    await act(async () => {
      result.current.stop();
    });
    await waitFor(() => expect(onChunk).toHaveBeenCalledWith("tier one tickets"));
    expect(result.current.state).toBe("idle");
    expect(result.current.isRecording).toBe(false);
    // The mic track was released (OS indicator clears).
    expect(FakeMediaRecorder.instances[0].stream.tracks[0].stopped).toBe(true);
  });

  it("surfaces a denied-mic permission as the type-instead fallback", async () => {
    getUserMedia.mockRejectedValueOnce(new DOMException("denied", "NotAllowedError"));
    const onChunk = vi.fn();
    const { result } = renderHook(() => useDictation({ onChunk }));
    await act(async () => {
      await result.current.start();
    });
    expect(result.current.state).toBe("denied");
    expect(result.current.error).toMatch(/type instead/i);
    expect(voiceApi.sttTranscribe).not.toHaveBeenCalled();
  });

  it("reports unavailable when the browser can't capture audio", async () => {
    (window as unknown as { MediaRecorder: unknown }).MediaRecorder = undefined;
    const onChunk = vi.fn();
    const { result } = renderHook(() => useDictation({ onChunk }));
    await act(async () => {
      await result.current.start();
    });
    expect(result.current.state).toBe("unavailable");
    expect(result.current.error).toMatch(/type instead/i);
    expect(voiceApi.sttTranscribe).not.toHaveBeenCalled();
  });

  it("skips a failed chunk without appending and keeps recording", async () => {
    voiceApi.sttTranscribe.mockRejectedValueOnce(
      new ApiError(503, "stt provider: no key", "voice_unavailable"),
    );
    const onChunk = vi.fn();
    const { result } = renderHook(() => useDictation({ onChunk }));
    await act(async () => {
      await result.current.start();
    });
    await act(async () => {
      FakeMediaRecorder.instances[0].stop();
    });
    await waitFor(() => expect(voiceApi.sttTranscribe).toHaveBeenCalled());
    // The failed chunk was skipped: no append, no blocking error, still live.
    expect(onChunk).not.toHaveBeenCalled();
    expect(result.current.state).toBe("recording");
  });

  it("toggle() starts when idle and stops when recording", async () => {
    const onChunk = vi.fn();
    const { result } = renderHook(() => useDictation({ onChunk }));
    await act(async () => {
      result.current.toggle();
    });
    await waitFor(() => expect(result.current.state).toBe("recording"));
    await act(async () => {
      result.current.toggle();
    });
    await waitFor(() => expect(result.current.state).toBe("idle"));
  });
});
