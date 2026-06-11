/**
 * Day-1 unblock FIX 2 — approval decide outcome honesty (NO-THEATER).
 *
 * The gateway's /decide envelope reports `execution: "dispatched" |
 * "failed"` — whether the approved action actually ran. The dashboard
 * toast must reflect that outcome instead of an unconditional green
 * "Approved":
 *
 *   approve + "dispatched" → success "Approved — action executed"
 *   approve + "failed"     → destructive "Approved, but execution failed"
 *   execution absent       → previous generic toast (back-compat)
 *
 * The Undo affordance stays on every variant. `@/lib/api/approvals` is
 * mocked so each test controls the decide envelope.
 */

import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { ReactElement } from "react";
import { ApprovalsOpsDashboard } from "./ApprovalsOpsDashboard";
import type { ApprovalItem } from "@/lib/sample-approvals";
import type { DecisionResult } from "@/lib/api/approvals";

const { toastSuccessMock, toastErrorMock, submitDecisionMock, submitUndoMock } =
  vi.hoisted(() => ({
    toastSuccessMock: vi.fn(),
    toastErrorMock: vi.fn(),
    submitDecisionMock: vi.fn(),
    submitUndoMock: vi.fn(),
  }));

vi.mock("sonner", () => ({
  toast: {
    success: toastSuccessMock,
    error: toastErrorMock,
    info: vi.fn(),
    warning: vi.fn(),
    loading: vi.fn(),
    promise: vi.fn(),
    dismiss: vi.fn(),
  },
}));

vi.mock("@/lib/api/approvals", () => ({
  submitDecision: submitDecisionMock,
  submitUndo: submitUndoMock,
  fetchApprovals: vi.fn(async () => ({ items: [], source: "live" })),
  newIdempotencyKey: () => "idem-test",
}));

function decisionResult(over: Partial<DecisionResult>): DecisionResult {
  return {
    ok: true,
    approvalId: "ap_exec",
    decision: "approve",
    source: "live",
    ...over,
  };
}

function renderWithClient(ui: ReactElement) {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  return render(<QueryClientProvider client={client}>{ui}</QueryClientProvider>);
}

const ITEM: ApprovalItem = {
  id: "ap_exec",
  worker: { name: "Mariah" },
  verb: "wants to send a refund",
  detail: "Refund order #4821",
  consequenceIfYes: "refund sent",
  consequenceIfNo: "refund held",
  channel: "slack",
  receivedAt: new Date().toISOString(),
  severity: "p0",
};

describe("ApprovalsOpsDashboard — execution-outcome toasts", () => {
  beforeEach(() => {
    toastSuccessMock.mockReset();
    toastErrorMock.mockReset();
    submitDecisionMock.mockReset();
    submitUndoMock.mockReset();
    vi.stubGlobal(
      "matchMedia",
      vi.fn().mockReturnValue({
        matches: false,
        addEventListener: vi.fn(),
        removeEventListener: vi.fn(),
      }),
    );
  });

  it('approve + execution "dispatched" → "Approved — action executed" success toast with Undo', async () => {
    submitDecisionMock.mockResolvedValue(
      decisionResult({ execution: "dispatched" }),
    );
    const user = userEvent.setup();
    renderWithClient(<ApprovalsOpsDashboard items={[ITEM]} />);

    await user.click(screen.getByTestId("approve-ap_exec"));

    await waitFor(() => expect(toastSuccessMock).toHaveBeenCalled());
    const [title, opts] = toastSuccessMock.mock.calls[0] as [
      string,
      { description?: string; action?: { label?: string } },
    ];
    expect(title).toBe("Approved — action executed");
    expect(opts.description).toMatch(/audit trail/i);
    expect(opts.action?.label).toBe("Undo");
    expect(toastErrorMock).not.toHaveBeenCalled();
  });

  it('approve + execution "failed" → destructive "Approved, but execution failed" toast with Undo', async () => {
    submitDecisionMock.mockResolvedValue(decisionResult({ execution: "failed" }));
    const user = userEvent.setup();
    renderWithClient(<ApprovalsOpsDashboard items={[ITEM]} />);

    await user.click(screen.getByTestId("approve-ap_exec"));

    await waitFor(() => expect(toastErrorMock).toHaveBeenCalled());
    const [title, opts] = toastErrorMock.mock.calls[0] as [
      string,
      { description?: string; action?: { label?: string } },
    ];
    expect(title).toBe("Approved, but execution failed");
    expect(opts.description).toMatch(/audit trail/i);
    expect(opts.action?.label).toBe("Undo");
    expect(toastSuccessMock).not.toHaveBeenCalled();
  });

  it("execution absent → previous generic toast (back-compat with older gateways)", async () => {
    submitDecisionMock.mockResolvedValue(decisionResult({}));
    const user = userEvent.setup();
    renderWithClient(<ApprovalsOpsDashboard items={[ITEM]} />);

    await user.click(screen.getByTestId("approve-ap_exec"));

    await waitFor(() => expect(toastSuccessMock).toHaveBeenCalled());
    const [title, opts] = toastSuccessMock.mock.calls[0] as [
      string,
      { action?: { label?: string } },
    ];
    expect(title).toBe("Approved");
    expect(opts.action?.label).toBe("Undo");
    expect(toastErrorMock).not.toHaveBeenCalled();
  });

  it("refuse keeps the generic Refused toast even if execution is present", async () => {
    submitDecisionMock.mockResolvedValue(
      decisionResult({ decision: "refuse", execution: "dispatched" }),
    );
    const user = userEvent.setup();
    renderWithClient(<ApprovalsOpsDashboard items={[ITEM]} />);

    await user.click(screen.getByTestId("refuse-ap_exec"));

    await waitFor(() => expect(toastSuccessMock).toHaveBeenCalled());
    expect(toastSuccessMock.mock.calls[0]?.[0]).toBe("Refused");
  });

  it("Undo on the failed-execution toast routes back through submitUndo", async () => {
    submitDecisionMock.mockResolvedValue(decisionResult({ execution: "failed" }));
    submitUndoMock.mockResolvedValue({
      ok: true,
      approvalId: "ap_exec",
      source: "live",
    });
    const user = userEvent.setup();
    renderWithClient(<ApprovalsOpsDashboard items={[ITEM]} />);

    await user.click(screen.getByTestId("approve-ap_exec"));
    await waitFor(() => expect(toastErrorMock).toHaveBeenCalled());

    const opts = toastErrorMock.mock.calls[0]?.[1] as {
      action?: { onClick?: () => void };
    };
    opts.action?.onClick?.();
    await waitFor(() => expect(submitUndoMock).toHaveBeenCalledWith("ap_exec"));
  });
});
