/**
 * Run-now result survives the post-run remount
 * (brief `web-run-now-result-state-survives-remount`).
 *
 * Observed live (2026-06-12 DEC-044 pass): a real Run-now completes,
 * but the page remounts right after — and the response (`job_id` /
 * `held_tools` / `probation`) lived only in `WorkerOverviewContent`
 * state. So the Output card showed "No runs yet" and the approval card
 * the toast pointed at never re-rendered.
 *
 * The fix persists the run summary per worker to sessionStorage; both
 * cards rehydrate from it on mount. These pins exercise the bridge:
 *  - after a held run + a simulated remount, the approval card +
 *    held-tool chips + probation chip re-render WITHOUT a second click;
 *  - the Output card (a sibling subtree) rehydrates the same run's
 *    `job_id` and fetches the bundle;
 *  - a worker that was never run keeps today's honest empty states.
 */

import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { WorkerOverviewContent } from "./WorkerOverviewContent";
import { RunOutputCard } from "./RunOutputCard";
import type { WorkerRunOutput } from "@/lib/api/worker-output";

const { triggerWorkerRunMock, fetchWorkerRunOutputMock } = vi.hoisted(() => ({
  triggerWorkerRunMock: vi.fn(),
  fetchWorkerRunOutputMock: vi.fn(),
}));

vi.mock("sonner", () => ({
  toast: {
    success: vi.fn(),
    error: vi.fn(),
    warning: vi.fn(),
    info: vi.fn(),
    loading: vi.fn(),
    promise: vi.fn(),
    dismiss: vi.fn(),
  },
}));

vi.mock("@/lib/api/workers", () => ({
  triggerWorkerRun: triggerWorkerRunMock,
}));

vi.mock("@/lib/api/worker-output", async () => {
  const actual = await vi.importActual<typeof import("@/lib/api/worker-output")>(
    "@/lib/api/worker-output",
  );
  return { ...actual, fetchWorkerRunOutput: fetchWorkerRunOutputMock };
});

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

// Sibling cards inside WorkerOverviewContent are out of scope — stub so
// the test exercises only the run-now wiring + the real cards under test.
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

const SAMPLE_OUTPUT: WorkerRunOutput = {
  jobId: "job-42",
  workerId: "w-1",
  workerName: "Ava",
  sessionId: SID,
  completedAt: "2026-06-12T00:00:00Z",
  stopReason: "approval_required",
  stopDetail: "",
  stepsCompleted: 2,
  actionsEmitted: 1,
  assistantOutput: "# Draft\n\nHere is the work I produced.",
  result: {},
  actions: { counts: { total: 1, pending_approval: 1 }, actions: [] },
  actionCounts: { total: 1, pendingApproval: 1 },
};

function heldRun() {
  return {
    action_id: "a1",
    audit_id: "a1",
    trigger_id: "t1",
    queued_at: "2026-06-12T00:00:00Z",
    session_id: SID,
    job_id: "job-42",
    held_tools: ["send_email"],
    probation: true,
  };
}

describe("WorkerOverviewContent — Run-now result survives remount", () => {
  beforeEach(() => {
    sessionStorage.clear();
    triggerWorkerRunMock.mockReset();
    fetchWorkerRunOutputMock.mockReset();
  });

  it("rehydrates the approval card + Output card after a post-run remount", async () => {
    triggerWorkerRunMock.mockResolvedValue(heldRun());
    fetchWorkerRunOutputMock.mockResolvedValue(SAMPLE_OUTPUT);
    const user = userEvent.setup();

    const { unmount } = render(<WorkerOverviewContent workerId="w-1" />);
    await user.click(screen.getByTestId("worker-overview-run-now"));
    await screen.findByTestId("probation-approval-card");

    // Simulate the post-run remount: tear the page down, mount fresh —
    // no second click. The summary now lives in sessionStorage.
    unmount();
    render(<WorkerOverviewContent workerId="w-1" />);

    // The approval card the toast pointed at re-renders from storage.
    expect(
      await screen.findByTestId("probation-approval-card"),
    ).toBeInTheDocument();
    expect(screen.getByTestId("held-tool-send_email")).toBeInTheDocument();
    expect(screen.getByTestId("worker-probation-chip")).toBeInTheDocument();

    // The Output card (a sibling subtree, no shared React state)
    // rehydrates the same run's job_id and fetches its bundle.
    render(<RunOutputCard workerId="w-1" />);
    await waitFor(() =>
      expect(screen.getByTestId("run-output-assistant")).toBeInTheDocument(),
    );
    expect(fetchWorkerRunOutputMock).toHaveBeenCalledWith("w-1", "job-42");
  });

  it("keeps the honest empty states for a worker that was never run", () => {
    render(<WorkerOverviewContent workerId="w-never" />);
    expect(screen.queryByTestId("probation-approval-card")).toBeNull();
    expect(screen.queryByTestId("worker-probation-chip")).toBeNull();

    render(<RunOutputCard workerId="w-never" />);
    expect(screen.getByTestId("run-output-empty")).toBeInTheDocument();
    expect(fetchWorkerRunOutputMock).not.toHaveBeenCalled();
  });
});
