import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { act, renderHook } from "@testing-library/react";

// The live-voice path mints an ephemeral secret + persists transcript turns.
// Mock the network module so the hook tests never touch the gateway — we are
// exercising the SSR-safe capability gate + the unsupported-environment guard,
// not the WebRTC SDP exchange (which needs a real browser + mic permission).
vi.mock("./voice", () => ({
  createRealtimeSession: vi.fn(),
  postVoiceTranscriptTurn: vi.fn(),
}));

import { realtimeVoiceSupported, useRealtimeVoice } from "./use-realtime-voice";
import { createRealtimeSession } from "./voice";

const mockedCreateSession = vi.mocked(createRealtimeSession);

describe("lib/api/use-realtime-voice · realtimeVoiceSupported", () => {
  const originalMediaDevices = navigator.mediaDevices;
  const originalRTC = (globalThis as Record<string, unknown>).RTCPeerConnection;

  afterEach(() => {
    Object.defineProperty(navigator, "mediaDevices", {
      value: originalMediaDevices,
      configurable: true,
    });
    (globalThis as Record<string, unknown>).RTCPeerConnection = originalRTC;
    vi.restoreAllMocks();
  });

  it("returns false when the browser exposes no getUserMedia (e.g. happy-dom / SSR)", () => {
    Object.defineProperty(navigator, "mediaDevices", {
      value: undefined,
      configurable: true,
    });
    expect(realtimeVoiceSupported()).toBe(false);
  });

  it("returns false when RTCPeerConnection is absent even if mic capture exists", () => {
    Object.defineProperty(navigator, "mediaDevices", {
      value: { getUserMedia: () => Promise.resolve({} as MediaStream) },
      configurable: true,
    });
    delete (globalThis as Record<string, unknown>).RTCPeerConnection;
    expect(realtimeVoiceSupported()).toBe(false);
  });

  it("returns true when both mic capture and WebRTC are present", () => {
    Object.defineProperty(navigator, "mediaDevices", {
      value: { getUserMedia: () => Promise.resolve({} as MediaStream) },
      configurable: true,
    });
    (globalThis as Record<string, unknown>).RTCPeerConnection = function () {} as unknown;
    expect(realtimeVoiceSupported()).toBe(true);
  });
});

describe("lib/api/use-realtime-voice · useRealtimeVoice", () => {
  beforeEach(() => {
    mockedCreateSession.mockReset();
  });

  it("starts in the idle phase with no transcripts and no error", () => {
    const { result } = renderHook(() => useRealtimeVoice());
    expect(result.current.phase).toBe("idle");
    expect(result.current.isLive).toBe(false);
    expect(result.current.error).toBeNull();
    expect(result.current.transcripts).toEqual([]);
    expect(result.current.liveCaption).toBe("");
  });

  it("start() in an unsupported environment sets an honest error, never minting a session", async () => {
    // happy-dom has no RTCPeerConnection, so realtimeVoiceSupported() is false
    // and start() must bail before any network mint.
    const { result } = renderHook(() => useRealtimeVoice());

    await act(async () => {
      await result.current.start();
    });

    expect(result.current.phase).toBe("error");
    expect(result.current.error).toContain("live voice");
    expect(mockedCreateSession).not.toHaveBeenCalled();
  });

  it("stop() is a safe no-op from idle", () => {
    const { result } = renderHook(() => useRealtimeVoice());
    act(() => {
      result.current.stop();
    });
    expect(result.current.phase).toBe("idle");
    expect(result.current.isLive).toBe(false);
  });
});
