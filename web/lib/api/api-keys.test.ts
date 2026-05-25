import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { createApiKey, fetchApiKeys, revokeApiKey } from "./api-keys";
import { ApiError } from "./types";
import { SAMPLE_API_KEYS } from "@/lib/sample-api-keys";

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

describe("lib/api/api-keys", () => {
  const originalEnv = { ...process.env };

  beforeEach(() => {
    process.env.NEXT_PUBLIC_GATEWAY_URL = "http://gw.test";
    vi.stubEnv("NODE_ENV", "test");
    delete process.env.NEXT_PUBLIC_ENV;
    delete process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS;
  });

  afterEach(() => {
    globalThis.fetch = ORIGINAL_FETCH;
    process.env = { ...originalEnv };
    vi.unstubAllEnvs();
    vi.restoreAllMocks();
  });

  describe("fetchApiKeys", () => {
    it("returns gateway response on 2xx happy path", async () => {
      const payload = SAMPLE_API_KEYS.slice(0, 1);
      installMock(() => makeResponse({ status: 200, body: payload }));
      const result = await fetchApiKeys();
      expect(result.source).toBe("gateway");
      expect(result.keys).toEqual(payload);
    });

    it("falls back to SAMPLE_API_KEYS when the gateway returns 404 in dev", async () => {
      installMock(() =>
        makeResponse({
          status: 404,
          body: { error: { code: "not_found", message: "no api-keys endpoint" } },
        }),
      );
      const result = await fetchApiKeys();
      expect(result.source).toBe("mock-offline");
      expect(result.keys).toBe(SAMPLE_API_KEYS);
    });

    it("propagates 5xx as ApiError (no mock swallow)", async () => {
      installMock(() =>
        makeResponse({ status: 503, body: "gateway down", contentType: "text/plain" }),
      );
      let caught: unknown;
      try {
        await fetchApiKeys();
      } catch (err) {
        caught = err;
      }
      expect(caught).toBeInstanceOf(ApiError);
      expect((caught as ApiError).status).toBe(503);
    });

    it("short-circuits to mock immediately when NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=1", async () => {
      process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS = "1";
      const fetchMock = installMock(() => makeResponse({ status: 200, body: [] }));
      const result = await fetchApiKeys();
      expect(result.source).toBe("mock-bypass");
      expect(result.keys).toBe(SAMPLE_API_KEYS);
      expect(fetchMock).not.toHaveBeenCalled();
    });

    it("propagates 404 in strict production (no mock fallback)", async () => {
      vi.stubEnv("NODE_ENV", "production");
      vi.stubEnv("NEXT_PUBLIC_ENV", "production");
      installMock(() =>
        makeResponse({
          status: 404,
          body: { error: { code: "not_found", message: "no api-keys endpoint" } },
        }),
      );
      await expect(fetchApiKeys()).rejects.toBeInstanceOf(ApiError);
    });
  });

  describe("createApiKey", () => {
    it("returns gateway response on 2xx happy path", async () => {
      const expected = {
        key: SAMPLE_API_KEYS[0],
        token: "sk_live_fullsecrettoken1234567890abcdef",
      };
      installMock(() => makeResponse({ status: 201, body: expected }));
      const result = await createApiKey({
        name: "CI pipeline",
        scopes: ["read:workers"],
        cohort_ids: [],
        all_cohorts: true,
        expiry: "90d",
      });
      expect(result).toEqual(expected);
    });

    it("falls back to a mock token + row when the gateway returns 404 in dev", async () => {
      installMock(() =>
        makeResponse({
          status: 404,
          body: { error: { code: "not_found", message: "no api-keys endpoint" } },
        }),
      );
      const result = await createApiKey({
        name: "Mock CI",
        scopes: ["read:workers"],
        cohort_ids: [],
        all_cohorts: true,
        expiry: "30d",
      });
      expect(result.token).toMatch(/^sk_live_/);
      expect(result.key.name).toBe("Mock CI");
      expect(result.key.scopes).toEqual(["read:workers"]);
      expect(result.key.all_cohorts).toBe(true);
      expect(result.key.expires_at).not.toBeNull();
    });

    it("propagates non-404 errors as ApiError", async () => {
      installMock(() =>
        makeResponse({
          status: 422,
          body: { error: { code: "invalid_scope", message: "bad scope" } },
        }),
      );
      let caught: unknown;
      try {
        await createApiKey({
          name: "bad",
          scopes: ["read:workers"],
          cohort_ids: [],
          all_cohorts: true,
          expiry: "never",
        });
      } catch (err) {
        caught = err;
      }
      expect(caught).toBeInstanceOf(ApiError);
      expect((caught as ApiError).status).toBe(422);
    });

    it("maps expiry=never to null expires_at in the mock fallback", async () => {
      installMock(() => makeResponse({ status: 404, body: { error: { code: "x", message: "x" } } }));
      const result = await createApiKey({
        name: "Forever key",
        scopes: ["read:workers"],
        cohort_ids: [],
        all_cohorts: true,
        expiry: "never",
      });
      expect(result.key.expires_at).toBeNull();
    });
  });

  describe("revokeApiKey", () => {
    it("returns ok envelope on 2xx", async () => {
      installMock(() => makeResponse({ status: 204, contentType: null }));
      const result = await revokeApiKey("key_abc");
      expect(result).toEqual({ ok: true, id: "key_abc" });
    });

    it("falls back to ok in dev when the gateway returns 404", async () => {
      installMock(() =>
        makeResponse({ status: 404, body: { error: { code: "x", message: "x" } } }),
      );
      const result = await revokeApiKey("key_abc");
      expect(result).toEqual({ ok: true, id: "key_abc" });
    });

    it("propagates 5xx", async () => {
      installMock(() => makeResponse({ status: 500, body: "boom", contentType: "text/plain" }));
      await expect(revokeApiKey("key_abc")).rejects.toBeInstanceOf(ApiError);
    });
  });
});
