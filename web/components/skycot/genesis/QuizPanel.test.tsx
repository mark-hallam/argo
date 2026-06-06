import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { QuizPanel } from "./QuizPanel";

/**
 * QuizPanel tests — the comprehension gate before Hire. These assert the gate
 * behaviour the parent relies on: suggested questions render, asking a question
 * shows the retrieved answer + citation, the honest "no SOP covers this" path is
 * surfaced distinctly, and the "I'm satisfied" confirmation fires `onSatisfied`
 * (the parent uses that to enable the Hire button).
 */

vi.mock("@/lib/api/genesis-quiz", () => ({
  askQuiz: vi.fn(),
  fetchSuggestedQuestions: vi.fn(),
}));

import { askQuiz, fetchSuggestedQuestions } from "@/lib/api/genesis-quiz";

const mockAsk = vi.mocked(askQuiz);
const mockSuggest = vi.mocked(fetchSuggestedQuestions);

function renderPanel(overrides: Partial<Parameters<typeof QuizPanel>[0]> = {}) {
  const onSatisfied = vi.fn();
  render(
    <QuizPanel
      verticalId="genesis-sam-ab12cd"
      name="Sam"
      roleDescription="support teammate"
      onSatisfied={onSatisfied}
      satisfied={false}
      {...overrides}
    />,
  );
  return { onSatisfied };
}

describe("QuizPanel", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockSuggest.mockResolvedValue([
      "Walk me through the most important thing you learned.",
      "Based on what you read, how would you handle a refund request?",
    ]);
  });

  it("renders suggested questions fetched for the teammate", async () => {
    renderPanel();
    await waitFor(() =>
      expect(screen.getByTestId("genesis-quiz-suggestions")).toBeInTheDocument(),
    );
    expect(screen.getByTestId("genesis-quiz-suggestions").textContent).toMatch(
      /how would you handle a refund/i,
    );
  });

  it("asks a question and shows the retrieved answer with its citation", async () => {
    mockAsk.mockResolvedValue({
      answered: true,
      answer: "Refunds over $500 require regional manager approval.",
      citations: [
        { source_label: "refund-policy.md", excerpt: "Refunds over $500 require regional manager approval." },
      ],
      suggested_questions: [],
    });
    renderPanel();
    fireEvent.change(screen.getByTestId("genesis-quiz-input"), {
      target: { value: "How do I handle a refund over $500?" },
    });
    fireEvent.click(screen.getByTestId("genesis-quiz-ask"));
    await waitFor(() =>
      expect(screen.getByTestId("genesis-quiz-history")).toBeInTheDocument(),
    );
    expect(screen.getByTestId("genesis-quiz-history").textContent).toContain(
      "regional manager approval",
    );
    // The citation source is shown so the operator sees which SOP was quoted.
    expect(screen.getByTestId("genesis-quiz-history").textContent).toContain(
      "refund-policy.md",
    );
  });

  it("surfaces the honest no-knowledge answer when nothing matches", async () => {
    mockAsk.mockResolvedValue({
      answered: false,
      answer: "I don't have a SOP that covers this.",
      citations: [],
      suggested_questions: [],
    });
    renderPanel();
    fireEvent.change(screen.getByTestId("genesis-quiz-input"), {
      target: { value: "What is our parental leave policy?" },
    });
    fireEvent.click(screen.getByTestId("genesis-quiz-ask"));
    await waitFor(() =>
      expect(screen.getByTestId("genesis-quiz-history").textContent).toMatch(
        /don't have a SOP/i,
      ),
    );
  });

  it("fires onSatisfied when the operator confirms — the Hire gate", async () => {
    const { onSatisfied } = renderPanel();
    fireEvent.click(screen.getByTestId("genesis-quiz-satisfied"));
    expect(onSatisfied).toHaveBeenCalledTimes(1);
  });

  it("shows the ready-to-hire state once satisfied", () => {
    renderPanel({ satisfied: true });
    expect(screen.getByTestId("genesis-quiz-satisfied-done")).toBeInTheDocument();
    expect(screen.queryByTestId("genesis-quiz-satisfied")).not.toBeInTheDocument();
  });
});
