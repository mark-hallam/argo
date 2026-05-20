import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { SeverityPill } from "./SeverityPill";

describe("SeverityPill", () => {
  it("renders the P0 label with the destructive tone token", () => {
    render(<SeverityPill severity="p0" />);
    const pill = screen.getByTestId("severity-pill-p0");
    expect(pill).toHaveTextContent("P0");
    expect(pill).toHaveAttribute("data-severity", "p0");
    expect(pill.className).toMatch(/var\(--destructive\)/);
  });

  it("renders the P1 label with the warning tone token", () => {
    render(<SeverityPill severity="p1" />);
    const pill = screen.getByTestId("severity-pill-p1");
    expect(pill).toHaveTextContent("P1");
    expect(pill.className).toMatch(/var\(--warning\)/);
  });

  it("renders the P2 label with the muted tone token", () => {
    render(<SeverityPill severity="p2" />);
    const pill = screen.getByTestId("severity-pill-p2");
    expect(pill).toHaveTextContent("P2");
    expect(pill.className).toMatch(/--muted-foreground/);
  });
});
