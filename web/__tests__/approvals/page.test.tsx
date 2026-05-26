import { render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { SAMPLE_APPROVALS, type ApprovalItem } from "@/lib/sample-approvals";

const useApprovalsMock = vi.fn<() => {
  data: ApprovalItem[] | undefined;
  isPending: boolean;
}>();

vi.mock("@/lib/hooks/use-approvals", () => ({
  useApprovals: () => useApprovalsMock(),
}));

import ApprovalsLandingPage from "@/app/(app)/approvals/page";

describe("/approvals landing page", () => {
  beforeEach(() => {
    useApprovalsMock.mockReset();
  });

  it("renders the ops-dashboard even with zero queued approvals", () => {
    // Per `fix-approvals-inbox-waiting-on-you-fixture-drift`, /approvals
    // must always render the locked E-ops-dashboard (5-KPI strip,
    // severity tiers, donuts, ring). The dashboard handles the 0-state
    // gracefully via "—" placeholders + "No <tier> severity approvals
    // waiting" per-tier rows; the previous EmptyState short-circuit
    // hid all of that and made /team and /approvals disagree about
    // pending counts.
    useApprovalsMock.mockReturnValue({ data: [], isPending: false });

    render(<ApprovalsLandingPage />);

    expect(screen.getByTestId("approvals-landing")).toBeInTheDocument();
    expect(
      screen.getByRole("heading", { level: 1, name: /approvals/i }),
    ).toBeInTheDocument();
    expect(screen.getByTestId("approvals-ops-dashboard")).toBeInTheDocument();
    expect(screen.getByTestId("kpi-pending")).toHaveTextContent("0");
    expect(screen.getByTestId("severity-tier-p0")).toBeInTheDocument();
    expect(screen.getByTestId("severity-tier-p1")).toBeInTheDocument();
    expect(screen.getByTestId("severity-tier-p2")).toBeInTheDocument();
    // The dashboard is the page a viewer sees, not the EmptyState.
    expect(screen.queryByTestId("approvals-empty")).not.toBeInTheDocument();
    expect(
      screen.queryByRole("heading", { name: /no approvals waiting/i }),
    ).not.toBeInTheDocument();
  });

  it("renders the ops-dashboard when pending approvals exist", () => {
    useApprovalsMock.mockReturnValue({ data: SAMPLE_APPROVALS, isPending: false });

    render(<ApprovalsLandingPage />);

    expect(screen.getByTestId("approvals-ops-dashboard")).toBeInTheDocument();
    expect(screen.getByTestId("approvals-kpi-strip")).toBeInTheDocument();
    expect(screen.getByTestId("kpi-pending")).toHaveTextContent("Pending");
    expect(screen.getByTestId("kpi-at-risk")).toHaveTextContent("$ at risk");
    expect(screen.getByTestId("kpi-active-reviewers")).toHaveTextContent(
      "Active reviewers",
    );
    expect(screen.getByTestId("severity-tier-p0")).toHaveTextContent("High");
    expect(screen.getByTestId("severity-tier-p1")).toHaveTextContent("Medium");
    expect(screen.getByTestId("severity-tier-p2")).toHaveTextContent("Low");
    expect(screen.getByTestId("at-risk-donut")).toBeInTheDocument();
    expect(screen.getAllByTestId("confidence-donut").length).toBeGreaterThan(1);
    // SAMPLE_APPROVALS carries 3 resolved rows (decision != null), so the
    // ring renders the 7-day approval rate rather than the empty placeholder.
    expect(screen.getByTestId("approval-rate-ring")).toBeInTheDocument();
    expect(screen.queryByTestId("approval-rate-empty")).not.toBeInTheDocument();
    expect(
      screen.getByRole("button", {
        name: /Nori wants to apply a 32% discount/i,
      }),
    ).toBeInTheDocument();
    expect(screen.queryByTestId("approvals-empty")).not.toBeInTheDocument();
  });

  it("mounts without throwing on a fresh workspace (no providers required)", () => {
    useApprovalsMock.mockReturnValue({ data: [], isPending: false });

    expect(() => render(<ApprovalsLandingPage />)).not.toThrow();
  });
});
