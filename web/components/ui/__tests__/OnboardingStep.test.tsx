import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { OnboardingStep } from "../OnboardingStep";

describe("OnboardingStep", () => {
  it("renders progress, headline, body, and actions", () => {
    render(
      <OnboardingStep
        totalSteps={3}
        currentStep={2}
        headline="Connect your inbox"
        body="We'll send a magic link to confirm."
        actions={<button>Next</button>}
      />,
    );
    const progress = screen.getByRole("progressbar");
    expect(progress).toHaveAttribute("aria-valuenow", "2");
    expect(progress).toHaveAttribute("aria-valuemax", "3");
    expect(screen.getByRole("heading", { name: "Connect your inbox" })).toBeInTheDocument();
    expect(screen.getByText(/magic link/i)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Next" })).toBeInTheDocument();
  });
});
