import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useState } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { Step2Listen } from "./Step2Listen";
import type { Question, QuestionAnswer } from "@/lib/hire/genesis-types";
import type { UseGenesisQuestionsResult } from "@/lib/hire/use-genesis-questions";

let mockResult: UseGenesisQuestionsResult = {
  questions: [],
  status: "loading",
  source: undefined,
  error: null,
  refetch: () => undefined,
  attemptCount: 0,
};

vi.mock("@/lib/hire/use-genesis-questions", () => ({
  useGenesisQuestions: () => mockResult,
}));

function setMock(next: UseGenesisQuestionsResult) {
  mockResult = next;
}

const GATEWAY_QUESTIONS: Question[] = [
  {
    id: "q1",
    text: "How many messages a day?",
    answer_type: "single-select",
    options: ["0–10", "10–50", "50+"],
    required: true,
  },
  {
    id: "q2",
    text: "What's the typical reply?",
    answer_type: "free-text",
    required: true,
  },
  {
    id: "q3",
    text: "Should the worker ever act without asking?",
    answer_type: "yes-no",
    required: true,
  },
];

/** Tracks snapshot emissions so tests can assert canContinue + answers. */
function Harness({
  initialAnswers,
  onSkipToTemplate,
}: {
  initialAnswers?: Record<string, QuestionAnswer>;
  onSkipToTemplate?: (description: string) => void;
}) {
  const [snap, setSnap] = useState<{
    answers: Record<string, QuestionAnswer>;
    canContinue: boolean;
  }>({ answers: {}, canContinue: false });
  return (
    <div>
      <div data-testid="harness-can-continue">{String(snap.canContinue)}</div>
      <div data-testid="harness-answer-count">
        {String(Object.keys(snap.answers).length)}
      </div>
      <Step2Listen
        problem="I sell hockey gear on Facebook Marketplace."
        initialAnswers={initialAnswers}
        onChange={(s) => setSnap({ answers: s.answers, canContinue: s.canContinue })}
        onSkipToTemplate={(example) => onSkipToTemplate?.(example.description)}
      />
    </div>
  );
}

beforeEach(() => {
  setMock({
    questions: [],
    status: "loading",
    source: undefined,
    error: null,
    refetch: () => undefined,
    attemptCount: 0,
  });
});

afterEach(() => {
  vi.restoreAllMocks();
});

describe("Step2Listen", () => {
  it("renders the loading skeleton while Genesis is loading", () => {
    setMock({
      questions: [],
      status: "loading",
      source: undefined,
      error: null,
      refetch: () => undefined,
      attemptCount: 0,
    });
    render(<Harness />);
    expect(screen.getByTestId("hire-step2-loading")).toBeInTheDocument();
    expect(screen.queryByTestId("hire-step2-questions")).not.toBeInTheDocument();
  });

  it("renders 1–3 question cards once Genesis is ready", () => {
    setMock({
      questions: GATEWAY_QUESTIONS,
      status: "ready",
      source: "gateway",
      error: null,
      refetch: () => undefined,
      attemptCount: 1,
    });
    render(<Harness />);
    expect(screen.getByTestId("hire-step2-questions")).toBeInTheDocument();
    expect(screen.getByTestId("hire-step2-question-q1")).toBeInTheDocument();
    expect(screen.getByTestId("hire-step2-question-q2")).toBeInTheDocument();
    expect(screen.getByTestId("hire-step2-question-q3")).toBeInTheDocument();
    // No fallback banner on gateway-sourced questions.
    expect(
      screen.queryByTestId("hire-step2-fallback-banner"),
    ).not.toBeInTheDocument();
  });

  it("Continue stays disabled until every required answer is filled", async () => {
    const user = userEvent.setup();
    setMock({
      questions: GATEWAY_QUESTIONS,
      status: "ready",
      source: "gateway",
      error: null,
      refetch: () => undefined,
      attemptCount: 1,
    });
    render(<Harness />);
    expect(screen.getByTestId("harness-can-continue")).toHaveTextContent("false");

    await user.click(screen.getByTestId("hire-step2-answer-q1-10–50"));
    expect(screen.getByTestId("harness-can-continue")).toHaveTextContent("false");

    await user.type(
      screen.getByTestId("hire-step2-answer-q2"),
      "Let me check and get back to you.",
    );
    expect(screen.getByTestId("harness-can-continue")).toHaveTextContent("false");

    await user.click(screen.getByTestId("hire-step2-answer-q3-no"));
    await waitFor(() =>
      expect(screen.getByTestId("harness-can-continue")).toHaveTextContent("true"),
    );
    expect(screen.getByTestId("harness-answer-count")).toHaveTextContent("3");
  });

  it("renders the fallback banner when Genesis falls back to the stub", () => {
    setMock({
      questions: [
        {
          id: "stub_action_mode",
          text: "Should they reply on your behalf, or draft replies for you to send?",
          answer_type: "single-select",
          options: ["Reply on my behalf", "Draft for me to send", "Let me decide later"],
          required: true,
        },
      ],
      status: "fallback",
      source: "stub",
      error: null,
      refetch: () => undefined,
      attemptCount: 1,
    });
    render(<Harness />);
    expect(screen.getByTestId("hire-step2-fallback-banner")).toHaveTextContent(
      /backup questions/i,
    );
    expect(screen.getByTestId("hire-step2-question-stub_action_mode")).toBeInTheDocument();
  });

  it("renders the skip-to-template panel after Genesis errors out", async () => {
    const user = userEvent.setup();
    const onSkip = vi.fn();
    setMock({
      questions: [],
      status: "error",
      source: undefined,
      error: new Error("retries exhausted"),
      refetch: () => undefined,
      attemptCount: 3,
    });
    render(<Harness onSkipToTemplate={onSkip} />);
    expect(screen.getByTestId("hire-step2-skip-to-template")).toBeInTheDocument();
    // All 5 example tiles should be present (re-using E2-b's EXAMPLE_ROLES).
    expect(
      screen.getByTestId("hire-step2-template-marketplace-seller"),
    ).toBeInTheDocument();
    expect(
      screen.getByTestId("hire-step2-template-tax-prep-document-intake"),
    ).toBeInTheDocument();

    await user.click(screen.getByTestId("hire-step2-template-marketplace-seller"));
    expect(onSkip).toHaveBeenCalledTimes(1);
    expect(onSkip).toHaveBeenCalledWith(
      expect.stringMatching(/hockey gear on facebook marketplace/i),
    );
  });

  it("does not echo the user's Step 1 problem text back to them", () => {
    setMock({
      questions: GATEWAY_QUESTIONS,
      status: "ready",
      source: "gateway",
      error: null,
      refetch: () => undefined,
      attemptCount: 1,
    });
    render(<Harness />);
    // PII echo protection — the raw input must NOT appear in rendered text.
    expect(
      screen.queryByText(/i sell hockey gear on facebook marketplace/i),
    ).not.toBeInTheDocument();
  });

  it("seeds answers from initialAnswers on mount", () => {
    setMock({
      questions: GATEWAY_QUESTIONS,
      status: "ready",
      source: "gateway",
      error: null,
      refetch: () => undefined,
      attemptCount: 1,
    });
    const seeded: Record<string, QuestionAnswer> = {
      q1: { question_id: "q1", type: "single-select", value: "10–50" },
      q2: { question_id: "q2", type: "free-text", value: "Sure, let me check." },
      q3: { question_id: "q3", type: "yes-no", value: false },
    };
    render(<Harness initialAnswers={seeded} />);
    expect(screen.getByTestId("harness-can-continue")).toHaveTextContent("true");
    expect(screen.getByTestId("harness-answer-count")).toHaveTextContent("3");
  });
});
