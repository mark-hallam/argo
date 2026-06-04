import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { fetchWorkerTrustGrade, recomputeWorkerTrustGrade } from "./worker-trust";

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

/** Real snake_case shape the `/api/v1/workers/<id>/trust-grade`
 *  gateway endpoint returns (mapped by `normalizeTrustGrade`). */
function makeGatewayBody(workerId: string) {
  return {
    worker_id: workerId,
    current_grade: "tenant",
    current_grade_label: "Tenant-verified",
    current_grade_tone: "success",
    current_grade_index: 2,
    next_grade: "published_verified",
    next_grade_label: "Published-verified",
    at_max_grade: false,
    signals: {
      trust_grade_value: 2,
      has_been_used: true,
      last_active_days_ago: 1,
    },
    next_grade_criteria: [],
  };
}

describe("lib/api/worker-trust", () => {
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

  describe("fetchWorkerTrustGrade", () => {
    it("short-circuits to the mock and skips the gateway under plain auth-bypass", async () => {
      process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS = "1";
      const fetchMock = installMock(() =>
        makeResponse({ status: 200, body: makeGatewayBody("priya") }),
      );
      const result = await fetchWorkerTrustGrade("priya");
      expect(result.source).toBe("mock-bypass");
      expect(fetchMock).not.toHaveBeenCalled();
    });

    it("hits the gateway in demo-live even when a local sample exists", async () => {
      // Scene 1/6 worker-detail Trust tab recording mode: DEMO_LIVE opts
      // out of the mock short-circuit so the on-camera Trust grade shows a
      // real `GET /api/v1/workers/<id>/trust-grade` round-trip, not the
      // bundled `findSampleWorkerTrust` fabrication.
      process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS = "1";
      process.env.NEXT_PUBLIC_SKYCOT_DEMO_LIVE = "1";
      const fetchMock = installMock(() =>
        makeResponse({ status: 200, body: makeGatewayBody("priya") }),
      );
      const result = await fetchWorkerTrustGrade("priya");
      expect(result.source).toBe("gateway");
      expect(result.snapshot.currentGrade).toBe("tenant");
      const hitTrust = fetchMock.mock.calls.some((call) => {
        const input = call[0];
        const url =
          typeof input === "string"
            ? input
            : input instanceof URL
              ? input.toString()
              : (input as Request).url;
        return url.includes("/api/v1/workers/priya/trust-grade");
      });
      expect(hitTrust).toBe(true);
    });

    it("propagates gateway 404 in demo-live instead of returning mock-offline", async () => {
      process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS = "1";
      process.env.NEXT_PUBLIC_SKYCOT_DEMO_LIVE = "1";
      installMock(() => makeResponse({ status: 404, body: { error: "not found" } }));
      await expect(fetchWorkerTrustGrade("priya")).rejects.toMatchObject({ status: 404 });
    });

    it("propagates recompute 404 in demo-live instead of returning a synthetic hold", async () => {
      process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS = "1";
      process.env.NEXT_PUBLIC_SKYCOT_DEMO_LIVE = "1";
      installMock(() => makeResponse({ status: 404, body: { error: "not found" } }));
      await expect(recomputeWorkerTrustGrade("priya")).rejects.toMatchObject({ status: 404 });
    });
  });
});

// ws3-3 (sprint-ws3-3-verify-prod-no-bypass-fetchers): a NODE_ENV=
// production bundle ignores the dev auth-bypass flag and never
// short-circuits to `findSampleWorkerTrust`; in strict production a
// 404 propagates rather than returning a sample snapshot.
describe("lib/api/worker-trust — ws3-3: prod path never serves fixtures", () => {
  const originalEnv = { ...process.env };

  beforeEach(() => {
    process.env.NEXT_PUBLIC_GATEWAY_URL = "http://gw.test";
  });

  afterEach(() => {
    globalThis.fetch = ORIGINAL_FETCH;
    process.env = { ...originalEnv };
    vi.unstubAllEnvs();
    vi.restoreAllMocks();
  });

  it("ignores a hostile auth-bypass flag under NODE_ENV=production and hits the gateway", async () => {
    vi.stubEnv("NODE_ENV", "production");
    vi.stubEnv("NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS", "1");
    const fetchMock = installMock(() =>
      makeResponse({ status: 200, body: makeGatewayBody("priya") }),
    );
    const result = await fetchWorkerTrustGrade("priya");
    expect(fetchMock).toHaveBeenCalled();
    expect(result.source).toBe("gateway");
  });

  it("propagates a gateway 404 in strict production instead of returning a sample snapshot", async () => {
    vi.stubEnv("NODE_ENV", "production");
    vi.stubEnv("NEXT_PUBLIC_ENV", "production");
    installMock(() => makeResponse({ status: 404, body: { error: "not found" } }));
    await expect(fetchWorkerTrustGrade("priya")).rejects.toMatchObject({ status: 404 });
  });
});
