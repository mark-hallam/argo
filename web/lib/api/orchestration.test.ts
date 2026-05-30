import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  EMPTY_ORCHESTRATION_FILTERS,
  SAMPLE_ORCHESTRATION_RUNS,
  SAMPLE_TEAM_RUNS,
  applyOrchestrationFilters,
  cancelRun,
  fetchOrchestrationRuns,
  groupTeamRuns,
  isTeamRun,
  resetRetry,
  summarizeRuns,
  teamRunLabel,
  type OrchestrationRun,
} from "./orchestration";
import { ApiError } from "./types";

const ORIGINAL_FETCH = globalThis.fetch;

function makeResponse(status: number, body?: unknown): Response {
  return new Response(body === undefined ? null : JSON.stringify(body), {
    status,
    statusText: `Status ${status}`,
    headers: { "content-type": "application/json" },
  });
}

function installMock(responder: (req: Request) => Response) {
  const fetchMock = vi.fn((input: RequestInfo | URL, init?: RequestInit) => {
    const url =
      typeof input === "string" ? input : input instanceof URL ? input.toString() : input.url;
    if (url.endsWith("/api/auth/session")) {
      return Promise.resolve(makeResponse(200, {}));
    }
    return Promise.resolve(responder(new Request(url, init)));
  });
  globalThis.fetch = fetchMock as unknown as typeof fetch;
  return fetchMock;
}

describe("lib/api/orchestration", () => {
  const originalEnv = { ...process.env };

  beforeEach(() => {
    process.env.NEXT_PUBLIC_GATEWAY_URL = "http://gw.test";
    vi.stubEnv("NODE_ENV", "test");
    delete process.env.NEXT_PUBLIC_ENV;
    delete process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS;
  });

  afterEach(() => {
    globalThis.fetch = ORIGINAL_FETCH;
    process.env = { ...originalEnv };
    vi.unstubAllEnvs();
    vi.restoreAllMocks();
  });

  it("summarizes total, running, queued, and breached counts", () => {
    const summary = summarizeRuns(SAMPLE_ORCHESTRATION_RUNS);
    expect(summary.total).toBeGreaterThanOrEqual(50);
    expect(summary.running).toBeGreaterThan(0);
    expect(summary.queued).toBeGreaterThan(0);
    expect(summary.breached).toBeGreaterThan(0);
  });

  it("applies worker, skill, status, and SLA filters together", () => {
    const first = SAMPLE_ORCHESTRATION_RUNS[0];
    const filtered = applyOrchestrationFilters(SAMPLE_ORCHESTRATION_RUNS, {
      worker: first.worker,
      skill: first.skill,
      status: first.status,
      slaState: first.slaState,
    });
    expect(filtered.length).toBeGreaterThan(0);
    expect(filtered.every((run) => run.worker === first.worker)).toBe(true);
  });

  it("returns gateway payload on the happy path", async () => {
    const runs = SAMPLE_ORCHESTRATION_RUNS.slice(0, 2);
    installMock(() => makeResponse(200, { runs, summary: summarizeRuns(runs) }));
    const result = await fetchOrchestrationRuns();
    expect(result.source).toBe("gateway");
    expect(result.runs).toEqual(runs);
  });

  it("falls back to mock queue (team runs + flat sample) on 404 in dev", async () => {
    installMock(() => makeResponse(404, { error: { message: "missing" } }));
    const result = await fetchOrchestrationRuns();
    expect(result.source).toBe("mock-offline");
    // Mock queue leads with the team runs so the tree view has data,
    // then the flat sample queue.
    expect(result.runs.slice(0, SAMPLE_TEAM_RUNS.length)).toEqual(SAMPLE_TEAM_RUNS);
    expect(result.runs).toEqual(expect.arrayContaining(SAMPLE_ORCHESTRATION_RUNS));
    expect(result.runs.length).toBe(SAMPLE_TEAM_RUNS.length + SAMPLE_ORCHESTRATION_RUNS.length);
  });

  it("propagates 404 in strict production", async () => {
    vi.stubEnv("NODE_ENV", "production");
    vi.stubEnv("NEXT_PUBLIC_ENV", "production");
    installMock(() => makeResponse(404, { error: { message: "missing" } }));
    await expect(fetchOrchestrationRuns()).rejects.toBeInstanceOf(ApiError);
  });

  it("posts operator actions to the gateway", async () => {
    const fetchMock = installMock(() => makeResponse(202, { ok: true }));
    await cancelRun("run-001");
    await resetRetry("run-002");
    expect(fetchMock).toHaveBeenCalledWith(
      expect.stringMatching(/\/api\/v1\/orchestration\/runs\/run-001\/cancel$/),
      expect.objectContaining({ method: "POST" }),
    );
    expect(fetchMock).toHaveBeenCalledWith(
      expect.stringMatching(/\/api\/v1\/orchestration\/runs\/run-002\/reset-retry$/),
      expect.objectContaining({ method: "POST" }),
    );
  });

  it("keeps empty filters from excluding runs", () => {
    expect(applyOrchestrationFilters(SAMPLE_ORCHESTRATION_RUNS, EMPTY_ORCHESTRATION_FILTERS))
      .toHaveLength(SAMPLE_ORCHESTRATION_RUNS.length);
  });
});

describe("lib/api/orchestration — team-run grouping", () => {
  // Identified rows carry the run-wide `[team:<runId>]` prefix the
  // Supervisor engine writes today.
  function leadRow(id: string, title: string, runId = "run-1"): OrchestrationRun {
    return { ...SAMPLE_TEAM_RUNS[0], id, skill: `[team:${runId}] lead synthesis — ${title}` };
  }
  function stepRow(id: string, step: string, task: string, runId = "run-1"): OrchestrationRun {
    return { ...SAMPLE_TEAM_RUNS[1], id, skill: `[team:${runId}] ${step} → ${task}` };
  }
  // Legacy rows written before the id existed carry a bare `[team]`
  // prefix and exercise the graceful fallback path.
  function legacyLeadRow(id: string, title: string): OrchestrationRun {
    return { ...SAMPLE_TEAM_RUNS[0], id, skill: `[team] lead synthesis — ${title}` };
  }
  function legacyStepRow(id: string, step: string, task: string): OrchestrationRun {
    return { ...SAMPLE_TEAM_RUNS[1], id, skill: `[team] ${step} → ${task}` };
  }

  it("detects team rows by the [team] prefix and leaves the queue flat otherwise", () => {
    expect(isTeamRun(leadRow("l", "Roll-up"))).toBe(true);
    expect(isTeamRun(stepRow("s", "survey", "Look around"))).toBe(true);
    expect(isTeamRun(SAMPLE_ORCHESTRATION_RUNS[0])).toBe(false);
  });

  it("parses a clean human label from lead and step titles", () => {
    expect(teamRunLabel(leadRow("l", "Q2 status roll-up"))).toBe("Q2 status roll-up");
    expect(teamRunLabel(stepRow("s", "revenue", "Compile revenue numbers"))).toBe(
      "Compile revenue numbers",
    );
  });

  it("groups a single-lead team run as a lead with its child steps", () => {
    const lead = leadRow("lead-1", "Competitor analysis");
    const runs = [lead, stepRow("s1", "survey", "Survey rivals"), stepRow("s2", "pricing", "Pull pricing")];
    const { teams, flat } = groupTeamRuns(runs);
    expect(flat).toHaveLength(0);
    expect(teams).toHaveLength(1);
    expect(teams[0].lead?.id).toBe("lead-1");
    expect(teams[0].title).toBe("Competitor analysis");
    expect(teams[0].children.map((c) => c.id)).toEqual(["s1", "s2"]);
  });

  it("separates team rows from the ordinary flat queue", () => {
    const lead = leadRow("lead-1", "Analysis");
    const { teams, flat } = groupTeamRuns([SAMPLE_ORCHESTRATION_RUNS[0], lead, stepRow("s1", "a", "b")]);
    expect(flat.map((r) => r.id)).toEqual([SAMPLE_ORCHESTRATION_RUNS[0].id]);
    expect(teams[0].children).toHaveLength(1);
  });

  it("returns no teams when there are no [team] rows", () => {
    const { teams, flat } = groupTeamRuns(SAMPLE_ORCHESTRATION_RUNS.slice(0, 3));
    expect(teams).toHaveLength(0);
    expect(flat).toHaveLength(3);
  });

  it("draws two separate trees from two distinct team-run ids (concurrent runs)", () => {
    const runs = [
      // Run A — Mariah, two children.
      leadRow("a-lead", "Competitor analysis", "run-a"),
      stepRow("a-s1", "survey", "Survey rivals", "run-a"),
      stepRow("a-s2", "pricing", "Pull pricing", "run-a"),
      // Run B — Patrick, one child. Interleaved with Run A's rows to
      // prove grouping is by id, not arrival order.
      leadRow("b-lead", "Q2 roll-up", "run-b"),
      stepRow("b-s1", "revenue", "Compile revenue", "run-b"),
    ];
    const { teams, flat } = groupTeamRuns(runs);
    expect(flat).toHaveLength(0);
    // TWO distinct trees, each with its OWN lead + its OWN children —
    // no shared "Delegated steps" node, no cross-attribution.
    expect(teams).toHaveLength(2);
    expect(teams.every((t) => t.lead !== null)).toBe(true);
    expect(teams.some((t) => t.title === "Delegated steps")).toBe(false);

    const runA = teams.find((t) => t.lead?.id === "a-lead");
    const runB = teams.find((t) => t.lead?.id === "b-lead");
    expect(runA?.title).toBe("Competitor analysis");
    expect(runA?.children.map((c) => c.id)).toEqual(["a-s1", "a-s2"]);
    expect(runB?.title).toBe("Q2 roll-up");
    expect(runB?.children.map((c) => c.id)).toEqual(["b-s1"]);
  });

  it("groups id-bearing step rows under their lead by run id", () => {
    const lead = leadRow("lead-1", "Analysis", "run-x");
    const { teams } = groupTeamRuns([
      lead,
      stepRow("s1", "a", "Task A", "run-x"),
      stepRow("s2", "b", "Task B", "run-x"),
    ]);
    expect(teams).toHaveLength(1);
    expect(teams[0].lead?.id).toBe("lead-1");
    expect(teams[0].children.map((c) => c.id)).toEqual(["s1", "s2"]);
  });

  it("legacy fallback: several bare-[team] leads share a 'Delegated steps' node", () => {
    const runs = [
      legacyLeadRow("lead-1", "Run A"),
      legacyLeadRow("lead-2", "Run B"),
      legacyStepRow("s1", "survey", "Survey"),
    ];
    const { teams } = groupTeamRuns(runs);
    // Two lead nodes (no attributable children) + one shared step node.
    expect(teams).toHaveLength(3);
    expect(teams.filter((t) => t.lead !== null)).toHaveLength(2);
    const shared = teams.find((t) => t.lead === null);
    expect(shared?.title).toBe("Delegated steps");
    expect(shared?.children.map((c) => c.id)).toEqual(["s1"]);
  });

  it("legacy fallback: orphan bare-[team] step rows group under an unattributed node", () => {
    const { teams } = groupTeamRuns([
      legacyStepRow("s1", "a", "Task A"),
      legacyStepRow("s2", "b", "Task B"),
    ]);
    expect(teams).toHaveLength(1);
    expect(teams[0].lead).toBeNull();
    expect(teams[0].children).toHaveLength(2);
  });

  it("groups the shipped sample team runs into two precise trees", () => {
    const { teams } = groupTeamRuns(SAMPLE_TEAM_RUNS);
    // Sample carries two distinct ids → two leads, each WITH children,
    // no shared step node.
    expect(teams).toHaveLength(2);
    expect(teams.filter((t) => t.lead !== null)).toHaveLength(2);
    expect(teams.some((t) => t.title === "Delegated steps")).toBe(false);
    const competitor = teams.find((t) => t.title === "Competitor landscape analysis");
    expect(competitor?.children).toHaveLength(3);
    const rollup = teams.find((t) => t.title === "Q2 status roll-up");
    expect(rollup?.children).toHaveLength(2);
  });
});
