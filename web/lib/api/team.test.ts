import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { fetchTeam, InvalidTeamEnvelope } from "./team";
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
    delete process.env.NEXT_PUBLIC_SKYCOT_DEMO_LIVE;
  });

  afterEach(() => {
    globalThis.fetch = ORIGINAL_FETCH;
    process.env = { ...originalEnv };
    vi.unstubAllEnvs();
    vi.restoreAllMocks();
  });

  it("returns gateway response on 2xx happy path (canonical envelope)", async () => {
    const payload = SAMPLE_TEAM.slice(0, 1);
    installMock(() =>
      makeResponse({ status: 200, body: { workers: payload, total_count: payload.length } }),
    );
    const result = await fetchTeam();
    expect(result.source).toBe("gateway");
    expect(result.workers).toEqual(payload);
  });

  it("rejects a bare-array response with InvalidTeamEnvelope (no back-compat tolerance)", async () => {
    // A legacy gateway / a buggy redeploy shipping a bare `Worker[]`
    // is a contract violation now, not a tolerated shape. Pinning the
    // rejection stops the bare-array path from silently re-appearing.
    installMock(() => makeResponse({ status: 200, body: SAMPLE_TEAM.slice(0, 1) }));
    let caught: unknown;
    try {
      await fetchTeam();
    } catch (err) {
      caught = err;
    }
    expect(caught).toBeInstanceOf(InvalidTeamEnvelope);
    expect((caught as ApiError).code).toBe("invalid_team_envelope");
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

  it("does not return SAMPLE_TEAM when demo-mode is paired with demo-live", async () => {
    vi.stubEnv("NEXT_PUBLIC_SKYCOT_DEMO_MODE", "1");
    vi.stubEnv("NEXT_PUBLIC_SKYCOT_DEMO_LIVE", "1");
    installMock(() =>
      makeResponse({
        status: 404,
        body: { error: { code: "not_found", message: "no team endpoint" } },
      }),
    );
    const result = await fetchTeam();
    expect(result.source).toBe("unreachable");
    expect(result.workers).toEqual([]);
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

  it("flags a malformed envelope as InvalidTeamEnvelope instead of silently returning []", async () => {
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
    expect(caught).toBeInstanceOf(InvalidTeamEnvelope);
    expect(caught).toBeInstanceOf(ApiError);
    expect((caught as ApiError).code).toBe("invalid_team_envelope");
  });

  describe("demo-mode catch-all (perpetual skeleton regression)", () => {
    // The /team-stuck-on-skeleton bug was a fetchTeam path that
    // surfaced an unhandled rejection — React Query then held the
    // query in `pending` forever, the skeleton tiles never resolved,
    // and Scene 1 step 1 of the v1.0 demo MP4 was unrecordable.
    // These tests pin that EVERY error path in demo mode resolves
    // synchronously to SAMPLE_TEAM, never throws.

    beforeEach(() => {
      vi.stubEnv("NEXT_PUBLIC_SKYCOT_DEMO_MODE", "1");
    });

    it("resolves to SAMPLE_TEAM within 1s when apiGet throws a 404", async () => {
      installMock(() =>
        makeResponse({
          status: 404,
          body: { error: { code: "not_found", message: "no team endpoint" } },
        }),
      );
      const start = Date.now();
      const result = await fetchTeam();
      const elapsed = Date.now() - start;
      expect(result.source).toBe("mock-offline");
      expect(result.workers).toBe(SAMPLE_TEAM);
      expect(elapsed).toBeLessThan(1000);
    });

    it("resolves to SAMPLE_TEAM on a 5xx gateway error (no throw)", async () => {
      // Without the demo-mode catch-all, a 503 would propagate and
      // leave the query in error state, which the page then renders
      // as the "We're having trouble loading your team" red panel —
      // also a camera-visible defect for the demo MP4.
      installMock(() =>
        makeResponse({ status: 503, body: "gateway down", contentType: "text/plain" }),
      );
      const result = await fetchTeam();
      expect(result.source).toBe("mock-offline");
      expect(result.workers).toBe(SAMPLE_TEAM);
    });

    it("resolves to SAMPLE_TEAM on a malformed envelope (no throw)", async () => {
      installMock(() => makeResponse({ status: 200, body: { rows: [], total: 0 } }));
      const result = await fetchTeam();
      expect(result.source).toBe("mock-offline");
      expect(result.workers).toBe(SAMPLE_TEAM);
    });

    it("resolves to SAMPLE_TEAM on a TypeError (CORS / DNS / refused)", async () => {
      globalThis.fetch = vi.fn(() => {
        throw new TypeError("Failed to fetch");
      }) as unknown as typeof fetch;
      const result = await fetchTeam();
      expect(result.source).toBe("mock-offline");
      expect(result.workers).toBe(SAMPLE_TEAM);
    });

    it("resolves to SAMPLE_TEAM when the gateway hangs past the 5s timeout", async () => {
      vi.useFakeTimers();
      try {
        globalThis.fetch = vi.fn(
          (input: RequestInfo | URL) => {
            const url =
              typeof input === "string"
                ? input
                : input instanceof URL
                  ? input.toString()
                  : input.url;
            if (url.endsWith("/api/auth/session")) {
              return Promise.resolve(makeResponse({ status: 200, body: {} }));
            }
            // Never settle — simulates a wedged gateway TCP socket.
            return new Promise<Response>(() => {});
          },
        ) as unknown as typeof fetch;

        const pending = fetchTeam();
        // Advance past the 5s timeout in fetchTeam's withTimeout race.
        await vi.advanceTimersByTimeAsync(5001);
        const result = await pending;
        expect(result.source).toBe("mock-offline");
        expect(result.workers).toBe(SAMPLE_TEAM);
      } finally {
        vi.useRealTimers();
      }
    });
  });

  describe("non-demo gateway hang (real-user perpetual-skeleton regression)", () => {
    // THE bug a real stranger hit during the live signup E2E walk: a
    // signed-in user (NOT demo mode) opens /team, the gateway accepts the
    // TCP connection but never answers, and — because the 5s timeout used
    // to be applied only in demo mode — the fetch promise never settled.
    // React Query then held the query `isPending` forever and the page
    // rendered loading tiles that never resolved. These tests pin that the
    // fetch is now bounded in EVERY mode and settles to the honest
    // terminal state (empty roster in dev, surfaced error in strict prod).

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
        // Never settle — wedged gateway socket.
        return new Promise<Response>(() => {});
      }) as unknown as typeof fetch;
    }

    it("settles to an EMPTY roster (source='unreachable') for a real user — never a perpetual skeleton", async () => {
      // Default beforeEach leaves all demo flags OFF → this is the real
      // signed-in-user path. The empty roster drives the page's
      // <TeamEmptyState/> ("hire your first teammate"), not a stuck skeleton.
      vi.useFakeTimers();
      try {
        installHangingGateway();
        const pending = fetchTeam();
        await vi.advanceTimersByTimeAsync(5001);
        const result = await pending;
        expect(result.source).toBe("unreachable");
        expect(result.workers).toEqual([]);
      } finally {
        vi.useRealTimers();
      }
    });

    it("surfaces a GatewayTimeoutError in strict production (no false empty roster)", async () => {
      // A real buyer must see an error state on a stalled gateway, NOT a
      // false "you have zero teammates". Strict production re-throws.
      vi.stubEnv("NODE_ENV", "production");
      vi.stubEnv("NEXT_PUBLIC_ENV", "production");
      vi.useFakeTimers();
      try {
        installHangingGateway();
        const pending = fetchTeam();
        // Attach the rejection assertion BEFORE advancing timers so the
        // rejection is observed (avoids an unhandled-rejection warning).
        const assertion = expect(pending).rejects.toThrow(/within 5000ms/);
        await vi.advanceTimersByTimeAsync(5001);
        await assertion;
      } finally {
        vi.useRealTimers();
      }
    });
  });
});
