import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { KeyFact } from "../KeyFact";

describe("KeyFact", () => {
  it("renders value, label, and context", () => {
    render(<KeyFact value="92%" label="Approval rate" context="Last 30 days" />);
    expect(screen.getByText("92%")).toBeInTheDocument();
    expect(screen.getByText("Approval rate")).toBeInTheDocument();
    expect(screen.getByText("Last 30 days")).toBeInTheDocument();
    expect(screen.getByTestId("key-fact")).toBeInTheDocument();
  });
});
