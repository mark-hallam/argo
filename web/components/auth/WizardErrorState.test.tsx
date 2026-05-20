import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { WizardErrorState } from "./WizardErrorState";

describe("WizardErrorState", () => {
  it("renders the draft-hydration headline with a reset action and back-to-team link", async () => {
    const onReset = vi.fn();
    const user = userEvent.setup();
    render(<WizardErrorState variant="draft-hydration" onReset={onReset} />);
    expect(screen.getByTestId("wizard-error-state-draft-hydration")).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: /couldn't restore your draft/i })).toBeInTheDocument();
    expect(screen.getByRole("link", { name: /back to team/i })).toHaveAttribute("href", "/team");
    await user.click(screen.getByTestId("wizard-error-state-reset"));
    expect(onReset).toHaveBeenCalledTimes(1);
  });

  it("renders the worker-fetch variant with its own copy", () => {
    render(<WizardErrorState variant="worker-fetch" />);
    expect(screen.getByTestId("wizard-error-state-worker-fetch")).toBeInTheDocument();
    expect(
      screen.getByRole("heading", { name: /couldn't load your new teammate/i }),
    ).toBeInTheDocument();
  });

  it("surfaces a caller-supplied errorMessage for debuggability", () => {
    render(
      <WizardErrorState
        variant="draft-hydration"
        errorMessage="QuotaExceededError: localStorage full"
      />,
    );
    expect(screen.getByTestId("wizard-error-state-detail")).toHaveTextContent(
      /localStorage full/,
    );
  });
});
