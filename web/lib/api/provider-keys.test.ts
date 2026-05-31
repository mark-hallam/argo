import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  __resetProviderKeysMockStore,
  fetchProviderKeys,
  saveProviderKey,
} from "./provider-keys";
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

describe("lib/api/provider-keys", () => {
  const originalEnv = { ...process.env };

  beforeEach(() => {
    __resetProviderKeysMockStore();
    process.env.NEXT_PUBLIC_GATEWAY_URL = "http://gw.test";
    vi.stubEnv("NODE_ENV", "test");
    delete process.env.NEXT_PUBLIC_ENV;
    delete process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS;
    delete process.env.NEXT_PUBLIC_SKYCOT_DEMO_LIVE;
  });

  afterEach(() => {
    __resetProviderKeysMockStore();
    globalThis.fetch = ORIGINAL_FETCH;
    process.env = { ...originalEnv };
    vi.unstubAllEnvs();
    vi.restoreAllMocks();
  });

  it("short-circuits to local provider rows under plain auth-bypass", async () => {
    process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS = "1";
    const fetchMock = installMock(() => makeResponse({ status: 200, body: [] }));
    const result = await fetchProviderKeys();
    expect(result.source).toBe("mock-bypass");
    expect(result.keys).toHaveLength(5);
    expect(fetchMock).not.toHaveBeenCalled();
  });

  it("propagates 404 in demo-live instead of returning unconfigured mock rows", async () => {
    process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS = "1";
    process.env.NEXT_PUBLIC_SKYCOT_DEMO_LIVE = "1";
    const fetchMock = installMock((req) => {
      expect(req.url).toMatch(/\/api\/v1\/providers\/keys$/);
      return makeResponse({
        status: 404,
        body: { error: { code: "not_found", message: "no provider-keys endpoint" } },
      });
    });
    await expect(fetchProviderKeys()).rejects.toBeInstanceOf(ApiError);
    expect(fetchMock).toHaveBeenCalled();
  });

  it("propagates 404 in demo-live instead of saving to the mock store", async () => {
    process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS = "1";
    process.env.NEXT_PUBLIC_SKYCOT_DEMO_LIVE = "1";
    installMock((req) => {
      expect(req.method).toBe("POST");
      expect(req.url).toMatch(/\/api\/v1\/providers\/keys$/);
      return makeResponse({
        status: 404,
        body: { error: { code: "not_found", message: "no provider-keys endpoint" } },
      });
    });
    await expect(
      saveProviderKey({ provider: "anthropic", key: "sk-ant-demo-live-1234" }),
    ).rejects.toBeInstanceOf(ApiError);

    process.env.NEXT_PUBLIC_SKYCOT_DEMO_LIVE = "0";
    const fallback = await fetchProviderKeys();
    expect(fallback.keys.find((k) => k.provider === "anthropic")?.configured).toBe(false);
  });
});
