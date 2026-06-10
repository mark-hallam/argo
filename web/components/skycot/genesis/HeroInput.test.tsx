import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";

/**
 * HeroInput mic-dictation tests. The component owns the mic button + its
 * idle/recording/denied affordances and appends each transcribed clip into the
 * controlled value via `onChange` (page.tsx stays the state owner). The
 * recording mechanics live in `useDictation`; here we mock that hook to drive
 * each visual state deterministically and assert the wiring (toggle, append,
 * fallback message) without touching real getUserMedia/MediaRecorder.
 */

const dictation = vi.hoisted(() => ({
  state: "idle" as "idle" | "recording" | "denied" | "unavailable",
  isRecording: false,
  error: null as string | null,
  start: vi.fn(),
  stop: vi.fn(),
  toggle: vi.fn(),
  onChunk: undefined as ((text: string) => void) | undefined,
}));

vi.mock("@/lib/api/use-dictation", () => ({
  useDictation: (opts: { onChunk: (text: string) => void }) => {
    dictation.onChunk = opts.onChunk;
    return {
      state: dictation.state,
      isRecording: dictation.isRecording,
      error: dictation.error,
      start: dictation.start,
      stop: dictation.stop,
      toggle: dictation.toggle,
    };
  },
}));

import { HeroInput } from "./HeroInput";

function resetDictation() {
  dictation.state = "idle";
  dictation.isRecording = false;
  dictation.error = null;
  dictation.start.mockReset();
  dictation.stop.mockReset();
  dictation.toggle.mockReset();
  dictation.onChunk = undefined;
}

describe("HeroInput mic dictation", () => {
  beforeEach(() => resetDictation());
  afterEach(() => vi.clearAllMocks());

  it("renders the idle Dictate affordance and calls toggle on click", () => {
    render(<HeroInput value="" onChange={() => {}} onSubmit={() => {}} isBusy={false} />);
    const mic = screen.getByTestId("genesis-mic-button");
    expect(mic.textContent).toMatch(/dictate/i);
    expect(mic).toHaveAttribute("aria-pressed", "false");
    fireEvent.click(mic);
    expect(dictation.toggle).toHaveBeenCalledTimes(1);
  });

  it("shows the recording state (Listening…) while dictating", () => {
    dictation.state = "recording";
    dictation.isRecording = true;
    render(<HeroInput value="" onChange={() => {}} onSubmit={() => {}} isBusy={false} />);
    const mic = screen.getByTestId("genesis-mic-button");
    expect(mic.textContent).toMatch(/listening/i);
    expect(mic).toHaveAttribute("aria-pressed", "true");
  });

  it("renders the type-instead fallback when the mic is denied", () => {
    dictation.state = "denied";
    dictation.error = "Mic unavailable — type instead.";
    render(<HeroInput value="" onChange={() => {}} onSubmit={() => {}} isBusy={false} />);
    expect(screen.getByTestId("genesis-mic-error").textContent).toMatch(/type instead/i);
    // The textarea is unaffected — typing still works.
    expect(screen.getByTestId("genesis-hero-form")).toBeInTheDocument();
  });

  it("appends a transcribed chunk into the controlled value via onChange", () => {
    const onChange = vi.fn();
    render(
      <HeroInput
        value="Triages tier-1 tickets"
        onChange={onChange}
        onSubmit={() => {}}
        isBusy={false}
      />,
    );
    // Simulate the hook emitting a clip transcript.
    dictation.onChunk?.("and escalates refunds over $500");
    expect(onChange).toHaveBeenCalledWith(
      "Triages tier-1 tickets and escalates refunds over $500",
    );
  });

  it("uses the bare transcript when the textarea is empty", () => {
    const onChange = vi.fn();
    render(<HeroInput value="" onChange={onChange} onSubmit={() => {}} isBusy={false} />);
    dictation.onChunk?.("a customer success lead");
    expect(onChange).toHaveBeenCalledWith("a customer success lead");
  });

  it("disables the mic button while the page is busy generating", () => {
    render(<HeroInput value="A valid role description" onChange={() => {}} onSubmit={() => {}} isBusy />);
    expect(screen.getByTestId("genesis-mic-button")).toBeDisabled();
  });
});
