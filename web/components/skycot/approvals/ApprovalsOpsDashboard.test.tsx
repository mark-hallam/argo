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

// The two inline-decide tests (approve/refuse) click the row buttons and assert
// the success toast fires. The toast moved from fire-and-forget at click to the
// mutation's `onSuccess` (NO-THEATER day-1 unblock FIX 2 — see the component),
// so the `useSubmitDecision` mutation must RESOLVE for the toast to fire. jsdom
// has no gateway, so the real `submitDecision` fetch rejects and the toast never
// fired — this is why these two tests were red on main. Stub the network calls
// to resolve with no `execution` field, which lands on the generic
// "Approved"/"Refused" + Undo toast branch. Every other export (types,
// fetchApprovals, KPI helpers) stays real, so the render/KPI tests — which pass
// `serverApprovalRate` as a prop and never fetch — are unaffected.
vi.mock("@/lib/api/approvals", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@/lib/api/approvals")>();
  return {
    ...actual,
    submitDecision: vi.fn().mockResolvedValue({}),
    submitUndo: vi.fn().mockResolvedValue({}),
  };
});

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

  it("renders the seeded SAMPLE_APPROVALS as 6 pending + 3 resolved (ring ≈ 67%)", () => {
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

    // 4 of the 6 pending rows carry an approver (Mark x3, Aisha x1) →
    // 2 distinct active reviewers in the KPI strip.
    expect(screen.getByTestId("kpi-active-reviewers")).toHaveTextContent("2");
    expect(screen.getByTestId("kpi-active-reviewers")).toHaveTextContent(
      "reviewing now",
    );

    // The 6 remaining rows stay actionable in the inbox tiers.
    const p0Rows = screen.getByTestId("severity-tier-p0").querySelectorAll("li");
    const p1Rows = screen.getByTestId("severity-tier-p1").querySelectorAll("li");
    const p2Rows = screen.getByTestId("severity-tier-p2").querySelectorAll("li");
    expect(p0Rows.length + p1Rows.length + p2Rows.length).toBe(6);
  });

  it("navigates pending approval rows to audit action detail using item.actionId", async () => {
    const user = userEvent.setup();
    renderWithClient(
      <ApprovalsOpsDashboard
        items={[
          item({
            id: "ap_01H9Z7",
            actionId: "act_019e6785-928a-7270-a766-f945b6cd7161_018",
          }),
        ]}
      />,
    );

    const row = screen.getByTestId("recent-approval-ap_01H9Z7");
    expect(row).toHaveAttribute(
      "data-action-id",
      "act_019e6785-928a-7270-a766-f945b6cd7161_018",
    );

    await user.click(row);

    expect(assignMock).toHaveBeenCalledWith(
      "/audit/action/act_019e6785-928a-7270-a766-f945b6cd7161_018",
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

  it("seeds all 9 SAMPLE_APPROVALS rows with an explicit actionId (no hardcoded map fallback)", () => {
    for (const row of SAMPLE_APPROVALS) {
      expect(row.actionId, `seed row ${row.id} missing actionId`).toMatch(/^act_/);
    }
  });

  it("renders a channel chip per pending row matching the row's channel field", () => {
    renderWithClient(
      <ApprovalsOpsDashboard
        items={[
          item({ id: "row-slack", channel: "slack" }),
          item({ id: "row-sms", channel: "sms" }),
          item({ id: "row-voice", channel: "voice" }),
        ]}
      />,
    );

    expect(screen.getAllByTestId("channel-chip-slack").length).toBeGreaterThanOrEqual(1);
    expect(screen.getAllByTestId("channel-chip-sms").length).toBeGreaterThanOrEqual(1);
    expect(screen.getAllByTestId("channel-chip-voice").length).toBeGreaterThanOrEqual(1);
  });

  it("summarises pending counts by channel under the KPI strip", () => {
    renderWithClient(
      <ApprovalsOpsDashboard
        items={[
          item({ id: "s1", channel: "slack" }),
          item({ id: "s2", channel: "slack" }),
          item({ id: "v1", channel: "voice" }),
          // Resolved rows don't contribute to the pending breakdown.
          item({ id: "resolved", channel: "web", decision: "approve" }),
        ]}
      />,
    );

    const summary = screen.getByTestId("pending-by-channel");
    expect(summary).toHaveTextContent(/Slack\s*2/);
    expect(summary).toHaveTextContent(/Voice\s*1/);
    expect(summary).not.toHaveTextContent(/Web/);
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

  it("renders the tier row as a width-filling flex layout, not a viewport-keyed fixed grid", () => {
    // Regression for the severity-tier card crush: the row used to be a
    // grid whose `xl:grid-cols-[minmax(0,1fr)_7.5rem_5rem_6.5rem_auto]`
    // template starved the content track to ~85px on /approvals (the
    // dashboard reserves a 320px aside, so at a 1280px viewport the row
    // only has ~640px and four fixed metadata tracks consumed it). The
    // fix makes the row a `flex flex-wrap` whose content block keeps
    // `flex-1 min-w-0 basis-*` so it fills the real container width,
    // with the SLA / $ / confidence / actions in a separate wrapping
    // meta cluster.
    renderWithClient(
      <ApprovalsOpsDashboard
        items={[item({ id: "tier-flex", severity: "p2" })]}
      />,
    );

    const row = screen.getByTestId("recent-approval-tier-flex");
    // Width-filling flex row, not the old crush-prone fixed grid.
    expect(row.className).toContain("flex");
    expect(row.className).toContain("flex-wrap");
    expect(row.className).not.toContain("grid-cols-");

    // Content block grows to fill the available width and can shrink.
    const col = screen.getByTestId("approval-content-col-tier-flex");
    expect(col.className).toContain("flex-1");
    expect(col.className).toContain("min-w-0");

    // Metadata + actions live in their own wrapping cluster so they no
    // longer occupy fixed tracks that starve the content column.
    const meta = screen.getByTestId("approval-meta-col-tier-flex");
    expect(meta.className).toContain("flex-wrap");
    expect(meta.querySelector('[data-testid="approval-row-actions-tier-flex"]')).not.toBeNull();

    // The body keeps `line-clamp-2` for clamping AND must NOT carry the
    // `block` utility, which would override line-clamp's `-webkit-box`
    // display and let a long detail run unbounded once the column fills.
    const body = col.querySelector("span.break-words");
    expect(body).not.toBeNull();
    expect(body?.className).toContain("line-clamp-2");
    expect((body?.className ?? "").split(/\s+/)).not.toContain("block");
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

  // Soak finding #2 (`fix-approvals-rate-kpi-durable-from-audit`): the
  // APPROVAL RATE KPI used to reset to "—" on reload because it derived
  // from the pending list, where resolved decisions have dropped out. When
  // the gateway supplies a durable `serverApprovalRate`, the KPI must show
  // the server value even though NO row in `items` carries a decision.
  it("renders the durable server approval rate when the pending list has no decided rows", () => {
    renderWithClient(
      <ApprovalsOpsDashboard
        items={[item({ id: "still-pending" })]}
        serverApprovalRate={{
          total: 8,
          approvedFraction: 0.75,
          refusedFraction: 0.25,
          expiredFraction: 0,
        }}
      />,
    );

    // Server-derived: 75%, 8 resolved — NOT "—", despite no decided item.
    expect(screen.getByTestId("kpi-approval-rate")).toHaveTextContent("75%");
    expect(screen.getByTestId("kpi-approval-rate")).toHaveTextContent(
      "8 resolved approvals",
    );
    // The aside ring renders (not the empty "—" placeholder).
    expect(screen.getByTestId("approval-rate-ring")).toHaveAttribute(
      "data-approved",
      "75",
    );
    expect(screen.queryByTestId("approval-rate-empty")).not.toBeInTheDocument();
  });

  // Back-compat: with the server field absent, the KPI keeps its client
  // compute over the pending list (the pre-brief behaviour, unchanged).
  it("falls back to the client compute when no server rate is supplied", () => {
    renderWithClient(
      <ApprovalsOpsDashboard items={[item({ id: "still-pending" })]} />,
    );

    expect(screen.getByTestId("kpi-approval-rate")).toHaveTextContent("—");
    expect(screen.getByTestId("kpi-approval-rate")).toHaveTextContent(
      "no resolved approvals",
    );
    expect(screen.getByTestId("approval-rate-empty")).toBeInTheDocument();
  });
});
