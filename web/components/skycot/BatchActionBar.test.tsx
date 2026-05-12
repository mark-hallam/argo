import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { BatchActionBar } from "./BatchActionBar";
import type { ApprovalGroup } from "@/lib/similarity/group-approvals";
import type { ApprovalItem } from "@/lib/sample-approvals";

function mkItem(id: string): ApprovalItem {
  return {
    id,
    worker: { name: "Mariah" },
    verb: "wants to send this listing",
    detail: "Bauer Vapor X5 reply yes",
    consequenceIfYes: "goes to Marketplace.",
    consequenceIfNo: "I'll wait.",
    channel: "slack",
    receivedAt: new Date("2026-05-12T10:00:00Z").toISOString(),
  };
}

function mkGroup(n = 5): ApprovalGroup {
  return {
    key: "Mariah|marketplace-reply|bauer reply vapor",
    worker: { name: "Mariah" },
    actionType: "marketplace-reply",
    items: Array.from({ length: n }, (_, i) => mkItem(`ap_${i}`)),
    reason: `${n} similar replies`,
  };
}

describe("BatchActionBar", () => {
  it("renders banner with worker name + plural noun + counts", () => {
    render(
      <BatchActionBar
        group={mkGroup(5)}
        phase="idle"
        onApproveAll={vi.fn()}
        onReviewEach={vi.fn()}
      />,
    );
    expect(screen.getByText("Mariah proposed 5 similar replies")).toBeInTheDocument();
    expect(screen.getByText(/Approve all 5/)).toBeInTheDocument();
    expect(screen.getByText("Review each")).toBeInTheDocument();
  });

  it("fires onApproveAll when the primary action is tapped", async () => {
    const onApproveAll = vi.fn();
    const user = userEvent.setup();
    render(
      <BatchActionBar
        group={mkGroup(3)}
        phase="idle"
        onApproveAll={onApproveAll}
        onReviewEach={vi.fn()}
      />,
    );
    await user.click(screen.getByTestId("batch-approve-all-Mariah|marketplace-reply|bauer reply vapor"));
    expect(onApproveAll).toHaveBeenCalledTimes(1);
  });

  it("fires onReviewEach when the secondary action is tapped", async () => {
    const onReviewEach = vi.fn();
    const user = userEvent.setup();
    render(
      <BatchActionBar
        group={mkGroup(3)}
        phase="idle"
        onApproveAll={vi.fn()}
        onReviewEach={onReviewEach}
      />,
    );
    await user.click(screen.getByTestId("batch-review-each-Mariah|marketplace-reply|bauer reply vapor"));
    expect(onReviewEach).toHaveBeenCalledTimes(1);
  });

  it("shows a loading countdown during the parallel-fire phase", () => {
    render(
      <BatchActionBar
        group={mkGroup(5)}
        phase="loading"
        completedCount={2}
        failedCount={0}
        onApproveAll={vi.fn()}
        onReviewEach={vi.fn()}
      />,
    );
    // 5 - 2 - 0 = 3 left.
    expect(screen.getByText(/Approving 3/)).toBeInTheDocument();
    expect(screen.getByText(/2 of 5 settled/)).toBeInTheDocument();
    // Idle buttons are gone.
    expect(screen.queryByText(/Approve all 5/)).not.toBeInTheDocument();
  });

  it("shows partial-failure summary + retry button after settle", async () => {
    const onRetryFailed = vi.fn();
    const user = userEvent.setup();
    render(
      <BatchActionBar
        group={mkGroup(5)}
        phase="settled"
        completedCount={4}
        failedCount={1}
        onApproveAll={vi.fn()}
        onReviewEach={vi.fn()}
        onRetryFailed={onRetryFailed}
      />,
    );
    expect(screen.getByText(/Approved 4 — 1 failed/)).toBeInTheDocument();
    await user.click(screen.getByTestId("batch-retry-Mariah|marketplace-reply|bauer reply vapor"));
    expect(onRetryFailed).toHaveBeenCalledTimes(1);
  });
});
