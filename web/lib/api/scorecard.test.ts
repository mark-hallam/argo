import { afterEach, describe, expect, it, vi } from "vitest";

const clientMock = vi.hoisted(() => ({ apiGet: vi.fn(), apiPost: vi.fn() }));
vi.mock("@/lib/api/client", () => clientMock);

import {
  fetchWorkerScorecard,
  fetchTeamScorecard,
  goalTone,
  type ScorecardGoal,
} from "./scorecard";

afterEach(() => {
  vi.clearAllMocks();
});

function goal(partial: Partial<ScorecardGoal>): ScorecardGoal {
  return {
    name: "median_response_seconds",
    label: "Median response seconds",
    kind: "duration_ms",
    gate: "release",
    target: { value: 60000, comparator: "lte", display: "≤ 60000ms" },
    source: "response_latency_ms",
    measured: true,
    ...partial,
  };
}

describe("fetchWorkerScorecard", () => {
  it("hits the per-worker endpoint and returns the raw payload", async () => {
    const payload = { worker_id: "w1", goals: [], summary: { goals_total: 0, goals_measured: 0, goals_met: 0 } };
    clientMock.apiGet.mockResolvedValueOnce(payload);
    await expect(fetchWorkerScorecard("w1")).resolves.toBe(payload);
    expect(clientMock.apiGet).toHaveBeenCalledWith("/api/v1/workers/w1/scorecard");
  });

  it("url-encodes the worker id", async () => {
    clientMock.apiGet.mockResolvedValueOnce({});
    await fetchWorkerScorecard("a/b");
    expect(clientMock.apiGet).toHaveBeenCalledWith("/api/v1/workers/a%2Fb/scorecard");
  });

  it("propagates errors (no fabricated fallback)", async () => {
    clientMock.apiGet.mockRejectedValueOnce(new Error("boom"));
    await expect(fetchWorkerScorecard("w1")).rejects.toThrow("boom");
  });
});

describe("fetchTeamScorecard", () => {
  it("hits the team rollup endpoint", async () => {
    clientMock.apiGet.mockResolvedValueOnce({ workers: [] });
    await fetchTeamScorecard();
    expect(clientMock.apiGet).toHaveBeenCalledWith("/api/v1/team/scorecard");
  });
});

describe("goalTone", () => {
  it("is muted for an unmeasured goal regardless of any stale status", () => {
    expect(goalTone(goal({ measured: false, note: "target set — measurement not wired", status: undefined }))).toBe("muted");
  });
  it("is success for a met goal", () => {
    expect(goalTone(goal({ measured: true, status: "met" }))).toBe("success");
  });
  it("is warning for an at-risk goal", () => {
    expect(goalTone(goal({ measured: true, status: "at_risk" }))).toBe("warning");
  });
});
