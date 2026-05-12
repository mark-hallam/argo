import { act, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { Step3Drafting } from "./Step3Drafting";
import type { WorkerDraft } from "@/lib/hire/draft-types";
import type { UseGenesisWorkerDraftResult } from "@/lib/hire/use-genesis-worker-draft";

let mockResult: UseGenesisWorkerDraftResult = {
  start: () => undefined,
  reset: () => undefined,
  data: undefined,
  error: null,
  isPending: false,
  isSuccess: false,
  isError: false,
};

const startSpy = vi.fn();
const resetSpy = vi.fn();

vi.mock("@/lib/hire/use-genesis-worker-draft", () => ({
  useGenesisWorkerDraft: () => mockResult,
}));

function setMock(next: Partial<UseGenesisWorkerDraftResult>) {
  mockResult = {
    start: startSpy,
    reset: resetSpy,
    data: undefined,
    error: null,
    isPending: false,
    isSuccess: false,
    isError: false,
    ...next,
  };
}

const SAMPLE_DRAFT: WorkerDraft = {
  suggestedName: "Mariah",
  persona: {
    tagline: "Marketplace assistant",
    avatarKind: "initials",
    voiceProfile: { tone: "warm", formality: "medium", signaturePhrases: [] },
  },
  charter: "Helps you with messages.",
  duties: [],
  exampleReplies: [],
  approvalPolicy: { level: "balanced" },
  suggestedVertical: "marketplace-seller",
};

beforeEach(() => {
  startSpy.mockClear();
  resetSpy.mockClear();
  setMock({});
  vi.useFakeTimers();
});

afterEach(() => {
  vi.useRealTimers();
  vi.restoreAllMocks();
});

function renderStep3(overrides?: Partial<React.ComponentProps<typeof Step3Drafting>>) {
  const onComplete = overrides?.onComplete ?? vi.fn();
  const onSkipToTemplate = overrides?.onSkipToTemplate ?? vi.fn();
  return {
    onComplete,
    onSkipToTemplate,
    ...render(
      <Step3Drafting
        problem={overrides?.problem ?? "I sell hockey gear."}
        answers={overrides?.answers ?? {}}
        onComplete={onComplete}
        onSkipToTemplate={onSkipToTemplate}
      />,
    ),
  };
}

describe("Step3Drafting", () => {
  it("fires the worker-draft mutation on mount", () => {
    renderStep3({
      problem: "p",
      answers: {
        q1: { question_id: "q1", type: "yes-no", value: true },
      },
    });
    expect(startSpy).toHaveBeenCalledTimes(1);
    expect(startSpy).toHaveBeenCalledWith({
      problem: "p",
      answers: { q1: { question_id: "q1", type: "yes-no", value: true } },
    });
  });

  it("reveals stages on the cosmetic timer at ~700ms intervals", () => {
    renderStep3();
    // Stage 0 visible immediately.
    expect(screen.getByTestId("hire-step3-stage-0")).toHaveAttribute("data-state", "active");
    expect(screen.getByTestId("hire-step3-stage-1")).toHaveAttribute("data-state", "pending");

    act(() => {
      vi.advanceTimersByTime(700);
    });
    expect(screen.getByTestId("hire-step3-stage-0")).toHaveAttribute("data-state", "done");
    expect(screen.getByTestId("hire-step3-stage-1")).toHaveAttribute("data-state", "active");

    act(() => {
      vi.advanceTimersByTime(700 * 3);
    });
    // 4 × 700 = 2800ms total → stage 4 ("Almost done…") now active.
    expect(screen.getByTestId("hire-step3-stage-4")).toHaveAttribute("data-state", "active");
  });

  it("waits for the cosmetic timer when backend resolves first", () => {
    const onComplete = vi.fn();
    setMock({ isSuccess: true, data: { draft: SAMPLE_DRAFT, source: "gateway" } });
    renderStep3({ onComplete });
    // Backend already done, but timer has not reached stage 4 yet.
    expect(onComplete).not.toHaveBeenCalled();
    act(() => {
      vi.advanceTimersByTime(700 * 4);
    });
    expect(onComplete).toHaveBeenCalledTimes(1);
    expect(onComplete).toHaveBeenCalledWith(SAMPLE_DRAFT);
  });

  it("holds at 'Almost done…' with a spinner when backend is slower than the timer", () => {
    const onComplete = vi.fn();
    setMock({ isPending: true });
    renderStep3({ onComplete });
    act(() => {
      vi.advanceTimersByTime(700 * 5);
    });
    expect(screen.getByTestId("hire-step3-stage-4")).toHaveAttribute("data-state", "active");
    expect(screen.getByTestId("hire-step3-almost-done-spinner")).toBeInTheDocument();
    expect(onComplete).not.toHaveBeenCalled();
  });

  it("shows the 'still working' hint after the 8s mark", () => {
    setMock({ isPending: true });
    renderStep3();
    expect(screen.queryByTestId("hire-step3-still-working")).not.toBeInTheDocument();
    act(() => {
      vi.advanceTimersByTime(8_000);
    });
    expect(screen.getByTestId("hire-step3-still-working")).toBeInTheDocument();
  });

  it("surfaces the retry panel at the 30s timeout and re-fires the mutation on Try again", () => {
    setMock({ isPending: true });
    renderStep3();
    act(() => {
      vi.advanceTimersByTime(30_000);
    });
    expect(screen.getByTestId("hire-step3-retry-panel")).toBeInTheDocument();
    expect(startSpy).toHaveBeenCalledTimes(1);

    act(() => {
      fireEvent.click(screen.getByTestId("hire-step3-retry"));
    });
    // Retry resets stage clock + re-fires the mutation.
    expect(startSpy).toHaveBeenCalledTimes(2);
    expect(screen.queryByTestId("hire-step3-retry-panel")).not.toBeInTheDocument();
    expect(screen.getByTestId("hire-step3-stage-0")).toHaveAttribute("data-state", "active");
  });

  it("surfaces the skip-to-template panel after 3 retries hit the timeout", () => {
    const onSkip = vi.fn();
    setMock({ isPending: true });
    renderStep3({ onSkipToTemplate: onSkip });

    for (let i = 0; i < 3; i++) {
      act(() => {
        vi.advanceTimersByTime(30_000);
      });
      act(() => {
        fireEvent.click(screen.getByTestId("hire-step3-retry"));
      });
    }
    // After 3 retries, allow the 4th 30s wall to land → skip panel.
    act(() => {
      vi.advanceTimersByTime(30_000);
    });
    expect(screen.getByTestId("hire-step3-skip-to-template")).toBeInTheDocument();

    act(() => {
      fireEvent.click(screen.getByTestId("hire-step3-template-marketplace-seller"));
    });
    expect(onSkip).toHaveBeenCalledTimes(1);
    expect(onSkip).toHaveBeenCalledWith(
      expect.objectContaining({ title: expect.stringMatching(/marketplace/i) }),
    );
  });

  it("immediately shows the skip-to-template panel when the user clicks 'Pick a template instead'", () => {
    const onSkip = vi.fn();
    setMock({ isPending: true });
    renderStep3({ onSkipToTemplate: onSkip });
    act(() => {
      vi.advanceTimersByTime(30_000);
    });
    expect(screen.getByTestId("hire-step3-retry-panel")).toBeInTheDocument();
    act(() => {
      fireEvent.click(screen.getByTestId("hire-step3-pick-template"));
    });
    expect(screen.getByTestId("hire-step3-skip-to-template")).toBeInTheDocument();
    expect(screen.queryByTestId("hire-step3-retry-panel")).not.toBeInTheDocument();
  });

  it("cancels the in-flight mutation on unmount", () => {
    const { unmount } = renderStep3();
    unmount();
    expect(resetSpy).toHaveBeenCalled();
  });
});
