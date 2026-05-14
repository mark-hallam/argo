import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  EMPTY_ORCHESTRATION_FILTERS,
  SAMPLE_ORCHESTRATION_RUNS,
  applyOrchestrationFilters,
  cancelRun,
  fetchOrchestrationRuns,
  resetRetry,
  summarizeRuns,
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
    delete process.env.NEXT_PUBLIC_GATEWAY_AUTH_BYPASS;
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

  it("falls back to mock queue on 404 in dev", async () => {
    installMock(() => makeResponse(404, { error: { message: "missing" } }));
    const result = await fetchOrchestrationRuns();
    expect(result.source).toBe("mock-offline");
    expect(result.runs).toBe(SAMPLE_ORCHESTRATION_RUNS);
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
      "http://gw.test/api/v1/orchestration/runs/run-001/cancel",
      expect.objectContaining({ method: "POST" }),
    );
    expect(fetchMock).toHaveBeenCalledWith(
      "http://gw.test/api/v1/orchestration/runs/run-002/reset-retry",
      expect.objectContaining({ method: "POST" }),
    );
  });

  it("keeps empty filters from excluding runs", () => {
    expect(applyOrchestrationFilters(SAMPLE_ORCHESTRATION_RUNS, EMPTY_ORCHESTRATION_FILTERS))
      .toHaveLength(SAMPLE_ORCHESTRATION_RUNS.length);
  });
});
