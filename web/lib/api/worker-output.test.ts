import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  fetchWorkerRunOutput,
  normalizeRunOutput,
} from "./worker-output";

const ORIGINAL_FETCH = globalThis.fetch;

function makeResponse({
  status = 200,
  body,
  contentType = "application/json",
}: {
  status?: number;
  body?: unknown;
  contentType?: string | null;
}): Response {
  const headers = new Headers();
  if (contentType) headers.set("content-type", contentType);
  return new Response(
    body === undefined ? null : typeof body === "string" ? body : JSON.stringify(body),
    { status, statusText: `Status ${status}`, headers },
  );
}

function installMock(responder: (req: Request) => Response | Promise<Response>) {
  const fetchMock = vi.fn((input: RequestInfo | URL, init?: RequestInit) => {
    const url =
      typeof input === "string" ? input : input instanceof URL ? input.toString() : input.url;
    if (url.endsWith("/api/auth/session")) {
      return Promise.resolve(makeResponse({ status: 200, body: {} }));
    }
    const req = new Request(url, init);
    return Promise.resolve(responder(req));
  });
  globalThis.fetch = fetchMock as unknown as typeof fetch;
  return fetchMock;
}

function clearCookies() {
  for (const pair of document.cookie.split("; ")) {
    const name = pair.split("=")[0];
    if (name) document.cookie = `${name}=; max-age=0; path=/`;
  }
}

const GATEWAY_BODY = {
  job_id: "job-1",
  worker_id: "wkr-1",
  worker_name: "Maya",
  session_id: "sess-1",
  completed_at: "2026-06-11T00:00:00Z",
  stop_reason: "approval_required",
  stop_detail: "",
  steps_completed: 2,
  actions_emitted: 3,
  assistant_output: "# Draft\n\nHere is the work I produced.",
  result: { not_ready_to_post: false },
  actions: { counts: { total: 3, pending_approval: 1, blocked: 1 }, actions: [] },
};

describe("lib/api/worker-output · normalizeRunOutput", () => {
  it("maps the snake_case gateway payload to camelCase + lifts action counts", () => {
    const out = normalizeRunOutput(GATEWAY_BODY, "wkr-1", "job-1");
    expect(out).toMatchObject({
      jobId: "job-1",
      workerId: "wkr-1",
      workerName: "Maya",
      stopReason: "approval_required",
      stepsCompleted: 2,
      actionsEmitted: 3,
    });
    expect(out.assistantOutput).toContain("work I produced");
    expect(out.result).toEqual({ not_ready_to_post: false });
    expect(out.actionCounts).toMatchObject({ total: 3, pendingApproval: 1, blocked: 1 });
  });

  it("tolerates a sparse payload (missing artifacts → null fields)", () => {
    const out = normalizeRunOutput({ job_id: "j", worker_id: "w" }, "w", "j");
    expect(out.assistantOutput).toBeNull();
    expect(out.result).toBeNull();
    expect(out.actions).toBeNull();
    expect(out.actionCounts).toBeNull();
    expect(out.stepsCompleted).toBeNull();
  });
});

describe("lib/api/worker-output · fetchWorkerRunOutput", () => {
  const originalEnv = { ...process.env };

  beforeEach(() => {
    process.env.NEXT_PUBLIC_GATEWAY_URL = "http://gw.test";
    vi.stubEnv("NODE_ENV", "test");
    delete process.env.NEXT_PUBLIC_ENV;
    delete process.env.NEXT_PUBLIC_SKYCOT_DEMO_LIVE;
    clearCookies();
  });

  afterEach(() => {
    globalThis.fetch = ORIGINAL_FETCH;
    process.env = { ...originalEnv };
    vi.unstubAllEnvs();
    vi.restoreAllMocks();
    clearCookies();
  });

  it("returns null with NO network for an anonymous visitor (no theater)", async () => {
    const fetchMock = installMock(() => makeResponse({ status: 200, body: GATEWAY_BODY }));
    const out = await fetchWorkerRunOutput("wkr-1", "job-1");
    expect(out).toBeNull();
    expect(fetchMock).not.toHaveBeenCalled();
  });

  it("a REAL signed-in user hits the gateway and gets the normalized output", async () => {
    document.cookie = "skycot-session=1; path=/";
    const fetchMock = installMock((req) => {
      expect(req.url).toContain("/api/v1/workers/wkr-1/runs/job-1/output");
      return makeResponse({ status: 200, body: GATEWAY_BODY });
    });
    const out = await fetchWorkerRunOutput("wkr-1", "job-1");
    expect(fetchMock).toHaveBeenCalled();
    expect(out?.workerName).toBe("Maya");
    expect(out?.actionCounts?.total).toBe(3);
  });

  it("a 404 (missing / cross-tenant run) degrades to null, not an error", async () => {
    document.cookie = "skycot-session=1; path=/";
    installMock(() => makeResponse({ status: 404, body: { error: { code: "not_found" } } }));
    const out = await fetchWorkerRunOutput("wkr-1", "nope");
    expect(out).toBeNull();
  });
});
