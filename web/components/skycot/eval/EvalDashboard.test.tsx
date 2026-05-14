import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { ReactElement } from "react";
import type { Worker } from "@/lib/types/worker";

vi.mock("next/navigation", () => ({
  useParams: () => ({ workerId: "wkr_eval" }),
}));

const useWorkerMock = vi.fn<() => { data: Worker | undefined }>();
vi.mock("@/lib/hooks/use-worker", () => ({
  useWorker: () => useWorkerMock(),
}));

vi.mock("@/lib/api/evals", async () => {
  const actual = await vi.importActual<typeof import("@/lib/api/evals")>("@/lib/api/evals");
  return {
    ...actual,
    fetchWorkerEvals: vi.fn(),
  };
});

import WorkerEvalsPage from "@/app/(app)/workers/[workerId]/evals/page";
import { EvalFixtureTable } from "@/components/skycot/eval/EvalFixtureTable";
import { EvalRunHistory } from "@/components/skycot/eval/EvalRunHistory";
import { RegressionBanner } from "@/components/skycot/eval/RegressionBanner";
import {
  fetchWorkerEvals,
  mockEvalDashboard,
  type EvalDashboardPayload,
} from "@/lib/api/evals";

const mockedFetchWorkerEvals = vi.mocked(fetchWorkerEvals);

function makeQueryClient() {
  return new QueryClient({
    defaultOptions: { queries: { retry: false, gcTime: 0 }, mutations: { retry: false } },
  });
}

function renderWithQuery(node: ReactElement) {
  return render(<QueryClientProvider client={makeQueryClient()}>{node}</QueryClientProvider>);
}

function payload(): EvalDashboardPayload {
  return mockEvalDashboard("wkr_eval", "mock-bypass");
}

beforeEach(() => {
  mockedFetchWorkerEvals.mockResolvedValue(payload());
  useWorkerMock.mockReturnValue({
    data: {
      id: "wkr_eval",
      name: "Priya",
      role: { tagline: "screens candidates", vertical: "recruiting" },
      status: "working",
    },
  });
});

afterEach(() => {
  vi.clearAllMocks();
});

describe("WorkerEvalsPage", () => {
  it("loads the route and renders latest eval metrics", async () => {
    renderWithQuery(<WorkerEvalsPage />);
    expect(await screen.findByTestId("worker-evals-page")).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Eval results" })).toBeInTheDocument();
    expect(screen.getByTestId("regression-banner")).toHaveAttribute("data-decision", "regressed");
    expect(screen.getAllByText("78%").length).toBeGreaterThan(0);
  });

  it("selects a prior run from history and updates fixture status", async () => {
    const user = userEvent.setup();
    renderWithQuery(<WorkerEvalsPage />);
    await screen.findByTestId("worker-evals-page");
    await user.click(screen.getByTestId("eval-run-eval_run_baseline"));
    expect(screen.getAllByText("91%").length).toBeGreaterThan(0);
    const row = screen.getByTestId("eval-fixture-reply-late-stage-candidate");
    expect(within(row).getByTestId("eval-fixture-status-reply-late-stage-candidate")).toHaveTextContent(
      "passed",
    );
  });

  it("renders an error state when the eval query fails", async () => {
    mockedFetchWorkerEvals.mockRejectedValue(new Error("gateway down"));
    renderWithQuery(<WorkerEvalsPage />);
    await waitFor(() => expect(screen.getByTestId("worker-evals-error")).toHaveTextContent("gateway down"));
  });
});

describe("eval dashboard components", () => {
  it("RegressionBanner summarizes blocking regressions", () => {
    render(<RegressionBanner run={payload().latest} />);
    expect(screen.getByTestId("regression-banner")).toHaveTextContent("Regression detected");
    expect(screen.getByTestId("regression-banner")).toHaveTextContent("need review");
  });

  it("EvalFixtureTable shows pass/fail status and score deltas", () => {
    render(<EvalFixtureTable fixtures={payload().latest.fixtures} />);
    const row = screen.getByTestId("eval-fixture-reply-late-stage-candidate");
    expect(within(row).getByTestId("eval-fixture-status-reply-late-stage-candidate")).toHaveTextContent(
      "failed",
    );
    expect(row).toHaveTextContent("-60 pts");
  });

  it("EvalRunHistory calls onSelect for the selected run", async () => {
    const user = userEvent.setup();
    const onSelect = vi.fn();
    const runs = payload().history;
    render(<EvalRunHistory runs={runs} selectedRunId={runs[0].id} onSelect={onSelect} />);
    await user.click(screen.getByTestId("eval-run-eval_run_baseline"));
    expect(onSelect).toHaveBeenCalledWith("eval_run_baseline");
  });
});
