import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { fetchTeam } from "./team";
import { ApiError } from "./types";
import { SAMPLE_TEAM } from "@/lib/sample-team";
import type { Worker } from "@/lib/types/worker";

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

function installMock(gatewayResponder: (req: Request) => Response | Promise<Response>) {
  const fetchMock = vi.fn((input: RequestInfo | URL, init?: RequestInit) => {
    const url =
      typeof input === "string" ? input : input instanceof URL ? input.toString() : input.url;
    if (url.endsWith("/api/auth/session")) {
      return Promise.resolve(makeResponse({ status: 200, body: {} }));
    }
    const req = new Request(url, init);
    return Promise.resolve(gatewayResponder(req));
  });
  globalThis.fetch = fetchMock as unknown as typeof fetch;
  return fetchMock;
}

describe("lib/api/team", () => {
  const originalEnv = { ...process.env };

  beforeEach(() => {
    process.env.NEXT_PUBLIC_GATEWAY_URL = "http://gw.test";
    vi.stubEnv("NODE_ENV", "test");
    delete process.env.NEXT_PUBLIC_ENV;
    delete process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS;
    delete process.env.NEXT_PUBLIC_SKYCOT_DEMO_MODE;
  });

  afterEach(() => {
    globalThis.fetch = ORIGINAL_FETCH;
    process.env = { ...originalEnv };
    vi.unstubAllEnvs();
    vi.restoreAllMocks();
  });

  it("returns gateway response on 2xx happy path", async () => {
    const payload = SAMPLE_TEAM.slice(0, 1);
    installMock(() => makeResponse({ status: 200, body: payload }));
    const result = await fetchTeam();
    expect(result.source).toBe("gateway");
    expect(result.workers).toEqual(payload);
  });

  it("returns empty roster + source='unreachable' on 404 outside demo mode", async () => {
    installMock(() =>
      makeResponse({
        status: 404,
        body: { error: { code: "not_found", message: "no team endpoint" } },
      }),
    );
    const result = await fetchTeam();
    // Source must NOT be 'gateway' when the gateway is down — that
    // was the silent lie that made the empty-state defect impossible
    // to diagnose in dev tools.
    expect(result.source).toBe("unreachable");
    expect(result.workers).toEqual([]);
  });

  it("falls back to SAMPLE_TEAM mock on 404 when NEXT_PUBLIC_SKYCOT_DEMO_MODE=1", async () => {
    vi.stubEnv("NEXT_PUBLIC_SKYCOT_DEMO_MODE", "1");
    installMock(() =>
      makeResponse({
        status: 404,
        body: { error: { code: "not_found", message: "no team endpoint" } },
      }),
    );
    const result = await fetchTeam();
    expect(result.source).toBe("mock-offline");
    expect(result.workers).toBe(SAMPLE_TEAM);
  });

  it("propagates 5xx errors as ApiError (no mock swallow)", async () => {
    installMock(() => makeResponse({ status: 503, body: "gateway down", contentType: "text/plain" }));
    let caught: unknown;
    try {
      await fetchTeam();
    } catch (err) {
      caught = err;
    }
    expect(caught).toBeInstanceOf(ApiError);
    expect((caught as ApiError).status).toBe(503);
  });

  it("short-circuits to mock immediately when auth-bypass flag is set", async () => {
    process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS = "1";
    const fetchMock = installMock(() => makeResponse({ status: 200, body: [] }));
    const result = await fetchTeam();
    expect(result.source).toBe("mock-bypass");
    expect(result.workers).toBe(SAMPLE_TEAM);
    expect(fetchMock).not.toHaveBeenCalled();
  });

  it("propagates 404 in strict production (no mock fallback)", async () => {
    vi.stubEnv("NODE_ENV", "production");
    vi.stubEnv("NEXT_PUBLIC_ENV", "production");
    installMock(() =>
      makeResponse({
        status: 404,
        body: { error: { code: "not_found", message: "no team endpoint" } },
      }),
    );
    await expect(fetchTeam()).rejects.toBeInstanceOf(ApiError);
  });

  it("parses the {workers,total_count} envelope returned by the gateway", async () => {
    const maya: Worker = SAMPLE_TEAM[0];
    installMock(() =>
      makeResponse({ status: 200, body: { workers: [maya], total_count: 1 } }),
    );
    const result = await fetchTeam();
    expect(result.source).toBe("gateway");
    expect(result.workers).toEqual([maya]);
    expect(result.workers.length).toBe(1);
  });

  it("reports source='unreachable' when fetch throws a network TypeError", async () => {
    // CORS preflight failure / DNS error / refused connection all surface
    // as a TypeError in the browser. The team-page bug was the catch
    // block swallowing this and reporting source='gateway' — a lie that
    // hid the failure from the operator's dev tools.
    globalThis.fetch = vi.fn(() => {
      throw new TypeError("Failed to fetch");
    }) as unknown as typeof fetch;
    const result = await fetchTeam();
    expect(result.source).toBe("unreachable");
    expect(result.workers).toEqual([]);
  });

  it("flags a malformed envelope as an ApiError instead of silently returning []", async () => {
    // Older gateway builds (or a buggy redeploy) might return a
    // payload that's neither an array nor `{workers: []}`. Treating
    // it as empty was a contributing cause to the silent
    // empty-state defect — surface it as an error instead.
    installMock(() => makeResponse({ status: 200, body: { rows: [], total: 0 } }));
    let caught: unknown;
    try {
      await fetchTeam();
    } catch (err) {
      caught = err;
    }
    expect(caught).toBeInstanceOf(ApiError);
    expect((caught as ApiError).code).toBe("malformed_envelope");
  });
});
