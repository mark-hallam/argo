import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { fetchVertical, fetchVerticals, SEED_VERTICALS } from "./verticals";
import { ApiError } from "./types";

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

const GATEWAY_VERTICAL = {
  id: "live-ops",
  name: "Live Ops",
  version: "2.0.0",
  description: "A real gateway-loaded vertical.",
  headline: "Ships from disk",
  tags: ["ops"],
  workflow_count: 3,
  action_count: 4,
  role_label: "Ops Engineer",
  kpi_count: 1,
  skill_count: 2,
};

describe("lib/api/verticals · fetchVerticals", () => {
  const originalEnv = { ...process.env };

  beforeEach(() => {
    process.env.NEXT_PUBLIC_GATEWAY_URL = "http://gw.test";
    vi.stubEnv("NODE_ENV", "test");
    delete process.env.NEXT_PUBLIC_ENV;
    delete process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS;
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

  it("an ANONYMOUS dev under auth-bypass gets the seed catalog, no network", async () => {
    process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS = "1";
    const fetchMock = installMock(() => makeResponse({ status: 500, body: "nope" }));

    const result = await fetchVerticals();

    expect(result.source).toBe("mock-bypass");
    expect(result.verticals).toBe(SEED_VERTICALS);
    expect(fetchMock).not.toHaveBeenCalled();
  });

  it("a REAL signed-in user is never served the seed — they hit the gateway", async () => {
    process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS = "1";
    document.cookie = "skycot-session=1; path=/";
    const fetchMock = installMock(() =>
      makeResponse({ status: 200, body: { verticals: [GATEWAY_VERTICAL] } }),
    );

    const result = await fetchVerticals();

    expect(result.source).toBe("gateway");
    expect(result.verticals[0].id).toBe("live-ops");
    expect(fetchMock).toHaveBeenCalled();
  });

  it("unwraps the {verticals:[...]} envelope on the happy path", async () => {
    installMock(() => makeResponse({ status: 200, body: { verticals: [GATEWAY_VERTICAL] } }));

    const result = await fetchVerticals();

    expect(result.source).toBe("gateway");
    expect(result.verticals).toHaveLength(1);
    expect(result.verticals[0].role_label).toBe("Ops Engineer");
  });

  it("an empty gateway catalog falls back to the seed for an anonymous visitor (mock-leak path 3)", async () => {
    installMock(() => makeResponse({ status: 200, body: { verticals: [] } }));

    const result = await fetchVerticals();

    expect(result.source).toBe("mock-offline");
    expect(result.verticals).toBe(SEED_VERTICALS);
  });

  it("a REAL signed-in user with an empty catalog gets [], NOT the seed", async () => {
    document.cookie = "skycot-session=1; path=/";
    installMock(() => makeResponse({ status: 200, body: { verticals: [] } }));

    const result = await fetchVerticals();

    expect(result.source).toBe("gateway");
    expect(result.verticals).toEqual([]);
    expect(result.verticals).not.toBe(SEED_VERTICALS);
  });

  it("falls back to the seed on a 404 in dev", async () => {
    installMock(() =>
      makeResponse({ status: 404, body: { error: { code: "not_found", message: "x" } } }),
    );

    const result = await fetchVerticals();

    expect(result.source).toBe("mock-offline");
    expect(result.verticals).toBe(SEED_VERTICALS);
  });

  it("a REAL signed-in user propagates a 404 as ApiError instead of seeding", async () => {
    document.cookie = "skycot-session=1; path=/";
    installMock(() =>
      makeResponse({ status: 404, body: { error: { code: "not_found", message: "x" } } }),
    );

    await expect(fetchVerticals()).rejects.toBeInstanceOf(ApiError);
  });
});

describe("lib/api/verticals · fetchVertical", () => {
  const originalEnv = { ...process.env };

  beforeEach(() => {
    process.env.NEXT_PUBLIC_GATEWAY_URL = "http://gw.test";
    vi.stubEnv("NODE_ENV", "test");
    delete process.env.NEXT_PUBLIC_ENV;
    delete process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS;
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

  it("returns a seed detail under auth-bypass for a known seed id", async () => {
    process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS = "1";
    const fetchMock = installMock(() => makeResponse({ status: 500, body: "nope" }));

    const result = await fetchVertical("finance-ar");

    expect(result.source).toBe("mock-bypass");
    expect(result.vertical.id).toBe("finance-ar");
    expect(fetchMock).not.toHaveBeenCalled();
  });

  it("returns the gateway detail on the happy path", async () => {
    installMock(() =>
      makeResponse({
        status: 200,
        body: { id: "finance-ar", name: "AR Clerk", version: "1.0.0", description: "d" },
      }),
    );

    const result = await fetchVertical("finance-ar");

    expect(result.source).toBe("gateway");
    expect(result.vertical.name).toBe("AR Clerk");
  });

  it("falls back to a seed detail on a 404 in dev", async () => {
    installMock(() =>
      makeResponse({ status: 404, body: { error: { code: "not_found", message: "x" } } }),
    );

    const result = await fetchVertical("finance-ar");

    expect(result.source).toBe("mock-offline");
    expect(result.vertical.id).toBe("finance-ar");
  });
});
