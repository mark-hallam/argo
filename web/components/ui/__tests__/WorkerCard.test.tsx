import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { WorkerCard } from "../WorkerCard";

describe("WorkerCard", () => {
  it("renders worker name, role, status pill, and duty list", () => {
    render(
      <WorkerCard
        name="Receptionist"
        role="Front desk"
        status="working"
        duties={["Greet visitors", "Route incoming calls"]}
      />,
    );
    expect(screen.getByText("Receptionist")).toBeInTheDocument();
    expect(screen.getByText("Front desk")).toBeInTheDocument();
    expect(screen.getByText("Greet visitors")).toBeInTheDocument();
    expect(screen.getByTestId("worker-card")).toHaveAttribute("data-status", "working");
  });
});
