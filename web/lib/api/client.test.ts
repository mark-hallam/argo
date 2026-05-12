import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { apiGet, apiPost } from "./client";
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

/**
 * Mock fetch that answers /api/auth/session with an empty session
 * (the wrapper consults this on every request for the bearer
 * token) and delegates everything else to `gatewayResponder`.
 */
function installMock(gatewayResponder: (req: Request) => Response) {
  const fetchMock = vi.fn((input: RequestInfo | URL, init?: RequestInit) => {
    const url = typeof input === "string" ? input : input instanceof URL ? input.toString() : input.url;
    if (url.endsWith("/api/auth/session")) {
      return Promise.resolve(makeResponse({ status: 200, body: {} }));
    }
    const req = new Request(url, init);
    return Promise.resolve(gatewayResponder(req));
  });
  globalThis.fetch = fetchMock as unknown as typeof fetch;
  return fetchMock;
}

describe("lib/api/client", () => {
  beforeEach(() => {
    process.env.NEXT_PUBLIC_GATEWAY_URL = "http://gw.test";
  });
  afterEach(() => {
    globalThis.fetch = ORIGINAL_FETCH;
    vi.restoreAllMocks();
  });

  it("returns parsed body on 2xx JSON", async () => {
    installMock(() => makeResponse({ status: 200, body: { id: "ap_1", verb: "test" } }));
    const out = await apiGet<{ id: string; verb: string }>("/api/v1/x");
    expect(out).toEqual({ id: "ap_1", verb: "test" });
  });

  it("throws ApiError with parsed envelope on 4xx", async () => {
    installMock(() =>
      makeResponse({
        status: 403,
        body: { error: { code: "forbidden", message: "no access" } },
      }),
    );
    await expect(apiGet("/api/v1/x")).rejects.toMatchObject({
      name: "ApiError",
      status: 403,
      code: "forbidden",
      message: "no access",
    });
  });

  it("throws ApiError without code on 5xx", async () => {
    installMock(() =>
      makeResponse({ status: 503, body: "gateway down", contentType: "text/plain" }),
    );
    let caught: unknown;
    try {
      await apiPost("/api/v1/x", { foo: 1 });
    } catch (err) {
      caught = err;
    }
    expect(caught).toBeInstanceOf(ApiError);
    expect((caught as ApiError).status).toBe(503);
    expect((caught as ApiError).code).toBeUndefined();
  });

  it("sends Authorization when token is passed", async () => {
    const calls: { url: string; auth: string | null }[] = [];
    installMock((req) => {
      calls.push({ url: req.url, auth: req.headers.get("Authorization") });
      return makeResponse({ status: 200, body: {} });
    });
    await apiGet("/api/v1/x", { token: "abc123" });
    const gatewayCall = calls.find((c) => c.url.includes("/api/v1/x"));
    expect(gatewayCall?.auth).toBe("Bearer abc123");
  });

  it("returns undefined on 204 No Content", async () => {
    installMock(() => makeResponse({ status: 204, contentType: null }));
    const out = await apiPost("/api/v1/x", { foo: 1 });
    expect(out).toBeUndefined();
  });
});
