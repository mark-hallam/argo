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

  it("renders a canonical source chip on a canonical-Q&A hit", async () => {
    pitchApi.askPitch.mockResolvedValueOnce({
      matched: true,
      source: "canonical",
      entry_id: "tech-stack",
      answer: "Rust single-binary core.",
      declared_claim_classes: ["architecture"],
      disposition: "Allow",
      block_reason: null,
      channel: "Rehearsal",
      latency_ms: 2,
    });
    render(<PitchPage />);
    fireEvent.change(screen.getByTestId("pitch-input"), {
      target: { value: "Tech stack?" },
    });
    fireEvent.click(screen.getByTestId("pitch-submit"));
    await waitFor(() => {
      expect(screen.getByTestId("pitch-pill-source")).toBeInTheDocument();
    });
    expect(screen.getByTestId("pitch-pill-source")).toHaveTextContent(/canonical/i);
    // No model / tokens chips on the canonical hot path.
    expect(screen.queryByTestId("pitch-pill-model")).toBeNull();
    expect(screen.queryByTestId("pitch-pill-tokens")).toBeNull();
    expect(screen.queryByTestId("pitch-swap-note")).toBeNull();
  });

  it("renders model + token chips when the LLM fallback path responded", async () => {
    pitchApi.askPitch.mockResolvedValueOnce({
      matched: false,
      source: "llm",
      answer: "Skycot serves enterprise customers that need audit + multi-tenant from day one.",
      declared_claim_classes: [],
      disposition: "Allow",
      block_reason: null,
      channel: "Rehearsal",
      latency_ms: 850,
      model: "claude-sonnet-4-6",
      tokens: { input: 412, output: 87 },
    });
    render(<PitchPage />);
    fireEvent.change(screen.getByTestId("pitch-input"), {
      target: { value: "What kinds of teams use Skycot?" },
    });
    fireEvent.click(screen.getByTestId("pitch-submit"));
    await waitFor(() => {
      expect(screen.getByTestId("pitch-pill-source")).toBeInTheDocument();
    });
    expect(screen.getByTestId("pitch-pill-source")).toHaveTextContent(/llm/i);
    expect(screen.getByTestId("pitch-pill-model")).toHaveTextContent("claude-sonnet-4-6");
    expect(screen.getByTestId("pitch-pill-tokens")).toHaveTextContent(/in 412/);
    expect(screen.getByTestId("pitch-pill-tokens")).toHaveTextContent(/out 87/);
    expect(screen.queryByTestId("pitch-swap-note")).toBeNull();
    expect(screen.queryByTestId("pitch-draft")).toBeNull();
  });

  it("shows the canonical-refusal swap note + draft when the LLM was blocked", async () => {
    pitchApi.askPitch.mockResolvedValueOnce({
      matched: false,
      source: "llm_blocked_canonical_refused",
      answer:
        "I'm not authorized to share valuation publicly. Mark, do you want to take that one?",
      draft: "We are valued at around $40m in the most recent round.",
      declared_claim_classes: ["valuation"],
      disposition: "Block",
      block_reason: "forbidden_class: defer-to-mark-financial",
      channel: "WebEmbed",
      latency_ms: 920,
      model: "claude-sonnet-4-6",
      tokens: { input: 410, output: 32 },
    });
    render(<PitchPage />);
    fireEvent.change(screen.getByTestId("pitch-input"), {
      target: { value: "What's the round at?" },
    });
    fireEvent.click(screen.getByTestId("pitch-submit"));
    await waitFor(() => {
      expect(screen.getByTestId("pitch-pill-disposition")).toHaveTextContent(/blocked/i);
    });
    expect(screen.getByTestId("pitch-pill-source")).toHaveTextContent(/llm/i);
    expect(screen.getByTestId("pitch-pill-source")).toHaveTextContent(/blocked/i);
    expect(screen.getByTestId("pitch-swap-note")).toHaveTextContent(
      /canonical refusal substituted/i,
    );
    expect(screen.getByTestId("pitch-draft")).toHaveTextContent(/valued at around/i);
    expect(screen.getByTestId("pitch-pill-model")).toHaveTextContent("claude-sonnet-4-6");
  });

  it("treats responses without a source field as canonical (back-compat)", async () => {
    pitchApi.askPitch.mockResolvedValueOnce({
      matched: true,
      entry_id: "tech-stack",
      answer: "Rust single-binary core.",
      declared_claim_classes: ["architecture"],
      disposition: "Allow",
      block_reason: null,
      channel: "Rehearsal",
      latency_ms: 1,
    });
    render(<PitchPage />);
    fireEvent.change(screen.getByTestId("pitch-input"), {
      target: { value: "Tech stack?" },
    });
    fireEvent.click(screen.getByTestId("pitch-submit"));
    await waitFor(() => {
      expect(screen.getByTestId("pitch-pill-source")).toBeInTheDocument();
    });
    expect(screen.getByTestId("pitch-pill-source")).toHaveTextContent(/canonical/i);
  });
});
