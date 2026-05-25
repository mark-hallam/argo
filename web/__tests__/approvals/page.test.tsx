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

  it("renders the inbox empty-state with both CTAs", () => {
    useApprovalsMock.mockReturnValue({ data: [], isPending: false });

    render(<ApprovalsLandingPage />);

    expect(screen.getByTestId("approvals-landing")).toBeInTheDocument();
    expect(
      screen.getByRole("heading", { level: 1, name: /approvals/i }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("heading", { name: /no approvals waiting/i }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("link", { name: /open the inbox/i }),
    ).toHaveAttribute("href", "/");
    expect(
      screen.getByRole("link", { name: /see your workers/i }),
    ).toHaveAttribute("href", "/workers");
    expect(screen.queryByTestId("approvals-ops-dashboard")).not.toBeInTheDocument();
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
    expect(screen.getAllByTestId("approval-rate-ring").length).toBeGreaterThan(0);
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
