import { render, screen, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  ApprovalsKpiStrip,
  buildApprovalsKpiCells,
  computeApprovalsKpis,
} from "./ApprovalsKpiStrip";
import type { ApprovalItem } from "@/lib/sample-approvals";

const NOW = new Date("2026-05-28T12:00:00Z").getTime();

function item(over: Partial<ApprovalItem>): ApprovalItem {
  return {
    id: over.id ?? "x",
    worker: { name: "Worker" },
    verb: "wants to",
    detail: "detail",
    consequenceIfYes: "yes",
    consequenceIfNo: "no",
    channel: "slack",
    receivedAt: new Date(NOW - 60_000).toISOString(),
    ...over,
  };
}

describe("ApprovalsKpiStrip", () => {
  beforeEach(() => {
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
  });

  it("renders exactly the five canonical labels in the canonical order", () => {
    render(<ApprovalsKpiStrip kpis={computeApprovalsKpis([], NOW)} />);
    const cells = screen.getAllByTestId(/^kpi-/);
    const orderedLabels = cells.map((el) => {
      const label = el.firstElementChild?.textContent ?? "";
      return label.trim();
    });
    expect(orderedLabels).toEqual([
      "Pending",
      "$ at risk",
      "Median time",
      "Approval rate",
      "Active reviewers",
    ]);
  });

  it("renders the empty/zero state with honest placeholders, never the old hard-coded baseline", () => {
    render(<ApprovalsKpiStrip kpis={computeApprovalsKpis([], NOW)} />);
    expect(screen.getByTestId("kpi-pending")).toHaveTextContent("0");
    expect(screen.getByTestId("kpi-pending")).toHaveTextContent(
      "active human decisions",
    );
    expect(screen.getByTestId("kpi-at-risk")).toHaveTextContent("—");
    expect(screen.getByTestId("kpi-at-risk")).toHaveTextContent(
      "no monetary exposure",
    );
    expect(screen.getByTestId("kpi-median-time")).toHaveTextContent("—");
    expect(screen.getByTestId("kpi-median-time")).toHaveTextContent(
      "no pending approvals",
    );
    expect(screen.getByTestId("kpi-approval-rate")).toHaveTextContent("—");
    expect(screen.getByTestId("kpi-approval-rate")).toHaveTextContent(
      "no resolved approvals",
    );
    expect(screen.getByTestId("kpi-active-reviewers")).toHaveTextContent("—");
    expect(screen.getByTestId("kpi-active-reviewers")).toHaveTextContent(
      "reviewer roster pending",
    );
    expect(screen.queryByText(/78% approved/i)).not.toBeInTheDocument();
    expect(screen.queryByText(/Resolved \/ wk/i)).not.toBeInTheDocument();
    expect(screen.queryByText(/Avg response/i)).not.toBeInTheDocument();
  });

  it("renders the populated state with derived numbers across all 5 cells", () => {
    const items = [
      item({
        id: "approved",
        decision: "approve",
        receivedAt: new Date(NOW - 10 * 60_000).toISOString(),
      }),
      item({
        id: "refused",
        decision: "refuse",
        receivedAt: new Date(NOW - 30 * 60_000).toISOString(),
      }),
      item({
        id: "pending-a",
        decision: null,
        dollarAtRisk: 1200,
        receivedAt: new Date(NOW - 12 * 60_000).toISOString(),
      }),
      item({
        id: "pending-b",
        decision: null,
        dollarAtRisk: 800,
        receivedAt: new Date(NOW - 4 * 60_000).toISOString(),
      }),
    ];

    render(<ApprovalsKpiStrip kpis={computeApprovalsKpis(items, NOW)} />);

    expect(screen.getByTestId("kpi-pending")).toHaveTextContent("2");
    expect(screen.getByTestId("kpi-at-risk")).toHaveTextContent("$2,000");
    expect(screen.getByTestId("kpi-at-risk")).toHaveTextContent(
      "across 2 pending",
    );
    expect(screen.getByTestId("kpi-median-time")).toHaveTextContent("8m");
    expect(screen.getByTestId("kpi-approval-rate")).toHaveTextContent("50%");
    expect(screen.getByTestId("kpi-approval-rate")).toHaveTextContent(
      "2 resolved approvals",
    );
    expect(screen.getByTestId("kpi-active-reviewers")).toHaveTextContent("—");
  });

  it("flags the Pending cell with destructive tone when a pending row is over SLA", () => {
    const cells = buildApprovalsKpiCells(
      computeApprovalsKpis(
        [
          item({
            id: "over",
            decision: null,
            slaExpiresAt: new Date(NOW - 60_000).toISOString(),
          }),
        ],
        NOW,
      ),
    );
    expect(cells[0]?.tone).toBe("bad");
  });

  it("flags the Pending cell with destructive tone when a pending row is severity p0", () => {
    const cells = buildApprovalsKpiCells(
      computeApprovalsKpis(
        [item({ id: "p0", decision: null, severity: "p0" })],
        NOW,
      ),
    );
    expect(cells[0]?.tone).toBe("bad");
  });

  it("scoped at-risk sum to pending rows only — resolved dollar amounts do not inflate the count", () => {
    const cells = buildApprovalsKpiCells(
      computeApprovalsKpis(
        [
          item({ id: "approved", decision: "approve", dollarAtRisk: 9999 }),
          item({ id: "pending", decision: null, dollarAtRisk: 500 }),
        ],
        NOW,
      ),
    );
    const atRisk = cells.find((c) => c.testId === "at-risk")!;
    expect(atRisk.value).toMatch(/\$500/);
  });

  it("does not render the legacy approval-rate ring on the strip itself", () => {
    render(
      <ApprovalsKpiStrip
        kpis={computeApprovalsKpis(
          [
            item({ id: "a", decision: "approve" }),
            item({ id: "b", decision: "refuse" }),
          ],
          NOW,
        )}
      />,
    );
    expect(screen.queryByTestId("kpi-rate-ring")).not.toBeInTheDocument();
  });

  it("converges the same 5 cell shape from any sample of items (root inbox === ops dashboard)", () => {
    const items = [
      item({ id: "a", decision: "approve" }),
      item({ id: "b", decision: null, dollarAtRisk: 250 }),
    ];
    const inboxKpis = computeApprovalsKpis(items, NOW);
    const dashboardKpis = computeApprovalsKpis(items, NOW);
    const inboxCells = buildApprovalsKpiCells(inboxKpis);
    const dashboardCells = buildApprovalsKpiCells(dashboardKpis);
    expect(inboxCells.map((c) => c.testId)).toEqual(
      dashboardCells.map((c) => c.testId),
    );
    expect(inboxCells.map((c) => c.label)).toEqual(
      dashboardCells.map((c) => c.label),
    );
  });

  it("renders the canonical labels regardless of which strip section the cell falls under (no orphan strip)", () => {
    const { container } = render(
      <ApprovalsKpiStrip kpis={computeApprovalsKpis([], NOW)} />,
    );
    // Should produce exactly one strip section, exactly five cells in it.
    const strip = within(container).getByTestId("approvals-kpi-strip");
    expect(within(strip).getAllByTestId(/^kpi-(?!rate-ring)/)).toHaveLength(5);
  });
});
