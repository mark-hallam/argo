import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { FeatureSection } from "../FeatureSection";

describe("FeatureSection", () => {
  it("renders headline, body, eyebrow, and media slots", () => {
    render(
      <FeatureSection
        eyebrow="Observability"
        headline="See every step"
        body="Each agent action is captured to the audit log."
        media={<img alt="screenshot" src="/x.png" />}
      />,
    );
    expect(screen.getByRole("heading", { name: "See every step" })).toBeInTheDocument();
    expect(screen.getByText("Observability")).toBeInTheDocument();
    expect(screen.getByText(/audit log/i)).toBeInTheDocument();
    expect(screen.getByRole("img", { name: "screenshot" })).toBeInTheDocument();
  });

  it("sets data-reverse when reverse=true", () => {
    render(<FeatureSection headline="x" reverse />);
    expect(screen.getByTestId("feature-section")).toHaveAttribute("data-reverse", "true");
  });
});
