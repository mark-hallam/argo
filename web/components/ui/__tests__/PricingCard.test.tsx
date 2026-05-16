import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { PricingCard } from "../PricingCard";

describe("PricingCard", () => {
  it("renders plan name, price, features, and CTA", () => {
    render(
      <PricingCard
        planName="Team"
        price="$99"
        cadence="/mo"
        description="For small operating teams."
        features={["Unlimited audits", "5 seats", "Email + chat support"]}
        cta={<button>Start trial</button>}
      />,
    );
    expect(screen.getByText("Team")).toBeInTheDocument();
    expect(screen.getByText("$99")).toBeInTheDocument();
    expect(screen.getByText("/mo")).toBeInTheDocument();
    expect(screen.getAllByRole("listitem")).toHaveLength(3);
    expect(screen.getByRole("button", { name: "Start trial" })).toBeInTheDocument();
  });

  it("applies the featured ring when featured=true", () => {
    render(
      <PricingCard
        planName="Pro"
        price="$299"
        features={["a"]}
        cta={<span />}
        featured
      />,
    );
    expect(screen.getByTestId("pricing-card")).toHaveAttribute("data-featured", "true");
  });
});
