import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { ApprovalCard } from "../ApprovalCard";

describe("ApprovalCard", () => {
  it("renders pending state with Approve + Deny buttons that invoke handlers", async () => {
    const onApprove = vi.fn();
    const onDeny = vi.fn();
    const user = userEvent.setup();
    render(
      <ApprovalCard
        title="Send refund to user 42"
        summary="$129 to card ending 4242"
        requestedBy="alice@"
        requestedAt="2026-05-16T10:00:00Z"
        onApprove={onApprove}
        onDeny={onDeny}
      />,
    );
    expect(screen.getByText("Send refund to user 42")).toBeInTheDocument();
    await user.click(screen.getByTestId("approval-approve"));
    await user.click(screen.getByTestId("approval-deny"));
    expect(onApprove).toHaveBeenCalledOnce();
    expect(onDeny).toHaveBeenCalledOnce();
  });

  it("hides action buttons and shows a status pill when state is finalized", () => {
    render(<ApprovalCard title="x" state="approved" />);
    expect(screen.queryByTestId("approval-approve")).toBeNull();
    expect(screen.queryByTestId("approval-deny")).toBeNull();
    expect(screen.getByText("Approved")).toBeInTheDocument();
  });
});
