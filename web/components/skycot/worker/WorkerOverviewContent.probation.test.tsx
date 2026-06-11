/**
 * Day-1 unblock FIX 1 — probation visibility + in-SPA tool approval.
 *
 * Behavior pins on WorkerOverviewContent:
 *  - a run response with `held_tools` renders the "Needs your
 *    approval" card (honest warning toast, NOT the green success
 *    toast);
 *  - approve success flips the card to the [Run again] affordance,
 *    which re-invokes Run now;
 *  - approve failure surfaces the gateway error message destructively
 *    and stays retryable;
 *  - `probation: true` on the run response renders the 24h-probation
 *    chip (derived from the run envelope only — the worker detail
 *    payload doesn't expose probation);
 *  - a response without the additive fields keeps the previous
 *    success-toast behavior (no card, no chip).
 *
 * Sibling cards are stubbed — this file tests the run-now wiring and
 * the real ProbationApprovalCard.
 */

import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { WorkerOverviewContent } from "./WorkerOverviewContent";
import { ApiError } from "@/lib/api/types";

const {
  toastSuccessMock,
  toastErrorMock,
  toastWarningMock,
  triggerWorkerRunMock,
  approveSessionToolsMock,
} = vi.hoisted(() => ({
  toastSuccessMock: vi.fn(),
  toastErrorMock: vi.fn(),
  toastWarningMock: vi.fn(),
  triggerWorkerRunMock: vi.fn(),
  approveSessionToolsMock: vi.fn(),
}));

vi.mock("sonner", () => ({
  toast: {
    success: toastSuccessMock,
    error: toastErrorMock,
    warning: toastWarningMock,
    info: vi.fn(),
    loading: vi.fn(),
    promise: vi.fn(),
    dismiss: vi.fn(),
  },
}));

vi.mock("@/lib/api/workers", () => ({
  triggerWorkerRun: triggerWorkerRunMock,
}));

vi.mock("@/lib/api/session-approvals", () => ({
  approveSessionTools: approveSessionToolsMock,
}));

vi.mock("@/lib/hooks/use-worker", () => ({
  useWorker: () => ({ data: { id: "w-1", name: "Ava" } }),
}));

const OVERVIEW = {
  overview: {
    charter: "Ava keeps the inbox moving.",
    duties: [],
    kpis: [],
    recentActivity: [],
    genesis_plan_id: null,
  },
  source: "gateway",
};

vi.mock("@/lib/hooks/use-worker-overview", () => ({
  useWorkerOverview: () => ({
    data: OVERVIEW,
    isPending: false,
    isError: false,
    error: null,
    refetch: vi.fn(),
  }),
}));

vi.mock("@/lib/hooks/use-duties", () => ({
  useToggleDuty: () => ({ mutate: vi.fn() }),
}));

// Sibling cards are out of scope — stub them so the test exercises
// only the run-now wiring + the real ProbationApprovalCard.
vi.mock("@/components/skycot/worker/WhatTheyDoCard", () => ({
  WhatTheyDoCard: () => null,
  WhatTheyDoCardSkeleton: () => null,
}));
vi.mock("@/components/skycot/worker/DutiesCard", () => ({
  DutiesCard: () => null,
  DutiesCardSkeleton: () => null,
}));
vi.mock("@/components/skycot/worker/ChannelStripCard", () => ({
  ChannelStripCard: () => null,
  ChannelStripCardSkeleton: () => null,
}));
vi.mock("@/components/skycot/worker/KpiCard", () => ({
  KpiCard: () => null,
  KpiCardSkeleton: () => null,
}));
vi.mock("@/components/skycot/worker/RecentActivityCard", () => ({
  RecentActivityCard: () => null,
  RecentActivityCardSkeleton: () => null,
}));
vi.mock("@/components/skycot/worker/WorkerCostChip", () => ({
  WorkerCostChip: () => null,
}));
vi.mock("@/components/skycot/worker/EditDutyDialog", () => ({
  EditDutyDialog: () => null,
}));

const SID = "019e6785-928a-7270-a766-f945b6cd7161";

function runEnvelope(over: Record<string, unknown> = {}) {
  return {
    action_id: "a1",
    audit_id: "a1",
    trigger_id: "t1",
    queued_at: "2026-06-11T00:00:00Z",
    ...over,
  };
}

function heldEnvelope() {
  return runEnvelope({
    session_id: SID,
    held_tools: ["send_email", "web_search"],
    probation: true,
  });
}

describe("WorkerOverviewContent — probation visibility + in-SPA approval", () => {
  beforeEach(() => {
    toastSuccessMock.mockReset();
    toastErrorMock.mockReset();
    toastWarningMock.mockReset();
    triggerWorkerRunMock.mockReset();
    approveSessionToolsMock.mockReset();
  });

  it("renders the Needs-your-approval card + honest toast when the run is held", async () => {
    triggerWorkerRunMock.mockResolvedValue(heldEnvelope());
    const user = userEvent.setup();
    render(<WorkerOverviewContent workerId="w-1" />);

    expect(screen.queryByTestId("probation-approval-card")).toBeNull();
    await user.click(screen.getByTestId("worker-overview-run-now"));

    const card = await screen.findByTestId("probation-approval-card");
    expect(card.textContent).toMatch(/Needs your approval/);
    expect(card.textContent).toMatch(/24-hour probation window/);
    expect(screen.getByTestId("held-tool-send_email")).toBeInTheDocument();
    expect(screen.getByTestId("held-tool-web_search")).toBeInTheDocument();

    // Honest toast, not the green "running now" success toast.
    expect(toastWarningMock).toHaveBeenCalledTimes(1);
    expect(String(toastWarningMock.mock.calls[0]?.[0])).toMatch(
      /Ava ran, but their tools are held by the probation window/,
    );
    expect(toastSuccessMock).not.toHaveBeenCalled();

    // Probation chip derives from the run envelope.
    expect(screen.getByTestId("worker-probation-chip")).toBeInTheDocument();
  });

  it("approve success → Run again affordance → re-invokes run-now", async () => {
    triggerWorkerRunMock.mockResolvedValue(heldEnvelope());
    approveSessionToolsMock.mockResolvedValue([
      { ok: true, grant_id: "g1", session_id: SID, tool: "send_email", one_shot: false },
      { ok: true, grant_id: "g2", session_id: SID, tool: "web_search", one_shot: false },
    ]);
    const user = userEvent.setup();
    render(<WorkerOverviewContent workerId="w-1" />);

    await user.click(screen.getByTestId("worker-overview-run-now"));
    await user.click(await screen.findByTestId("probation-approve-tools"));

    expect(approveSessionToolsMock).toHaveBeenCalledWith(
      SID,
      ["send_email", "web_search"],
      expect.objectContaining({ note: expect.any(String) }),
    );

    const runAgain = await screen.findByTestId("probation-run-again");
    expect(screen.queryByTestId("probation-approve-tools")).toBeNull();

    // Second run succeeds clean → card clears, success toast fires.
    triggerWorkerRunMock.mockResolvedValue(runEnvelope({ probation: true }));
    await user.click(runAgain);

    await waitFor(() =>
      expect(screen.queryByTestId("probation-approval-card")).toBeNull(),
    );
    expect(triggerWorkerRunMock).toHaveBeenCalledTimes(2);
    expect(toastSuccessMock).toHaveBeenCalled();
    expect(String(toastSuccessMock.mock.calls.at(-1)?.[0])).toMatch(
      /Ava is running now/,
    );
  });

  it("approve failure → destructive toast with the gateway envelope message, still retryable", async () => {
    triggerWorkerRunMock.mockResolvedValue(heldEnvelope());
    approveSessionToolsMock.mockRejectedValue(
      new ApiError(404, `no session with id '${SID}'`),
    );
    const user = userEvent.setup();
    render(<WorkerOverviewContent workerId="w-1" />);

    await user.click(screen.getByTestId("worker-overview-run-now"));
    await user.click(await screen.findByTestId("probation-approve-tools"));

    await waitFor(() => expect(toastErrorMock).toHaveBeenCalled());
    const [title, opts] = toastErrorMock.mock.calls[0] as [
      string,
      { description?: string },
    ];
    expect(title).toMatch(/Couldn't approve the tools/);
    expect(opts.description).toBe(`no session with id '${SID}'`);

    // The card stays in the approvable state for a retry.
    expect(screen.getByTestId("probation-approve-tools")).toBeInTheDocument();
    expect(screen.queryByTestId("probation-run-again")).toBeNull();
  });

  it("keeps the previous success behavior when the envelope has no probation fields", async () => {
    triggerWorkerRunMock.mockResolvedValue(runEnvelope());
    const user = userEvent.setup();
    render(<WorkerOverviewContent workerId="w-1" />);

    await user.click(screen.getByTestId("worker-overview-run-now"));

    await waitFor(() => expect(toastSuccessMock).toHaveBeenCalled());
    expect(String(toastSuccessMock.mock.calls[0]?.[0])).toMatch(/Ava is running now/);
    expect(screen.queryByTestId("probation-approval-card")).toBeNull();
    expect(screen.queryByTestId("worker-probation-chip")).toBeNull();
    expect(toastWarningMock).not.toHaveBeenCalled();
  });
});
