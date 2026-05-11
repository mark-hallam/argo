import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { ApprovalCard } from "./ApprovalCard";

function renderCard(overrides: Partial<React.ComponentProps<typeof ApprovalCard>> = {}) {
  const onDecide = vi.fn();
  const onUndo = vi.fn();
  render(
    <ApprovalCard
      worker={{ name: "Mariah" }}
      verb="wants to send this listing"
      detail="Bauer Vapor X5 · $90"
      consequenceIfYes="goes to Facebook Marketplace."
      consequenceIfNo="waits for direction."
      reason="you said 'always check counter-offers'."
      onDecide={onDecide}
      onUndo={onUndo}
      {...overrides}
    />,
  );
  return { onDecide, onUndo };
}

describe("ApprovalCard", () => {
  it("renders the load-bearing structure in order", () => {
    renderCard();
    expect(screen.getByText(/Mariah/)).toBeInTheDocument();
    expect(screen.getByText(/wants to send this listing/)).toBeInTheDocument();
    expect(screen.getByText(/Bauer Vapor X5/)).toBeInTheDocument();
    expect(screen.getByText(/goes to Facebook Marketplace/)).toBeInTheDocument();
    expect(screen.getByText(/waits for direction/)).toBeInTheDocument();
    expect(screen.getByTestId("approval-approve")).toBeInTheDocument();
    expect(screen.getByTestId("approval-refuse")).toBeInTheDocument();
    expect(screen.getByTestId("approval-why")).toBeInTheDocument();
  });

  it("calls onDecide on approve", async () => {
    const user = userEvent.setup();
    const { onDecide } = renderCard();
    await user.click(screen.getByTestId("approval-approve"));
    expect(onDecide).toHaveBeenCalledWith("approve");
  });

  it("calls onDecide on refuse", async () => {
    const user = userEvent.setup();
    const { onDecide } = renderCard();
    await user.click(screen.getByTestId("approval-refuse"));
    expect(onDecide).toHaveBeenCalledWith("refuse");
  });

  it("reveals the reason when Why? tapped", async () => {
    const user = userEvent.setup();
    renderCard();
    expect(screen.queryByText(/always check counter-offers/)).not.toBeInTheDocument();
    await user.click(screen.getByTestId("approval-why"));
    expect(screen.getByText(/always check counter-offers/)).toBeInTheDocument();
  });

  it("renders the post-decision bar when controlled decision is set", () => {
    renderCard({ decision: "approve" });
    expect(screen.getByText(/Approved/)).toBeInTheDocument();
    expect(screen.getByTestId("approval-undo")).toBeInTheDocument();
  });

  it("hides the Why button when no reason is provided", () => {
    renderCard({ reason: undefined });
    expect(screen.queryByTestId("approval-why")).not.toBeInTheDocument();
  });
});
