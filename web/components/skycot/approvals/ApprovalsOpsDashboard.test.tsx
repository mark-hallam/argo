import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { ApprovalsOpsDashboard } from "./ApprovalsOpsDashboard";
import type { ApprovalItem } from "@/lib/sample-approvals";

const NOW = new Date("2026-05-25T12:00:00Z").getTime();
const assignMock = vi.fn();
const originalLocation = window.location;

function item(over: Partial<ApprovalItem>): ApprovalItem {
  return {
    id: over.id ?? "approval",
    worker: { name: "Mariah" },
    verb: "wants approval",
    detail: "detail",
    consequenceIfYes: "yes",
    consequenceIfNo: "no",
    channel: "slack",
    receivedAt: new Date(NOW - 60_000).toISOString(),
    ...over,
  };
}

describe("ApprovalsOpsDashboard", () => {
  beforeEach(() => {
    assignMock.mockReset();
    Object.defineProperty(window, "location", {
      configurable: true,
      value: {
        ...originalLocation,
        assign: assignMock,
      },
    });
    vi.stubGlobal(
      "matchMedia",
      vi.fn().mockReturnValue({
        matches: false,
        addEventListener: vi.fn(),
        removeEventListener: vi.fn(),
      }),
    );
  });

  afterEach(() => {
    vi.unstubAllGlobals();
    Object.defineProperty(window, "location", {
      configurable: true,
      value: originalLocation,
    });
  });

  it("shows honest placeholders when no resolved rows or reviewer roster exist", () => {
    render(
      <ApprovalsOpsDashboard
        items={[
          item({ id: "pending-money", dollarAtRisk: 1200 }),
          item({ id: "pending-normal" }),
        ]}
      />,
    );

    expect(screen.getByTestId("kpi-approval-rate")).toHaveTextContent("—");
    expect(screen.getByTestId("kpi-approval-rate")).toHaveTextContent(
      "no resolved approvals",
    );
    expect(screen.getByTestId("kpi-active-reviewers")).toHaveTextContent("—");
    expect(screen.getByTestId("kpi-active-reviewers")).toHaveTextContent(
      "reviewer roster pending",
    );
    expect(screen.getByTestId("at-risk-donut")).toHaveAttribute(
      "data-percent",
      "none",
    );
    expect(screen.getByTestId("approval-rate-empty")).toBeInTheDocument();
    expect(screen.queryByTestId("approval-rate-ring")).not.toBeInTheDocument();
  });

  it("derives approval-rate visuals from resolved approval rows", () => {
    render(
      <ApprovalsOpsDashboard
        items={[
          item({ id: "approved", decision: "approve" }),
          item({ id: "refused", decision: "refuse" }),
          item({ id: "pending", dollarAtRisk: 900 }),
        ]}
      />,
    );

    expect(screen.getByTestId("kpi-approval-rate")).toHaveTextContent("50%");
    expect(screen.getByTestId("kpi-approval-rate")).toHaveTextContent(
      "2 resolved approvals",
    );
    expect(screen.getByTestId("approval-rate-ring")).toHaveAttribute(
      "data-approved",
      "50",
    );
    expect(screen.getByTestId("approval-rate-ring")).toHaveAttribute(
      "data-refused",
      "50",
    );
    expect(screen.getByTestId("approval-rate-ring")).toHaveAttribute(
      "data-expired",
      "0",
    );
    expect(screen.getByText(/50% approved.*50% refused.*0% expired/)).toBeInTheDocument();
  });

  it("does not render the old fabricated baseline copy", () => {
    render(<ApprovalsOpsDashboard items={[]} />);

    expect(screen.queryByText(/7-day baseline/i)).not.toBeInTheDocument();
    expect(screen.queryByText(/78% approved/i)).not.toBeInTheDocument();
    expect(screen.queryByText(/14% refused/i)).not.toBeInTheDocument();
    expect(screen.queryByText(/8% expired/i)).not.toBeInTheDocument();
  });

  it("navigates pending approval rows to audit action detail", async () => {
    const user = userEvent.setup();
    render(<ApprovalsOpsDashboard items={[item({ id: "ap_01H9Z7" })]} />);

    const row = screen.getByTestId("recent-approval-ap_01H9Z7");
    expect(row).toHaveAttribute(
      "data-action-id",
      "act_018f67d0-0000-7000-8000-000000000001_018",
    );

    await user.click(row);

    expect(assignMock).toHaveBeenCalledWith(
      "/audit/action/act_018f67d0-0000-7000-8000-000000000001_018",
    );
  });
});
