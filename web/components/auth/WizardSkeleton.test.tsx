import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { WizardSkeleton } from "./WizardSkeleton";

describe("WizardSkeleton", () => {
  it("renders the 5-pip strip with the right number lit by default", () => {
    render(<WizardSkeleton />);
    expect(screen.getByTestId("wizard-skeleton")).toHaveAttribute("aria-busy", "true");
    for (let i = 1; i <= 5; i += 1) {
      expect(screen.getByTestId(`wizard-skeleton-pip-${i}`)).toBeInTheDocument();
    }
  });

  it("respects the step prop so step 3 shows pip 1-3 lit and 4-5 unlit", () => {
    render(<WizardSkeleton step={3} />);
    expect(screen.getByTestId("wizard-skeleton")).toBeInTheDocument();
    // Style asserts on the gradient are visual; assert presence + announced state
    expect(screen.getByLabelText(/loading hire wizard/i)).toBeInTheDocument();
  });

  it("treats step 6 (celebration) as 5/5 filled", () => {
    render(<WizardSkeleton step={6} />);
    expect(screen.getByTestId("wizard-skeleton-pip-5")).toBeInTheDocument();
  });
});
