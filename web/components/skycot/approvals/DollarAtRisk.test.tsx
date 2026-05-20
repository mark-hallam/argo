import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { DollarAtRisk } from "./DollarAtRisk";

describe("DollarAtRisk", () => {
  it("renders an em-dash when amount is omitted", () => {
    render(<DollarAtRisk />);
    expect(screen.getByTestId("dollar-at-risk-empty")).toHaveTextContent("—");
  });

  it("renders an em-dash for amount=0 (design-lock: no '$0')", () => {
    render(<DollarAtRisk amount={0} />);
    expect(screen.getByTestId("dollar-at-risk-empty")).toHaveTextContent("—");
  });

  it("renders an em-dash for null (gateway omitted)", () => {
    render(<DollarAtRisk amount={null} />);
    expect(screen.getByTestId("dollar-at-risk-empty")).toBeInTheDocument();
  });

  it("renders a currency-formatted amount when monetary impact is set", () => {
    render(<DollarAtRisk amount={48290} />);
    const el = screen.getByTestId("dollar-at-risk");
    expect(el).toHaveTextContent("$48,290");
    expect(el).toHaveAttribute("data-amount", "48290");
  });

  it("honors a non-default currency", () => {
    render(<DollarAtRisk amount={1200} currency="EUR" locale="en-GB" />);
    expect(screen.getByTestId("dollar-at-risk").textContent).toMatch(/1,200/);
  });
});
