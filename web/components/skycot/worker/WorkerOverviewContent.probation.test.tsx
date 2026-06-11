/**
 * Probation visibility + the ONE-action "Approve & run" flow
 * (keystone fix — the old approve-then-run-again two-step minted
 * grants on a dead session and never converged).
 *
 * Behavior pins on WorkerOverviewContent:
 *  - a run response with `held_tools` renders the "Needs your
 *    approval" card (honest warning toast, NOT the green success
 *    toast);
 *  - [Approve & run] makes a SINGLE run call carrying
 *    `approve_tools = heldTools` — no separate session-approve call;
 *  - a converged run (no `held_tools` in the response) clears the card
 *    and shows the "did the work — see the audit trail" success state;
 *  - a run that reaches a DIFFERENT held set re-renders the card with
 *    the new tools (progressive approval);
 *  - run failure surfaces destructively and the card stays for retry;
 *  - `probation: true` on the run response renders the 24h-probation
 *    chip; a response without the additive fields keeps the previous
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
} = vi.hoisted(() => ({
  toastSuccessMock: vi.fn(),
  toastErrorMock: vi.fn(),
  toastWarningMock: vi.fn(),
  triggerWorkerRunMock: vi.fn(),
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

function heldEnvelope(tools: string[] = ["send_email", "web_search"]) {
  return runEnvelope({
    session_id: SID,
    held_tools: tools,
    probation: true,
  });
}

describe("WorkerOverviewContent — probation + one-action Approve & run", () => {
  beforeEach(() => {
    // The component now rehydrates held-tools / probation state from a
    // per-worker sessionStorage summary on mount; clear it so each test
    // starts from a never-run worker rather than a prior test's run.
    sessionStorage.clear();
    toastSuccessMock.mockReset();
    toastErrorMock.mockReset();
    toastWarningMock.mockReset();
    triggerWorkerRunMock.mockReset();
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

  it("Approve & run sends ONE run call carrying approve_tools = the held tools", async () => {
    triggerWorkerRunMock.mockResolvedValueOnce(heldEnvelope());
    const user = userEvent.setup();
    render(<WorkerOverviewContent workerId="w-1" />);

    await user.click(screen.getByTestId("worker-overview-run-now"));

    // Converged second run: tools granted on the new session, no holds.
    triggerWorkerRunMock.mockResolvedValueOnce(
      runEnvelope({
        probation: true,
        approved_tools: ["send_email", "web_search"],
      }),
    );
    await user.click(await screen.findByTestId("probation-approve-and-run"));

    // THE keystone pin: a single run call with approve_tools — never a
    // separate approve-this-session round trip.
    expect(triggerWorkerRunMock).toHaveBeenCalledTimes(2);
    expect(triggerWorkerRunMock).toHaveBeenLastCalledWith("w-1", undefined, [
      "send_email",
      "web_search",
    ]);

    // Converged → card clears + "did the work" success state.
    await waitFor(() =>
      expect(screen.queryByTestId("probation-approval-card")).toBeNull(),
    );
    expect(toastSuccessMock).toHaveBeenCalledTimes(1);
    expect(String(toastSuccessMock.mock.calls[0]?.[0])).toMatch(
      /Ava did the work/,
    );
    expect(
      String(
        (toastSuccessMock.mock.calls[0]?.[1] as { description?: string })
          ?.description,
      ),
    ).toMatch(/audit trail/);
  });

  it("a run that reaches NEW held tools re-renders the card with the new set", async () => {
    triggerWorkerRunMock.mockResolvedValueOnce(heldEnvelope(["send_email"]));
    const user = userEvent.setup();
    render(<WorkerOverviewContent workerId="w-1" />);

    await user.click(screen.getByTestId("worker-overview-run-now"));
    expect(await screen.findByTestId("held-tool-send_email")).toBeInTheDocument();

    // The approved run gets further, then a DIFFERENT tool is held.
    triggerWorkerRunMock.mockResolvedValueOnce(
      heldEnvelope(["close_ticket"]),
    );
    await user.click(screen.getByTestId("probation-approve-and-run"));

    expect(await screen.findByTestId("held-tool-close_ticket")).toBeInTheDocument();
    expect(screen.queryByTestId("held-tool-send_email")).toBeNull();
    // Progressive-approval copy, still honest (warning, not success).
    expect(String(toastWarningMock.mock.calls.at(-1)?.[0])).toMatch(
      /got further, but reached more held tools/,
    );
    expect(toastSuccessMock).not.toHaveBeenCalled();
  });

  it("Approve & run failure surfaces destructively and the card stays for retry", async () => {
    triggerWorkerRunMock.mockResolvedValueOnce(heldEnvelope());
    const user = userEvent.setup();
    render(<WorkerOverviewContent workerId="w-1" />);

    await user.click(screen.getByTestId("worker-overview-run-now"));

    triggerWorkerRunMock.mockRejectedValueOnce(
      new ApiError(503, "gateway store unavailable"),
    );
    await user.click(await screen.findByTestId("probation-approve-and-run"));

    await waitFor(() => expect(toastErrorMock).toHaveBeenCalled());
    const [title, opts] = toastErrorMock.mock.calls[0] as [
      string,
      { description?: string },
    ];
    expect(title).toMatch(/Couldn't trigger a run/);
    expect(opts.description).toBe("gateway store unavailable");

    // The card stays in the approvable state for a retry.
    expect(screen.getByTestId("probation-approve-and-run")).toBeInTheDocument();
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
