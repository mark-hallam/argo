import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { apiGet, apiPost } from "./client";
import { ApiError, GatewayTimeoutError } from "./types";

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
    // The CSRF-missing defensive warn fires on any mutating browser
    // request that can't resolve a token (no cookie + XHR fallback
    // fails). Most tests don't bother priming a token, so silence the
    // warn by default; the dedicated CSRF describe block re-spies
    // when it needs to assert on the call.
    vi.spyOn(console, "warn").mockImplementation(() => {});
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

  describe("session-expired 401 redirect", () => {
    let assignSpy: ReturnType<typeof vi.fn>;
    let originalLocation: Location;

    beforeEach(() => {
      originalLocation = window.location;
      assignSpy = vi.fn();
      Object.defineProperty(window, "location", {
        configurable: true,
        value: {
          pathname: "/app/workers",
          search: "?cohort=peta",
          assign: assignSpy,
        },
      });
    });

    afterEach(() => {
      Object.defineProperty(window, "location", {
        configurable: true,
        value: originalLocation,
      });
    });

    it("redirects to /auth/signin with returnTo on `session expired` body", async () => {
      installMock(() =>
        makeResponse({ status: 401, body: { error: "session expired" } }),
      );
      await expect(apiGet("/api/v1/x")).rejects.toMatchObject({
        status: 401,
        code: "session_expired",
      });
      expect(assignSpy).toHaveBeenCalledTimes(1);
      expect(assignSpy).toHaveBeenCalledWith(
        "/auth/signin?returnTo=%2Fapp%2Fworkers%3Fcohort%3Dpeta",
      );
    });

    it("redirects on `session invalid` and `session revoked` bodies too", async () => {
      installMock(() =>
        makeResponse({ status: 401, body: { error: "session invalid" } }),
      );
      await expect(apiGet("/api/v1/x")).rejects.toMatchObject({ status: 401 });
      expect(assignSpy).toHaveBeenCalledTimes(1);
    });

    it("redirects on SSO-shape envelope { error: { code: 'session_expired' } }", async () => {
      installMock(() =>
        makeResponse({
          status: 401,
          body: { error: { code: "session_expired", message: "expired" } },
        }),
      );
      await expect(apiGet("/api/v1/x")).rejects.toMatchObject({
        status: 401,
        code: "session_expired",
      });
      expect(assignSpy).toHaveBeenCalledTimes(1);
    });

    it("does NOT redirect on bearer-auth `unauthorized` 401", async () => {
      installMock(() =>
        makeResponse({
          status: 401,
          body: { error: "unauthorized", reason: "missing bearer token" },
        }),
      );
      await expect(apiGet("/api/v1/x")).rejects.toMatchObject({ status: 401 });
      expect(assignSpy).not.toHaveBeenCalled();
    });

    it("does NOT redirect on 403 RBAC denial", async () => {
      installMock(() =>
        makeResponse({
          status: 403,
          body: { error: { code: "forbidden", message: "no capability" } },
        }),
      );
      await expect(apiGet("/api/v1/x")).rejects.toMatchObject({
        status: 403,
        code: "forbidden",
      });
      expect(assignSpy).not.toHaveBeenCalled();
    });

    it("does NOT redirect when already on /auth/* surface", async () => {
      Object.defineProperty(window, "location", {
        configurable: true,
        value: { pathname: "/auth/signin", search: "", assign: assignSpy },
      });
      installMock(() =>
        makeResponse({ status: 401, body: { error: "session expired" } }),
      );
      await expect(apiGet("/api/v1/x")).rejects.toMatchObject({ status: 401 });
      expect(assignSpy).not.toHaveBeenCalled();
    });
  });

  describe("CSRF double-submit (X-CSRF-Token attachment)", () => {
    const ORIGINAL_COOKIE = Object.getOwnPropertyDescriptor(
      Document.prototype,
      "cookie",
    );

    function stubCookie(value: string) {
      Object.defineProperty(document, "cookie", {
        configurable: true,
        get: () => value,
        set: () => {
          /* test stub: writes ignored */
        },
      });
    }

    afterEach(() => {
      if (ORIGINAL_COOKIE) {
        Object.defineProperty(Document.prototype, "cookie", ORIGINAL_COOKIE);
      }
    });

    it("attaches X-CSRF-Token from __Host-csrf cookie on POST", async () => {
      stubCookie("__Host-csrf=jti-token-abc; other=ignored");
      let observed: string | null = null;
      installMock((req) => {
        observed = req.headers.get("X-CSRF-Token");
        return makeResponse({ status: 200, body: { ok: true } });
      });
      await apiPost("/api/v1/verticals/pr/workflows/qualify/execute", {});
      expect(observed).toBe("jti-token-abc");
    });

    it("attaches X-CSRF-Token on PATCH and DELETE too", async () => {
      stubCookie("__Host-csrf=jti-patch-del");
      const seen: Record<string, string | null> = {};
      installMock((req) => {
        const method = req.method;
        seen[method] = req.headers.get("X-CSRF-Token");
        return makeResponse({ status: 200, body: {} });
      });
      const { apiPatch, apiDelete } = await import("./client");
      await apiPatch("/api/v1/x", {});
      await apiDelete("/api/v1/x");
      expect(seen.PATCH).toBe("jti-patch-del");
      expect(seen.DELETE).toBe("jti-patch-del");
    });

    it("does NOT attach X-CSRF-Token on GET (safe method)", async () => {
      stubCookie("__Host-csrf=jti-token-get");
      let observed: string | null = "sentinel";
      installMock((req) => {
        observed = req.headers.get("X-CSRF-Token");
        return makeResponse({ status: 200, body: {} });
      });
      await apiGet("/api/v1/audit");
      expect(observed).toBeNull();
    });

    it("falls back to GET /api/v1/auth/csrf when cookie missing", async () => {
      stubCookie(""); // no __Host-csrf
      const calls: { url: string; csrf: string | null }[] = [];
      const fetchMock = vi.fn((input: RequestInfo | URL, init?: RequestInit) => {
        const url = typeof input === "string" ? input : input instanceof URL ? input.toString() : input.url;
        if (url.endsWith("/api/auth/session")) {
          return Promise.resolve(makeResponse({ status: 200, body: {} }));
        }
        if (url.endsWith("/api/v1/auth/csrf")) {
          return Promise.resolve(makeResponse({ status: 200, body: { token: "xhr-fallback-tok" } }));
        }
        const req = new Request(url, init);
        calls.push({ url, csrf: req.headers.get("X-CSRF-Token") });
        return Promise.resolve(makeResponse({ status: 200, body: {} }));
      });
      globalThis.fetch = fetchMock as unknown as typeof fetch;
      await apiPost("/api/v1/verticals/pr/workflows/qualify/execute", {});
      const gw = calls.find((c) => c.url.includes("/api/v1/verticals/"));
      expect(gw?.csrf).toBe("xhr-fallback-tok");
    });

    it("does NOT overwrite a caller-supplied X-CSRF-Token header", async () => {
      stubCookie("__Host-csrf=cookie-tok");
      let observed: string | null = null;
      installMock((req) => {
        observed = req.headers.get("X-CSRF-Token");
        return makeResponse({ status: 200, body: {} });
      });
      await apiPost("/api/v1/x", {}, {
        headers: { "X-CSRF-Token": "caller-supplied-tok" },
      });
      expect(observed).toBe("caller-supplied-tok");
    });

    it("logs a console.warn when CSRF token cannot be resolved", async () => {
      stubCookie(""); // no cookie
      // XHR fallback also fails — gateway is down for the test:
      const fetchMock = vi.fn((input: RequestInfo | URL, init?: RequestInit) => {
        const url = typeof input === "string" ? input : input instanceof URL ? input.toString() : input.url;
        if (url.endsWith("/api/auth/session")) {
          return Promise.resolve(makeResponse({ status: 200, body: {} }));
        }
        if (url.endsWith("/api/v1/auth/csrf")) {
          return Promise.resolve(makeResponse({ status: 500, body: "down" }));
        }
        void init;
        return Promise.resolve(makeResponse({ status: 200, body: {} }));
      });
      globalThis.fetch = fetchMock as unknown as typeof fetch;
      const warn = vi.spyOn(console, "warn").mockImplementation(() => {});
      try {
        await apiPost("/api/v1/x", {});
        expect(warn).toHaveBeenCalledTimes(1);
        const msg = String(warn.mock.calls[0]?.[0] ?? "");
        expect(msg).toContain("csrf_missing");
        expect(msg).toContain("POST /api/v1/x");
      } finally {
        warn.mockRestore();
      }
    });
  });

  describe("gatewayBase URL resolution (F01 fix — same-origin in browser)", () => {
    it("browser context: uses same-origin path with no gateway prefix", async () => {
      // The default test env (happy-dom) has `window` defined, so this
      // matches the SPA's runtime behavior: `apiGet('/api/v1/audit')`
      // must call `fetch('/api/v1/audit')`, NOT
      // `fetch('http://gw.test/api/v1/audit')`.
      const calls: string[] = [];
      const fetchMock = vi.fn((input: RequestInfo | URL, init?: RequestInit) => {
        const url = typeof input === "string" ? input : input instanceof URL ? input.toString() : input.url;
        if (url.endsWith("/api/auth/session")) {
          return Promise.resolve(makeResponse({ status: 200, body: {} }));
        }
        calls.push(url);
        const req = new Request(`http://localhost${url}`, init);
        void req;
        return Promise.resolve(makeResponse({ status: 200, body: {} }));
      });
      globalThis.fetch = fetchMock as unknown as typeof fetch;
      await apiGet("/api/v1/audit");
      expect(calls).toEqual(["/api/v1/audit"]);
      expect(calls[0]).not.toMatch(/^https?:/);
    });

    it("SSR context (no window): uses absolute NEXT_PUBLIC_GATEWAY_URL", async () => {
      // Pretend we're in the Next.js server process — typeof window
      // === 'undefined' — and confirm the absolute URL is restored so
      // SSR/route-handler fetches still hit the gateway directly.
      vi.stubGlobal("window", undefined);
      const calls: string[] = [];
      const fetchMock = vi.fn((input: RequestInfo | URL) => {
        const url = typeof input === "string" ? input : input instanceof URL ? input.toString() : input.url;
        calls.push(url);
        return Promise.resolve(makeResponse({ status: 200, body: {} }));
      });
      globalThis.fetch = fetchMock as unknown as typeof fetch;
      try {
        // Pass a token override so request() skips readSessionPeek
        // (which would otherwise try to import @/auth in SSR mode).
        await apiGet("/api/v1/audit", { token: "ssr-token" });
        expect(calls).toEqual(["http://gw.test/api/v1/audit"]);
      } finally {
        vi.unstubAllGlobals();
      }
    });
  });

  describe("request timeout (universal perpetual-skeleton backstop)", () => {
    // The root fix for the /team + /audit "loads forever" bug class:
    // every read is bounded at this chokepoint, so React Query can never
    // sit in `isPending` forever against a gateway that accepts the TCP
    // socket but never sends an HTTP response. These tests drive a
    // never-settling fetch with fake timers and assert the promise
    // settles (rejects with GatewayTimeoutError) within the budget.

    /**
     * Mock where `/api/auth/session` answers instantly (so the bearer
     * peek doesn't interfere) but the gateway call NEVER settles —
     * simulating a wedged socket.
     */
    function installHangingGateway() {
      globalThis.fetch = vi.fn((input: RequestInfo | URL) => {
        const url =
          typeof input === "string"
            ? input
            : input instanceof URL
              ? input.toString()
              : input.url;
        if (url.endsWith("/api/auth/session")) {
          return Promise.resolve(makeResponse({ status: 200, body: {} }));
        }
        return new Promise<Response>(() => {});
      }) as unknown as typeof fetch;
    }

    it("rejects a hung GET with GatewayTimeoutError after the 15s read default", async () => {
      vi.useFakeTimers();
      try {
        installHangingGateway();
        const pending = apiGet("/api/v1/x");
        const assertion = expect(pending).rejects.toBeInstanceOf(
          GatewayTimeoutError,
        );
        await vi.advanceTimersByTimeAsync(15_001);
        await assertion;
      } finally {
        vi.useRealTimers();
      }
    });

    it("honours a per-call timeoutMs override (snappier 5s)", async () => {
      vi.useFakeTimers();
      try {
        installHangingGateway();
        const pending = apiGet("/api/v1/x", { timeoutMs: 5_000 });
        const assertion = expect(pending).rejects.toThrow(/within 5000ms/);
        await vi.advanceTimersByTimeAsync(5_001);
        await assertion;
      } finally {
        vi.useRealTimers();
      }
    });

    it("treats timeoutMs:0 as UNBOUNDED (no timeout fires)", async () => {
      vi.useFakeTimers();
      try {
        installHangingGateway();
        let settled = false;
        void apiGet("/api/v1/x", { timeoutMs: 0 }).then(
          () => {
            settled = true;
          },
          () => {
            settled = true;
          },
        );
        await vi.advanceTimersByTimeAsync(30_000);
        expect(settled).toBe(false);
      } finally {
        vi.useRealTimers();
      }
    });

    it("leaves mutations UNBOUNDED by default (long ops like Genesis must not be aborted)", async () => {
      vi.useFakeTimers();
      try {
        installHangingGateway();
        let settled = false;
        void apiPost("/api/v1/x", { foo: 1 }).then(
          () => {
            settled = true;
          },
          () => {
            settled = true;
          },
        );
        await vi.advanceTimersByTimeAsync(30_000);
        expect(settled).toBe(false);
      } finally {
        vi.useRealTimers();
      }
    });

    it("a hung session-peek does NOT strand the request — it proceeds unauthenticated", async () => {
      // readSessionPeek() is awaited before every request; if IT hung,
      // every page would strand. The 10s session-peek bound makes it
      // resolve to "no session" so the gateway call still goes out.
      vi.useFakeTimers();
      try {
        globalThis.fetch = vi.fn((input: RequestInfo | URL) => {
          const url =
            typeof input === "string"
              ? input
              : input instanceof URL
                ? input.toString()
                : input.url;
          // Session peek hangs; the gateway call answers fine.
          if (url.endsWith("/api/auth/session")) {
            return new Promise<Response>(() => {});
          }
          return Promise.resolve(makeResponse({ status: 200, body: { ok: true } }));
        }) as unknown as typeof fetch;

        const pending = apiGet<{ ok: boolean }>("/api/v1/x");
        await vi.advanceTimersByTimeAsync(10_001);
        const out = await pending;
        expect(out).toEqual({ ok: true });
      } finally {
        vi.useRealTimers();
      }
    });
  });
});
