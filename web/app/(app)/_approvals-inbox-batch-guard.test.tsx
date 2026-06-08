import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { ApprovalItem } from "@/lib/sample-approvals";

/**
 * Re-entrancy guard for batch "Approve all".
 *
 * The page fires N parallel decisions; the single-card path was guarded
 * but the batch path could double-RECORD on a retry / double-click. The
 * fix adds a synchronous per-group in-flight latch (a ref Set) checked at
 * the top of `handleApproveAll`, so two rapid clicks — fired before React
 * commits the "loading" state that would hide the button — still only
 * launch ONE `batchSubmit.submit`.
 *
 * This test renders the real inbox shell with the heavy hooks mocked and
 * fires two synchronous "Approve all" clicks, asserting `submit` runs once.
 */

const submitMock = vi.fn();

vi.mock("@/lib/hooks/use-approvals", () => ({
  useApprovals: () => ({
    data: GROUP_ITEMS,
    dataSource: "live",
    isPending: false,
    liveStatus: "connected",
    lastLiveEventAt: null,
  }),
  useSubmitDecision: () => ({ mutate: vi.fn(), isPending: false }),
  useUndoDecision: () => ({ mutate: vi.fn(), isPending: false }),
  useBatchSubmitDecisions: () => ({
    submit: submitMock,
    reset: vi.fn(),
    state: { isPending: false, total: 0, completed: 0, failed: 0, outcomes: [] },
  }),
}));

vi.mock("@/lib/hooks/use-approval-bundles", () => ({
  useApprovalBundles: () => ({ data: undefined, isPending: false }),
  useDecideBundle: () => ({ mutate: vi.fn(), isPending: false }),
}));

vi.mock("@/lib/hooks/use-redaction-policy", async () => {
  const { POLICY_SHOW_ALL } = await import("@/lib/redaction/redact");
  return { useRedactionPolicy: () => POLICY_SHOW_ALL };
});

vi.mock("@/lib/hooks/use-notifications", () => ({
  useUnreadNotificationCount: () => 0,
}));

vi.mock("@/lib/hooks/use-approvals-kpis", () => ({
  useApprovalsKpis: () => ({
    pending: 3,
    dollarAtRisk: 0,
    currency: "USD",
    activeReviewers: 1,
    p0: 0,
    p1: 0,
    p2: 3,
    approvalRate: null,
  }),
}));

// Next navigation hooks the shell awaits — minimal stand-ins.
vi.mock("next/navigation", () => ({
  useRouter: () => ({ replace: vi.fn(), push: vi.fn() }),
  usePathname: () => "/approvals",
  useSearchParams: () => new URLSearchParams(),
}));

// Three near-identical approvals from the same worker cluster into one
// batch group (SIMILARITY_THRESHOLD = 3), so the "Approve all" affordance
// renders.
const TODAY = new Date().toISOString();
function mkItem(id: string): ApprovalItem {
  return {
    id,
    worker: { name: "Mariah" },
    verb: "wants to send this listing reply",
    detail: "Bauer Vapor X5 — reply yes to the buyer",
    consequenceIfYes: "the reply goes to Marketplace.",
    consequenceIfNo: "it stays queued.",
    channel: "slack",
    receivedAt: TODAY,
    decision: null,
  };
}
const GROUP_ITEMS: ApprovalItem[] = [mkItem("ap_1"), mkItem("ap_2"), mkItem("ap_3")];

import ApprovalsInboxPage from "./_approvals-inbox-page";

describe("ApprovalsInboxPage — batch Approve-all re-entrancy", () => {
  beforeEach(() => {
    // A slow submit keeps the batch "in flight" across the second click,
    // so the synchronous latch is what must block the duplicate.
    submitMock.mockImplementation(
      () => new Promise((resolve) => setTimeout(() => resolve([]), 20)),
    );
  });
  afterEach(() => {
    submitMock.mockReset();
    vi.clearAllMocks();
  });

  it("fires batchSubmit.submit ONCE for two rapid Approve-all clicks", async () => {
    render(<ApprovalsInboxPage />);

    // Find the grouped "Approve all" button (testid is suffixed by the
    // group's stable key).
    const approveAll = await waitFor(() => {
      const btn = screen
        .getAllByTestId(/^batch-approve-all-/)
        .find((el) => el.textContent?.includes("Approve all"));
      if (!btn) throw new Error("Approve all button not rendered yet");
      return btn;
    });

    // Two synchronous clicks in the same tick — the double-click race.
    fireEvent.click(approveAll);
    fireEvent.click(approveAll);

    await waitFor(() => {
      expect(submitMock).toHaveBeenCalledTimes(1);
    });

    // The single submit carried all three decisions (one batch, not two).
    const [requests] = submitMock.mock.calls[0];
    expect(requests).toHaveLength(3);
  });
});
