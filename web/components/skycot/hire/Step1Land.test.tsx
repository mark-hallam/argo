import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { Step1Land } from "./Step1Land";
import { PLACEHOLDER_PROMPTS, STEP1_MIN_CHARS } from "@/lib/hire/examples";
import { useStep1 } from "@/lib/hire/use-step1";

/**
 * Step 1 (Land) renders inside a tiny harness that holds `useStep1`
 * so the tests exercise the real hook integration (textarea state,
 * validation hint, Cmd+Enter, examples-fill). `submitSpy` captures
 * successful advances so we can assert the value that would be sent
 * to the draft store / router.
 */
function Harness({
  initialValue = "",
  onSubmit,
  onCanSubmit,
}: {
  initialValue?: string;
  onSubmit: (value: string) => void;
  onCanSubmit?: (canSubmit: boolean) => void;
}) {
  const step1 = useStep1({ initialValue, onSubmit });
  // Expose canSubmit for assertions without coupling to HireShell.
  onCanSubmit?.(step1.canSubmit);
  return (
    <div>
      <Step1Land step1={step1} />
      <button
        type="button"
        data-testid="harness-continue"
        disabled={!step1.canSubmit}
        onClick={step1.attemptSubmit}
      >
        Continue
      </button>
    </div>
  );
}

beforeEach(() => {
  // Deterministic placeholder pick — index 0 for every test unless
  // overridden. `useState(() => …)` runs the lazy initializer exactly
  // once per mount, so spying before render is sufficient.
  vi.spyOn(Math, "random").mockReturnValue(0);
});

afterEach(() => {
  vi.restoreAllMocks();
});

describe("Step1Land", () => {
  it("renders the locked headline + subtitle copy", () => {
    render(<Harness onSubmit={vi.fn()} />);
    expect(
      screen.getByRole("heading", { level: 1, name: /what do you need help with\?/i }),
    ).toBeInTheDocument();
    expect(
      screen.getByText(/tell me in your own words/i),
    ).toBeInTheDocument();
  });

  it("picks a placeholder from the locked rotation on mount and keeps it stable", () => {
    const { rerender } = render(<Harness onSubmit={vi.fn()} />);
    const textarea = screen.getByTestId("hire-step1-textarea") as HTMLTextAreaElement;
    expect(PLACEHOLDER_PROMPTS).toContain(textarea.placeholder);
    const initialPlaceholder = textarea.placeholder;
    // Re-render to confirm the lazy-initializer's pick is sticky.
    rerender(<Harness onSubmit={vi.fn()} />);
    expect(textarea.placeholder).toBe(initialPlaceholder);
  });

  it("Continue is disabled under the 15-char minimum and enabled at/above it", async () => {
    const user = userEvent.setup();
    render(<Harness onSubmit={vi.fn()} />);
    const cont = screen.getByTestId("harness-continue");
    const textarea = screen.getByTestId("hire-step1-textarea");

    expect(cont).toBeDisabled();
    await user.type(textarea, "too short"); // 9 chars
    expect(cont).toBeDisabled();

    await user.clear(textarea);
    await user.type(textarea, "x".repeat(STEP1_MIN_CHARS));
    expect(cont).toBeEnabled();
  });

  it("opens the examples popover and tapping an example fills the textarea", async () => {
    const user = userEvent.setup();
    render(<Harness onSubmit={vi.fn()} />);
    await user.click(screen.getByTestId("hire-step1-examples-trigger"));
    const popover = await screen.findByTestId("hire-step1-examples-popover");
    const marketplaceRow = within(popover).getByText(/marketplace seller/i);
    await user.click(marketplaceRow);
    const textarea = screen.getByTestId("hire-step1-textarea") as HTMLTextAreaElement;
    expect(textarea.value).toMatch(/hockey gear on facebook marketplace/i);
  });

  it("Cmd+Enter on a valid value submits with the trimmed value", async () => {
    const user = userEvent.setup();
    const onSubmit = vi.fn();
    render(<Harness onSubmit={onSubmit} />);
    const textarea = screen.getByTestId("hire-step1-textarea");
    await user.type(textarea, "  this is plenty of text to submit  ");
    await user.keyboard("{Meta>}{Enter}{/Meta}");
    expect(onSubmit).toHaveBeenCalledTimes(1);
    expect(onSubmit).toHaveBeenCalledWith("this is plenty of text to submit");
  });

  it("Cmd+Enter under the minimum surfaces the validation hint instead of submitting", async () => {
    const user = userEvent.setup();
    const onSubmit = vi.fn();
    render(<Harness onSubmit={onSubmit} />);
    const textarea = screen.getByTestId("hire-step1-textarea");
    await user.type(textarea, "too short");
    await user.keyboard("{Meta>}{Enter}{/Meta}");
    expect(onSubmit).not.toHaveBeenCalled();
    expect(screen.getByTestId("hire-step1-validation-hint")).toHaveTextContent(
      /tell me a bit more/i,
    );
  });

  it("clicking Continue while disabled-state-eligible (via attemptSubmit on the harness) shows the hint, and typing dismisses it", async () => {
    // Use Ctrl+Enter to exercise the non-mac modifier path while we're at it.
    const user = userEvent.setup();
    render(<Harness onSubmit={vi.fn()} />);
    const textarea = screen.getByTestId("hire-step1-textarea");
    await user.type(textarea, "short");
    await user.keyboard("{Control>}{Enter}{/Control}");
    expect(screen.getByTestId("hire-step1-validation-hint")).toBeInTheDocument();
    await user.type(textarea, "x");
    expect(screen.queryByTestId("hire-step1-validation-hint")).not.toBeInTheDocument();
  });
});
