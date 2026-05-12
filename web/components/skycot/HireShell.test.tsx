import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { HireShell } from "./HireShell";

const pushMock = vi.fn();
const backMock = vi.fn();

vi.mock("next/navigation", () => ({
  useRouter: () => ({ push: pushMock, back: backMock, replace: vi.fn() }),
}));

beforeEach(() => {
  pushMock.mockReset();
  backMock.mockReset();
});

afterEach(() => {
  vi.clearAllMocks();
});

describe("HireShell", () => {
  it("renders progress dots with the current step highlighted", () => {
    render(
      <HireShell step={3}>
        <p>step content</p>
      </HireShell>,
    );
    const progressbar = screen.getByRole("progressbar");
    expect(progressbar).toHaveAttribute("aria-valuenow", "3");
    expect(progressbar).toHaveAttribute("aria-valuemax", "7");
    // Step 3 dot should be in "current" state.
    expect(screen.getByTestId("progress-dot-3").getAttribute("data-state")).toBe("current");
    expect(screen.getByTestId("progress-dot-2").getAttribute("data-state")).toBe("done");
    expect(screen.getByTestId("progress-dot-5").getAttribute("data-state")).toBe("upcoming");
  });

  it("disables Back on step 1 and routes via router.back() on later steps", async () => {
    const user = userEvent.setup();
    const { rerender } = render(
      <HireShell step={1}>
        <p>step 1</p>
      </HireShell>,
    );
    expect(screen.getByTestId("hire-shell-back")).toBeDisabled();

    rerender(
      <HireShell step={3}>
        <p>step 3</p>
      </HireShell>,
    );
    await user.click(screen.getByTestId("hire-shell-back"));
    expect(backMock).toHaveBeenCalledOnce();
  });

  it("opens the confirm dialog on escape when there is an unsaved draft", async () => {
    const user = userEvent.setup();
    const onDiscard = vi.fn();
    render(
      <HireShell step={2} unsavedHire onDiscard={onDiscard}>
        <p>step 2</p>
      </HireShell>,
    );
    expect(screen.queryByTestId("alert-dialog")).not.toBeInTheDocument();
    await user.click(screen.getByTestId("hire-shell-escape"));
    expect(await screen.findByTestId("alert-dialog")).toBeInTheDocument();
    expect(screen.getByText(/Discard your in-progress teammate/)).toBeInTheDocument();
    await user.click(screen.getByTestId("alert-dialog-confirm"));
    expect(onDiscard).toHaveBeenCalledOnce();
    expect(pushMock).toHaveBeenCalledWith("/");
  });

  it("does not confirm when there is no unsaved draft — exits directly to /", async () => {
    const user = userEvent.setup();
    const onDiscard = vi.fn();
    render(
      <HireShell step={1} onDiscard={onDiscard}>
        <p>step 1</p>
      </HireShell>,
    );
    await user.click(screen.getByTestId("hire-shell-escape"));
    expect(screen.queryByTestId("alert-dialog")).not.toBeInTheDocument();
    expect(pushMock).toHaveBeenCalledWith("/");
    expect(onDiscard).not.toHaveBeenCalled();
  });

  it("brand square triggers the same confirm-then-leave flow", async () => {
    const user = userEvent.setup();
    const onDiscard = vi.fn();
    render(
      <HireShell step={4} unsavedHire onDiscard={onDiscard}>
        <p>step 4</p>
      </HireShell>,
    );
    await user.click(screen.getByTestId("hire-shell-brand"));
    expect(await screen.findByTestId("alert-dialog")).toBeInTheDocument();
    await user.click(screen.getByTestId("alert-dialog-cancel"));
    expect(onDiscard).not.toHaveBeenCalled();
    expect(pushMock).not.toHaveBeenCalled();
  });

  it("Continue button is enabled only when onContinue is provided and not disabled", async () => {
    const user = userEvent.setup();
    const onContinue = vi.fn();
    const { rerender } = render(
      <HireShell step={2}>
        <p>placeholder</p>
      </HireShell>,
    );
    expect(screen.getByTestId("hire-shell-continue")).toBeDisabled();

    rerender(
      <HireShell step={2} onContinue={onContinue} continueDisabled>
        <p>placeholder</p>
      </HireShell>,
    );
    expect(screen.getByTestId("hire-shell-continue")).toBeDisabled();

    rerender(
      <HireShell step={2} onContinue={onContinue}>
        <p>placeholder</p>
      </HireShell>,
    );
    await user.click(screen.getByTestId("hire-shell-continue"));
    expect(onContinue).toHaveBeenCalledOnce();
  });
});
