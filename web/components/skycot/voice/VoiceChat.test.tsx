import { act, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { createRef } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { VoiceChat, voiceChatSupported, type VoiceChatHandle } from "./VoiceChat";

class FakeMediaRecorder {
  static instances: FakeMediaRecorder[] = [];
  state: RecordingState = "inactive";
  mimeType = "audio/webm";
  ondataavailable: ((event: BlobEvent) => void) | null = null;
  onerror: (() => void) | null = null;
  onstop: (() => void) | null = null;

  constructor(public stream: MediaStream) {
    FakeMediaRecorder.instances.push(this);
  }

  start() {
    this.state = "recording";
  }

  stop() {
    this.state = "inactive";
    this.ondataavailable?.({
      data: new Blob(["voice"], { type: "audio/webm" }),
    } as BlobEvent);
    this.onstop?.();
  }

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

class FakeAudio {
  static instances: FakeAudio[] = [];
  onended: (() => void) | null = null;
  onerror: (() => void) | null = null;
  pause = vi.fn();
  play = vi.fn(() => Promise.resolve());
  src: string;

  constructor(src: string) {
    this.src = src;
    FakeAudio.instances.push(this);
  }

  finish() {
    this.onended?.();
  }
}

let getUserMedia: ReturnType<typeof vi.fn>;
let createObjectURL: ReturnType<typeof vi.fn>;
let revokeObjectURL: ReturnType<typeof vi.fn>;

function installVoiceBrowser() {
  FakeMediaRecorder.instances = [];
  FakeAudio.instances = [];
  getUserMedia = vi.fn(() => Promise.resolve(new FakeMediaStream() as unknown as MediaStream));
  createObjectURL = vi.fn(() => "blob:voice-response");
  revokeObjectURL = vi.fn();

  Object.defineProperty(navigator, "mediaDevices", {
    configurable: true,
    value: { getUserMedia },
  });
  Object.defineProperty(window, "MediaRecorder", {
    configurable: true,
    value: FakeMediaRecorder,
  });
  Object.defineProperty(window, "Audio", {
    configurable: true,
    value: FakeAudio,
  });
  Object.defineProperty(globalThis, "Audio", {
    configurable: true,
    value: FakeAudio,
  });
  Object.defineProperty(URL, "createObjectURL", {
    configurable: true,
    value: createObjectURL,
  });
  Object.defineProperty(URL, "revokeObjectURL", {
    configurable: true,
    value: revokeObjectURL,
  });
}

describe("VoiceChat", () => {
  beforeEach(() => {
    installVoiceBrowser();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("reports browser voice support when MediaRecorder and getUserMedia exist", () => {
    expect(voiceChatSupported()).toBe(true);
  });

  it("records mic audio and passes the blob to onAudio without posting itself", async () => {
    const user = userEvent.setup();
    const onAudio = vi.fn((blob: Blob) => {
      void blob;
      return undefined;
    });
    const onStateChange = vi.fn();
    render(<VoiceChat label="Talk to Portia" onAudio={onAudio} onStateChange={onStateChange} />);

    await user.click(screen.getByRole("button", { name: "Tap to talk" }));

    expect(getUserMedia).toHaveBeenCalledWith({ audio: true });
    expect(screen.getByTestId("voice-chat")).toHaveAttribute("data-state", "recording");
    expect(screen.getByRole("button", { name: "Stop recording" })).toHaveAttribute(
      "aria-pressed",
      "true",
    );

    await user.click(screen.getByRole("button", { name: "Stop recording" }));

    await waitFor(() => expect(onAudio).toHaveBeenCalledTimes(1));
    const [[blob]] = onAudio.mock.calls as [[Blob]];
    expect(blob).toBeInstanceOf(Blob);
    expect(blob.type).toBe("audio/webm");
    await waitFor(() =>
      expect(screen.getByTestId("voice-chat")).toHaveAttribute("data-state", "idle"),
    );
    expect(onStateChange).toHaveBeenCalledWith("recording");
    expect(onStateChange).toHaveBeenCalledWith("transcribing");
  });

  it("stays in transcribing while onAudio is pending, then plays a returned response blob", async () => {
    const user = userEvent.setup();
    let resolveAudio: (value: { audio: Blob }) => void = () => {};
    const onAudio = vi.fn(
      () =>
        new Promise<{ audio: Blob }>((resolve) => {
          resolveAudio = resolve;
        }),
    );
    render(<VoiceChat onAudio={onAudio} />);

    await user.click(screen.getByRole("button", { name: "Tap to talk" }));
    await user.click(screen.getByRole("button", { name: "Stop recording" }));

    await waitFor(() =>
      expect(screen.getByTestId("voice-chat")).toHaveAttribute("data-state", "transcribing"),
    );

    await act(async () => {
      resolveAudio({ audio: new Blob(["reply"], { type: "audio/mpeg" }) });
    });

    await waitFor(() =>
      expect(screen.getByTestId("voice-chat")).toHaveAttribute("data-state", "speaking"),
    );
    expect(createObjectURL).toHaveBeenCalledTimes(1);
    expect(FakeAudio.instances[0].src).toBe("blob:voice-response");
    expect(FakeAudio.instances[0].play).toHaveBeenCalledTimes(1);

    act(() => FakeAudio.instances[0].finish());

    await waitFor(() =>
      expect(screen.getByTestId("voice-chat")).toHaveAttribute("data-state", "idle"),
    );
    expect(revokeObjectURL).toHaveBeenCalledWith("blob:voice-response");
  });

  it("exposes playResponse(blob | url) for consumers that post audio themselves", async () => {
    const ref = createRef<VoiceChatHandle>();
    render(<VoiceChat ref={ref} onAudio={vi.fn()} />);

    let promise: Promise<void> | undefined;
    act(() => {
      promise = ref.current?.playResponse("https://audio.example/answer.mp3");
    });
    await waitFor(() =>
      expect(screen.getByTestId("voice-chat")).toHaveAttribute("data-state", "speaking"),
    );
    await act(async () => {
      FakeAudio.instances[0].finish();
      await promise;
    });

    expect(FakeAudio.instances[0].src).toBe("https://audio.example/answer.mp3");
    await waitFor(() =>
      expect(screen.getByTestId("voice-chat")).toHaveAttribute("data-state", "idle"),
    );
    expect(ref.current?.state).toBe("idle");
  });

  it("surfaces denied mic permission without calling onAudio", async () => {
    const user = userEvent.setup();
    getUserMedia.mockRejectedValueOnce(new DOMException("denied", "NotAllowedError"));
    const onAudio = vi.fn();
    const onError = vi.fn();
    render(<VoiceChat onAudio={onAudio} onError={onError} />);

    await user.click(screen.getByRole("button", { name: "Tap to talk" }));

    expect(await screen.findByTestId("voice-chat-error")).toHaveTextContent(
      /microphone permission denied/i,
    );
    expect(onError).toHaveBeenCalledWith(expect.stringMatching(/permission denied/i));
    expect(onAudio).not.toHaveBeenCalled();
    expect(screen.getByTestId("voice-chat")).toHaveAttribute("data-state", "idle");
  });

  it("flags empty recordings instead of calling onAudio with silence", async () => {
    const user = userEvent.setup();
    const onAudio = vi.fn();
    render(<VoiceChat onAudio={onAudio} />);

    await user.click(screen.getByRole("button", { name: "Tap to talk" }));
    act(() => FakeMediaRecorder.instances[0].stopEmpty());

    expect(await screen.findByTestId("voice-chat-error")).toHaveTextContent(/no audio captured/i);
    expect(onAudio).not.toHaveBeenCalled();
  });

  it("handles unsupported browsers gracefully", async () => {
    const user = userEvent.setup();
    Object.defineProperty(window, "MediaRecorder", {
      configurable: true,
      value: undefined,
    });
    const onAudio = vi.fn();
    render(<VoiceChat onAudio={onAudio} />);

    await user.click(screen.getByRole("button", { name: "Tap to talk" }));

    expect(await screen.findByTestId("voice-chat-error")).toHaveTextContent(
      /can't capture microphone/i,
    );
    expect(onAudio).not.toHaveBeenCalled();
  });
});
