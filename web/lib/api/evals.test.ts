import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { fetchWorkerEvals, mockEvalDashboard } from "./evals";

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

/** Minimal real-shape payload the `/api/v1/workers/<id>/evals`
 *  gateway endpoint returns. `latest: null` is a legitimate empty. */
function makeGatewayPayload(workerId: string) {
  return {
    workerId,
    latest: {
      id: "eval_run_live",
      workerId,
      skill: "candidate-screening-followup",
      version: "v5.0.0",
      createdAt: "2026-05-29T10:00:00.000Z",
      status: "passed",
      passRate: 0.93,
      meanScore: 0.9,
      fixtureCount: 12,
      durationMs: 4200,
      decision: "stable",
      fixtures: [],
    },
    history: [],
  };
}

describe("lib/api/evals", () => {
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

  describe("fetchWorkerEvals", () => {
    it("short-circuits to the mock and skips the gateway under plain auth-bypass", async () => {
      process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS = "1";
      const fetchMock = installMock(() =>
        makeResponse({ status: 200, body: makeGatewayPayload("recruiter-rita") }),
      );
      const result = await fetchWorkerEvals("recruiter-rita");
      expect(result.source).toBe("mock-bypass");
      expect(fetchMock).not.toHaveBeenCalled();
    });

    it("hits the gateway in demo-live even when a local sample exists", async () => {
      // Scene 1/3 worker-detail Evals drill-in recording mode: DEMO_LIVE
      // opts out of the mock short-circuit so the on-camera Evals tab
      // shows a real `GET /api/v1/workers/<id>/evals` round-trip, not the
      // bundled `mockEvalDashboard` fabrication.
      process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS = "1";
      process.env.NEXT_PUBLIC_SKYCOT_DEMO_LIVE = "1";
      const fetchMock = installMock(() =>
        makeResponse({ status: 200, body: makeGatewayPayload("recruiter-rita") }),
      );
      const result = await fetchWorkerEvals("recruiter-rita");
      expect(result.source).toBe("gateway");
      const hitEvals = fetchMock.mock.calls.some((call) => {
        const input = call[0];
        const url =
          typeof input === "string"
            ? input
            : input instanceof URL
              ? input.toString()
              : (input as Request).url;
        return url.includes("/api/v1/workers/recruiter-rita/evals");
      });
      expect(hitEvals).toBe(true);
    });

    it("falls back to mock-offline when demo-live gateway 404s (offline fallback preserved)", async () => {
      process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS = "1";
      process.env.NEXT_PUBLIC_SKYCOT_DEMO_LIVE = "1";
      installMock(() => makeResponse({ status: 404, body: { error: "not found" } }));
      const result = await fetchWorkerEvals("recruiter-rita");
      expect(result.source).toBe("mock-offline");
      expect(result).toEqual(mockEvalDashboard("recruiter-rita", "mock-offline"));
    });
  });
});
