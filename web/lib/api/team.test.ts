import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { fetchTeam } from "./team";
import { ApiError } from "./types";
import { SAMPLE_TEAM } from "@/lib/sample-team";

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
    delete process.env.NEXT_PUBLIC_GATEWAY_AUTH_BYPASS;
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

  it("falls back to SAMPLE_TEAM mock when the gateway returns 404", async () => {
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
    process.env.NEXT_PUBLIC_GATEWAY_AUTH_BYPASS = "1";
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
});
