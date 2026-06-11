import { render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { RunOutputBody, RunOutputCard } from "@/components/skycot/worker/RunOutputCard";
import type { WorkerRunOutput } from "@/lib/api/worker-output";

vi.mock("@/lib/api/worker-output", async () => {
  const actual = await vi.importActual<typeof import("@/lib/api/worker-output")>(
    "@/lib/api/worker-output",
  );
  return { ...actual, fetchWorkerRunOutput: vi.fn() };
});

import { fetchWorkerRunOutput } from "@/lib/api/worker-output";

const SAMPLE: WorkerRunOutput = {
  jobId: "job-1",
  workerId: "wkr-1",
  workerName: "Maya",
  sessionId: "sess-1",
  completedAt: "2026-06-11T00:00:00Z",
  stopReason: "approval_required",
  stopDetail: "",
  stepsCompleted: 2,
  actionsEmitted: 3,
  assistantOutput: "# Draft listing\n\nHere is the work I produced.",
  result: { not_ready_to_post: false },
  actions: { counts: { total: 3, pending_approval: 1 }, actions: [] },
  actionCounts: { total: 3, pendingApproval: 1 },
};

beforeEach(() => {
  // The card now rehydrates a stored jobId on mount when no prop is
  // passed; clear so the no-jobId cases start from a never-run worker.
  sessionStorage.clear();
});

afterEach(() => {
  vi.restoreAllMocks();
});

describe("RunOutputBody", () => {
  it("renders the honest empty state when there is no output", () => {
    render(<RunOutputBody state="idle" output={null} />);
    expect(screen.getByTestId("run-output-empty")).toBeInTheDocument();
    expect(screen.getByText(/Run now/)).toBeInTheDocument();
  });

  it("renders the assistant output + proposed-action summary when present", () => {
    render(<RunOutputBody state="loaded" output={SAMPLE} />);
    expect(screen.getByTestId("run-output-assistant")).toHaveTextContent(
      "work I produced",
    );
    expect(screen.getByTestId("run-output-actions")).toHaveTextContent(
      "3 proposed actions",
    );
    expect(screen.getByTestId("run-output-actions")).toHaveTextContent(
      "1 awaiting approval",
    );
  });
});

describe("RunOutputCard", () => {
  it("shows the empty state with no jobId and does not fetch", () => {
    render(<RunOutputCard workerId="wkr-1" />);
    expect(screen.getByTestId("run-output-empty")).toBeInTheDocument();
    expect(fetchWorkerRunOutput).not.toHaveBeenCalled();
  });

  it("fetches and renders a run's output when a jobId is supplied", async () => {
    vi.mocked(fetchWorkerRunOutput).mockResolvedValue(SAMPLE);
    render(<RunOutputCard workerId="wkr-1" jobId="job-1" />);
    await waitFor(() =>
      expect(screen.getByTestId("run-output-assistant")).toBeInTheDocument(),
    );
    expect(fetchWorkerRunOutput).toHaveBeenCalledWith("wkr-1", "job-1");
  });
});
