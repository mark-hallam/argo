import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, within } from "@testing-library/react";
import { createElement, type ReactNode } from "react";
import { describe, expect, it } from "vitest";
import { ApprovalsOpsDashboard } from "@/components/skycot/approvals/ApprovalsOpsDashboard";
import type { ApprovalItem } from "@/lib/sample-approvals";

// ApprovalsOpsDashboard now calls useSubmitDecision() → useMutation,
// which needs a QueryClient in context. Wrap every render.
function renderWithQuery(ui: ReactNode) {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false, gcTime: 0 }, mutations: { retry: false } },
  });
  return render(createElement(QueryClientProvider, { client }, ui));
}

const NOW = new Date("2026-05-26T02:00:00Z").getTime();

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

describe("ApprovalsOpsDashboard metric cards", () => {
  it("renders $ at risk as a stat card when there is no real percentage", () => {
    renderWithQuery(
      <ApprovalsOpsDashboard
        items={[
          item({ id: "pending-money", dollarAtRisk: 1200 }),
          item({ id: "pending-normal" }),
          item({ id: "approved", decision: "approve" }),
        ]}
      />,
    );

    const card = screen.getByTestId("at-risk-donut");
    expect(card).toHaveAttribute("data-visual", "stat");
    expect(card).toHaveAttribute("data-percent", "none");
    expect(within(card).getByTestId("at-risk-donut-value")).toHaveTextContent(
      "$1,200",
    );
    expect(card).toHaveTextContent(
      "1 pending approval with monetary exposure in the current queue",
    );
    expect(card.querySelector('[style*="conic-gradient"]')).toBeNull();
    expect(card.querySelector(".rounded-full")).toBeNull();

    expect(screen.getByTestId("approval-rate-ring")).toBeInTheDocument();
    expect(screen.getAllByTestId("confidence-donut").length).toBeGreaterThan(0);
  });
});
