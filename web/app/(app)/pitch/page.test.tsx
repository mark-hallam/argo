import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const pitchApi = vi.hoisted(() => ({
  askPitch: vi.fn(),
}));

vi.mock("@/lib/api/pitch", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@/lib/api/pitch")>();
  return { ...actual, ...pitchApi };
});

import PitchPage from "./page";
import { ApiError } from "@/lib/api/types";

describe("/pitch page", () => {
  beforeEach(() => {
    pitchApi.askPitch.mockReset();
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  it("renders the empty state until the operator asks something", () => {
    render(<PitchPage />);
    expect(screen.getByTestId("pitch-empty")).toBeInTheDocument();
    expect(screen.getByTestId("pitch-input")).toBeInTheDocument();
    expect(screen.getByTestId("pitch-submit")).toBeDisabled();
  });

  it("submits the question + renders the agent reply with disposition pill", async () => {
    pitchApi.askPitch.mockResolvedValueOnce({
      matched: true,
      entry_id: "tech-stack",
      answer: "Rust single-binary core.",
      declared_claim_classes: ["architecture"],
      disposition: "Allow",
      block_reason: null,
      channel: "Rehearsal",
      latency_ms: 2,
    });
    render(<PitchPage />);
    const input = screen.getByTestId("pitch-input") as HTMLTextAreaElement;
    fireEvent.change(input, { target: { value: "What's the tech stack?" } });
    fireEvent.click(screen.getByTestId("pitch-submit"));
    await waitFor(() => {
      expect(screen.getByTestId("pitch-msg-agent")).toBeInTheDocument();
    });
    expect(screen.getByTestId("pitch-msg-operator")).toHaveTextContent(
      "What's the tech stack?",
    );
    expect(screen.getByTestId("pitch-msg-agent")).toHaveTextContent("Rust single-binary core.");
    expect(screen.getByTestId("pitch-pill-disposition")).toHaveTextContent(/allowed/i);
    expect(screen.getByTestId("pitch-pill-entry")).toHaveTextContent("tech-stack");
    expect(pitchApi.askPitch).toHaveBeenCalledWith({
      question: "What's the tech stack?",
      channel: "Rehearsal",
    });
  });

  it("renders a Block pill when the gateway flags a forbidden claim class", async () => {
    pitchApi.askPitch.mockResolvedValueOnce({
      matched: true,
      entry_id: "valuation-question",
      answer: "Mark handles financial specifics directly.",
      declared_claim_classes: ["valuation"],
      disposition: "Block",
      block_reason: "forbidden_class: defer-to-mark-financial",
      channel: "Rehearsal",
      latency_ms: 1,
    });
    render(<PitchPage />);
    fireEvent.change(screen.getByTestId("pitch-input"), {
      target: { value: "What's the valuation?" },
    });
    fireEvent.click(screen.getByTestId("pitch-submit"));
    await waitFor(() => {
      expect(screen.getByTestId("pitch-pill-disposition")).toHaveTextContent(/blocked/i);
    });
  });

  it("surfaces an error banner on gateway 5xx", async () => {
    pitchApi.askPitch.mockRejectedValueOnce(
      new ApiError(503, "gateway down", "service_unavailable"),
    );
    render(<PitchPage />);
    fireEvent.change(screen.getByTestId("pitch-input"), {
      target: { value: "Hi" },
    });
    fireEvent.click(screen.getByTestId("pitch-submit"));
    await waitFor(() => {
      expect(screen.getByTestId("pitch-error")).toBeInTheDocument();
    });
    expect(screen.getByTestId("pitch-error")).toHaveTextContent(/gateway down/i);
  });
});
