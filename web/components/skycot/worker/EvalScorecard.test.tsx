import { render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import {
  EvalScorecard,
  EvalScorecardBody,
} from "@/components/skycot/worker/EvalScorecard";
import type { WorkerEvalReplay } from "@/lib/api/worker-evals-replay";

vi.mock("@/lib/api/worker-evals-replay", async () => {
  const actual = await vi.importActual<
    typeof import("@/lib/api/worker-evals-replay")
  >("@/lib/api/worker-evals-replay");
  return { ...actual, fetchWorkerEvalReplay: vi.fn() };
});

vi.mock("@/lib/skycot/demo-mode", () => ({
  hasRealSession: vi.fn(() => true),
  isDemoLive: vi.fn(() => false),
}));

import { fetchWorkerEvalReplay } from "@/lib/api/worker-evals-replay";
import { hasRealSession } from "@/lib/skycot/demo-mode";

function card(over: Partial<WorkerEvalReplay["merged"]> = {}): WorkerEvalReplay["merged"] {
  return {
    teammate: "maya",
    passed: 3,
    total: 4,
    passRate: 0.75,
    axisMeans: [0.92, 0.81, 0.6, 0.88, 0.95, 0.7],
    threshold: 0.7,
    weakestAxis: { axis: "boundary", mean: 0.6 },
    ...over,
  };
}

const SAMPLE: WorkerEvalReplay = {
  workerId: "wkr-1",
  teammate: "maya",
  threshold: 0.7,
  limit: 50,
  synthetic: card(),
  production: { scored: 2, unmatched: 1, uncovered: 0, scorecard: card() },
  merged: card(),
};

beforeEach(() => {
  vi.mocked(hasRealSession).mockReturnValue(true);
});

afterEach(() => {
  vi.restoreAllMocks();
});

describe("EvalScorecardBody", () => {
  it("renders all six labelled axes with the weakest flagged", () => {
    render(<EvalScorecardBody state="loaded" data={SAMPLE} />);
    expect(screen.getByTestId("eval-scorecard-axes")).toBeInTheDocument();
    for (const axis of [
      "correctness",
      "escalation",
      "boundary",
      "skill_usage",
      "honesty",
      "budget",
    ]) {
      expect(screen.getByTestId(`eval-axis-${axis}`)).toBeInTheDocument();
    }
    // boundary (0.60) is below the 0.70 threshold → warning tone.
    expect(screen.getByTestId("eval-axis-boundary")).toHaveAttribute(
      "data-passed",
      "false",
    );
    expect(screen.getByTestId("eval-axis-correctness")).toHaveAttribute(
      "data-passed",
      "true",
    );
    expect(screen.getByTestId("eval-scorecard-weakest")).toHaveTextContent(
      "boundary",
    );
  });

  it("labels real replayed work in the provenance footer", () => {
    render(<EvalScorecardBody state="loaded" data={SAMPLE} />);
    expect(screen.getByTestId("eval-scorecard-provenance")).toHaveTextContent(
      "2 scores from replayed real work",
    );
  });

  it("flags a synthetic-only score honestly (NO THEATER)", () => {
    const synthOnly: WorkerEvalReplay = {
      ...SAMPLE,
      production: { scored: 0, unmatched: 0, uncovered: 4, scorecard: card() },
    };
    render(<EvalScorecardBody state="loaded" data={synthOnly} />);
    expect(screen.getByTestId("eval-scorecard-provenance")).toHaveTextContent(
      "no replayed real work yet",
    );
  });

  it("renders the honest empty state when there are no scenarios", () => {
    const empty: WorkerEvalReplay = {
      ...SAMPLE,
      merged: card({ passed: 0, total: 0, passRate: 0, weakestAxis: null }),
    };
    render(<EvalScorecardBody state="loaded" data={empty} />);
    expect(screen.getByTestId("eval-scorecard-empty")).toBeInTheDocument();
    expect(screen.queryByTestId("eval-scorecard-axes")).not.toBeInTheDocument();
  });

  it("renders the idle gate when no live session", () => {
    render(<EvalScorecardBody state="idle" data={null} />);
    expect(screen.getByTestId("eval-scorecard-idle")).toBeInTheDocument();
  });

  it("renders an error state on fetch failure", () => {
    render(<EvalScorecardBody state="error" data={null} />);
    expect(screen.getByTestId("eval-scorecard-error")).toBeInTheDocument();
  });
});

describe("EvalScorecard", () => {
  it("fetches and renders the scorecard against a real session", async () => {
    vi.mocked(fetchWorkerEvalReplay).mockResolvedValue(SAMPLE);
    render(<EvalScorecard workerId="wkr-1" />);
    await waitFor(() =>
      expect(screen.getByTestId("eval-scorecard-axes")).toBeInTheDocument(),
    );
    expect(fetchWorkerEvalReplay).toHaveBeenCalledWith("wkr-1");
    expect(screen.getByTestId("card-eval-scorecard")).toHaveTextContent(
      "3/4 scenarios passed",
    );
  });

  it("does NOT fetch when there is no real session (mock-gated)", () => {
    vi.mocked(hasRealSession).mockReturnValue(false);
    render(<EvalScorecard workerId="wkr-1" />);
    expect(screen.getByTestId("eval-scorecard-idle")).toBeInTheDocument();
    expect(fetchWorkerEvalReplay).not.toHaveBeenCalled();
  });
});
