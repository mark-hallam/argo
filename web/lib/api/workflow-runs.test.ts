import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  fetchWorkflowRuns,
  type WorkflowRunsSummary,
} from "./workflow-runs";

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

function makeSummary(verticalId: string, name: string): WorkflowRunsSummary {
  return {
    vertical_id: verticalId,
    workflow_name: name,
    window_days: 30,
    run_count: 7,
    success_rate: 0.91,
    avg_duration_ms: 5100,
    p95_duration_ms: 13000,
    last_run_at: "2026-05-29T08:00:00.000Z",
    page: 1,
    page_size: 25,
    total_filtered: 7,
    runs: [],
  };
}

describe("lib/api/workflow-runs", () => {
  const originalEnv = { ...process.env };

  beforeEach(() => {
    process.env.NEXT_PUBLIC_GATEWAY_URL = "http://gw.test";
    vi.stubEnv("NODE_ENV", "test");
    delete process.env.NEXT_PUBLIC_ENV;
    delete process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS;
    delete process.env.NEXT_PUBLIC_SKYCOT_DEMO_LIVE;
  });

  afterEach(() => {
    globalThis.fetch = ORIGINAL_FETCH;
    process.env = { ...originalEnv };
    vi.unstubAllEnvs();
    vi.restoreAllMocks();
  });

  describe("fetchWorkflowRuns", () => {
    it("short-circuits to the mock and skips the gateway under plain auth-bypass", async () => {
      process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS = "1";
      const fetchMock = installMock(() =>
        makeResponse({ status: 200, body: makeSummary("sales", "qualify_lead") }),
      );
      const result = await fetchWorkflowRuns({ verticalId: "sales", name: "qualify_lead" });
      expect(result.source).toBe("mock-bypass");
      expect(fetchMock).not.toHaveBeenCalled();
    });

    it("hits the gateway in demo-live even when a local sample exists", async () => {
      // Scene-5 runs-panel recording mode: DEMO_LIVE opts out of the mock
      // short-circuit so the on-camera "Run history" stat grid shows a
      // real `GET /api/v1/verticals/<v>/workflows/<n>/runs` round-trip,
      // not the deterministic `sampleWorkflowRunsSummary` fabrication.
      process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS = "1";
      process.env.NEXT_PUBLIC_SKYCOT_DEMO_LIVE = "1";
      const fetchMock = installMock(() =>
        makeResponse({ status: 200, body: makeSummary("sales", "qualify_lead") }),
      );
      const result = await fetchWorkflowRuns({ verticalId: "sales", name: "qualify_lead" });
      expect(result.source).toBe("gateway");
      expect(result.summary.run_count).toBe(7);
      const hitRuns = fetchMock.mock.calls.some((call) => {
        const input = call[0];
        const url =
          typeof input === "string"
            ? input
            : input instanceof URL
              ? input.toString()
              : (input as Request).url;
        return url.includes("/api/v1/verticals/sales/workflows/qualify_lead/runs");
      });
      expect(hitRuns).toBe(true);
    });

    it("falls back to mock-offline when demo-live gateway 404s (offline fallback preserved)", async () => {
      process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS = "1";
      process.env.NEXT_PUBLIC_SKYCOT_DEMO_LIVE = "1";
      installMock(() => makeResponse({ status: 404, body: { error: "not found" } }));
      const result = await fetchWorkflowRuns({ verticalId: "sales", name: "qualify_lead" });
      expect(result.source).toBe("mock-offline");
      expect(result.summary.vertical_id).toBe("sales");
      expect(result.summary.workflow_name).toBe("qualify_lead");
    });
  });
});
