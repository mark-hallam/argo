import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { ReactElement } from "react";
import { ApprovalsOpsDashboard } from "./ApprovalsOpsDashboard";
import { SAMPLE_APPROVALS, type ApprovalItem } from "@/lib/sample-approvals";

const NOW = new Date("2026-05-25T12:00:00Z").getTime();
const assignMock = vi.fn();
const originalLocation = window.location;

const toastSuccessMock = vi.fn();
vi.mock("sonner", () => ({
  toast: {
    success: (...args: unknown[]) => toastSuccessMock(...args),
    error: vi.fn(),
  },
}));

function renderWithClient(ui: ReactElement) {
  const client = new QueryClient({
    defaultOptions: {
      queries: { retry: false },
      mutations: { retry: false },
    },
  });
  return render(<QueryClientProvider client={client}>{ui}</QueryClientProvider>);
}

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
    toastSuccessMock.mockReset();
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
    renderWithClient(
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
      "no reviewers assigned",
    );
    expect(screen.getByTestId("at-risk-donut")).toHaveAttribute(
      "data-percent",
      "none",
    );
    expect(screen.getByTestId("approval-rate-empty")).toBeInTheDocument();
    expect(screen.queryByTestId("approval-rate-ring")).not.toBeInTheDocument();
  });

  it("derives approval-rate visuals from resolved approval rows", () => {
    renderWithClient(
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
    renderWithClient(<ApprovalsOpsDashboard items={[]} />);

    expect(screen.queryByText(/7-day baseline/i)).not.toBeInTheDocument();
    expect(screen.queryByText(/78% approved/i)).not.toBeInTheDocument();
    expect(screen.queryByText(/14% refused/i)).not.toBeInTheDocument();
    expect(screen.queryByText(/8% expired/i)).not.toBeInTheDocument();
    expect(screen.queryByText(/reviewer roster pending/i)).not.toBeInTheDocument();
  });

  it("derives the Active reviewers KPI from distinct approvers on pending rows", () => {
    renderWithClient(
      <ApprovalsOpsDashboard
        items={[
          item({ id: "p1", approver: { name: "Mark" } }),
          item({ id: "p2", approver: { name: "Mark" } }),
          item({ id: "p3", approver: { name: "Aisha" } }),
          item({ id: "p4-unassigned" }),
          item({
            id: "decided",
            decision: "approve",
            approver: { name: "Priya" },
          }),
        ]}
      />,
    );

    expect(screen.getByTestId("kpi-active-reviewers")).toHaveTextContent("2");
    expect(screen.getByTestId("kpi-active-reviewers")).toHaveTextContent(
      "reviewing now",
    );
  });

  it("renders the seeded SAMPLE_APPROVALS as 5 pending + 3 resolved (ring ≈ 67%)", () => {
    renderWithClient(<ApprovalsOpsDashboard items={SAMPLE_APPROVALS} />);

    // 2 of the 3 resolved rows are approved → 67% headline.
    expect(screen.getByTestId("kpi-approval-rate")).toHaveTextContent("67%");
    expect(screen.getByTestId("kpi-approval-rate")).toHaveTextContent(
      "3 resolved approvals",
    );
    expect(screen.getByTestId("approval-rate-ring")).toHaveAttribute(
      "data-approved",
      "67",
    );
    expect(screen.getByTestId("approval-rate-ring")).toHaveAttribute(
      "data-refused",
      "33",
    );

    // 3 of the 5 pending rows carry an approver (Mark x2, Aisha x1) →
    // 2 distinct active reviewers in the KPI strip.
    expect(screen.getByTestId("kpi-active-reviewers")).toHaveTextContent("2");
    expect(screen.getByTestId("kpi-active-reviewers")).toHaveTextContent(
      "reviewing now",
    );

    // The 5 remaining rows stay actionable in the inbox tiers.
    const p0Rows = screen.getByTestId("severity-tier-p0").querySelectorAll("li");
    const p1Rows = screen.getByTestId("severity-tier-p1").querySelectorAll("li");
    const p2Rows = screen.getByTestId("severity-tier-p2").querySelectorAll("li");
    expect(p0Rows.length + p1Rows.length + p2Rows.length).toBe(5);
  });

  it("navigates pending approval rows to audit action detail using item.actionId", async () => {
    const user = userEvent.setup();
    renderWithClient(
      <ApprovalsOpsDashboard
        items={[
          item({
            id: "ap_01H9Z7",
            actionId: "act_018f67d0-0000-7000-8000-000000000001_018",
          }),
        ]}
      />,
    );

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

  it("renders a row without actionId as non-clickable and does not drill in", async () => {
    const user = userEvent.setup();
    const warnSpy = vi.spyOn(console, "warn").mockImplementation(() => {});
    renderWithClient(
      <ApprovalsOpsDashboard items={[item({ id: "no-action-id" })]} />,
    );

    const row = screen.getByTestId("recent-approval-no-action-id");
    expect(row).not.toHaveAttribute("role", "button");
    expect(row).not.toHaveAttribute("data-action-id");
    expect(row.className).toContain("cursor-default");
    expect(row.className).not.toContain("cursor-pointer");

    await user.click(row);
    expect(assignMock).not.toHaveBeenCalled();

    expect(warnSpy).toHaveBeenCalledWith(
      "[approvals] item no-action-id has no action_id — drill-in disabled",
    );
    warnSpy.mockRestore();
  });

  it("seeds all 8 SAMPLE_APPROVALS rows with an explicit actionId (no hardcoded map fallback)", () => {
    for (const row of SAMPLE_APPROVALS) {
      expect(row.actionId, `seed row ${row.id} missing actionId`).toMatch(/^act_/);
    }
  });

  it("renders inline Approve and Refuse buttons on every pending row", () => {
    renderWithClient(
      <ApprovalsOpsDashboard
        items={[
          item({ id: "row-p0", severity: "p0" }),
          item({ id: "row-p2", severity: "p2" }),
        ]}
      />,
    );

    expect(screen.getByTestId("approve-row-p0")).toBeInTheDocument();
    expect(screen.getByTestId("refuse-row-p0")).toBeInTheDocument();
    expect(screen.getByTestId("approve-row-p2")).toBeInTheDocument();
    expect(screen.getByTestId("refuse-row-p2")).toBeInTheDocument();
  });

  it("approves a row inline without firing the row drill-in", async () => {
    const user = userEvent.setup();
    renderWithClient(
      <ApprovalsOpsDashboard items={[item({ id: "ap_01H9Z7", severity: "p0" })]} />,
    );

    await user.click(screen.getByTestId("approve-ap_01H9Z7"));

    expect(assignMock).not.toHaveBeenCalled();
    await waitFor(() => {
      expect(toastSuccessMock).toHaveBeenCalled();
    });
    expect(toastSuccessMock.mock.calls[0]?.[0]).toBe("Approved");
    const opts = toastSuccessMock.mock.calls[0]?.[1] as
      | { action?: { label?: string } }
      | undefined;
    expect(opts?.action?.label).toBe("Undo");
  });

  it("does not collapse the content column when the body contains a long unbroken token (scene-6 F4)", () => {
    // Regression for scene-6 F4: without `flex-1 min-w-0` on the inner
    // content span (+ `break-words` on the body text), a long unbroken
    // word would force the column to shrink to roughly its char width,
    // leaving the title rendered as "W…" and the body wrapping
    // letter-by-letter. The fix gives the column a `flex-1 min-w-0`
    // basis and breaks long words mid-token.
    renderWithClient(
      <ApprovalsOpsDashboard
        items={[
          item({
            id: "long-word",
            severity: "p0",
            verb: "wants approval",
            detail:
              "reproductivity-of-the-customer-segment-across-the-same-API-surface",
          }),
        ]}
      />,
    );

    const col = screen.getByTestId("approval-content-col-long-word");
    // Inner content span carries the layout-critical classes.
    const inner = col.querySelector("span.min-w-0.flex-1");
    expect(inner).not.toBeNull();
    // Body text node has the wrap-anywhere class so long tokens break.
    const body = inner?.querySelector("span.break-words");
    expect(body).not.toBeNull();
    expect(body?.textContent ?? "").toContain(
      "reproductivity-of-the-customer-segment-across-the-same-API-surface",
    );
  });

  it("refuses a row inline and fires the Refused toast", async () => {
    const user = userEvent.setup();
    renderWithClient(
      <ApprovalsOpsDashboard items={[item({ id: "ap_01H9Z7", severity: "p1" })]} />,
    );

    await user.click(screen.getByTestId("refuse-ap_01H9Z7"));

    expect(assignMock).not.toHaveBeenCalled();
    await waitFor(() => {
      expect(toastSuccessMock).toHaveBeenCalled();
    });
    expect(toastSuccessMock.mock.calls[0]?.[0]).toBe("Refused");
  });
});
