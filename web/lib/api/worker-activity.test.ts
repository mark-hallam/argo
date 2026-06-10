import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  fetchWorkerActivity,
  rangeCeiling,
  rangeFloor,
  type WorkerActivityFilters,
} from "./worker-activity";
import { ApiError } from "./types";

const ORIGINAL_FETCH = globalThis.fetch;

const FILTERS: WorkerActivityFilters = { range: "7d" };

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

describe("lib/api/worker-activity · fetchWorkerActivity", () => {
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

  it("an ANONYMOUS dev under auth-bypass gets a client-side mock page, no network", async () => {
    process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS = "1";
    const fetchMock = installMock(() => makeResponse({ status: 500, body: "nope" }));

    const page = await fetchWorkerActivity("wkr-1", FILTERS);

    expect(Array.isArray(page.rows)).toBe(true);
    expect(fetchMock).not.toHaveBeenCalled();
  });

  it("a REAL signed-in user is never served the mock — they hit the gateway", async () => {
    process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS = "1";
    document.cookie = "skycot-session=1; path=/";
    const fetchMock = installMock((req) => {
      expect(req.url).toContain("/api/v1/workers/wkr-1/activity");
      return makeResponse({ status: 200, body: { items: [], next_cursor: null } });
    });

    const page = await fetchWorkerActivity("wkr-1", FILTERS);

    expect(page.rows).toEqual([]);
    expect(fetchMock).toHaveBeenCalled();
  });

  it("normalizes the gateway {items:[...]} envelope into WorkerActivityPage rows", async () => {
    installMock(() =>
      makeResponse({
        status: 200,
        body: {
          items: [
            {
              id: "act_1",
              timestamp: "2026-05-27T05:20:00Z",
              worker: { id: "wkr-1", name: "Mariah Quinn" },
              action_type: "memory_written",
              verb: "wrote memory",
              status: "succeeded",
              cost: 0.012,
            },
          ],
          next_cursor: "cursor-2",
        },
      }),
    );

    const page = await fetchWorkerActivity("wkr-1", FILTERS);

    expect(page.rows).toHaveLength(1);
    expect(page.rows[0]).toMatchObject({
      id: "act_1",
      workerName: "Mariah Quinn",
      workerInitials: "MQ",
      actionType: "memory_written",
      status: "succeeded",
      costUsd: 0.012,
    });
    expect(page.nextCursor).toBe("cursor-2");
  });

  it("encodes range + filters into the gateway query string", async () => {
    const fetchMock = installMock(() =>
      makeResponse({ status: 200, body: { items: [], next_cursor: null } }),
    );

    await fetchWorkerActivity(
      "wkr-1",
      { range: "today", actionType: "approval_asked", status: "pending" },
      "50",
    );

    const url = String(fetchMock.mock.calls.find((c) => String(c[0]).includes("activity"))?.[0] ?? "");
    expect(url).toContain("range=today");
    expect(url).toContain("type=approval_asked");
    expect(url).toContain("status=pending");
    expect(url).toContain("cursor=50");
  });

  it("falls back to a client-side mock page on a 404 in dev", async () => {
    installMock(() =>
      makeResponse({ status: 404, body: { error: { code: "not_found", message: "x" } } }),
    );

    const page = await fetchWorkerActivity("wkr-1", FILTERS);

    expect(Array.isArray(page.rows)).toBe(true);
  });

  it("a REAL signed-in user propagates a 5xx outage as ApiError", async () => {
    document.cookie = "skycot-session=1; path=/";
    installMock(() => makeResponse({ status: 500, body: "boom", contentType: "text/plain" }));

    let caught: unknown;
    try {
      await fetchWorkerActivity("wkr-1", FILTERS);
    } catch (err) {
      caught = err;
    }
    expect(caught).toBeInstanceOf(ApiError);
    expect((caught as ApiError).status).toBe(500);
  });
});

describe("lib/api/worker-activity · range bounds", () => {
  const NOW = Date.parse("2026-05-27T05:20:00.000Z");

  it("rangeFloor('1h') is exactly one hour before now", () => {
    expect(rangeFloor("1h", NOW)).toBe(NOW - 60 * 60_000);
  });

  it("rangeFloor('7d') is seven days before now", () => {
    expect(rangeFloor("7d", NOW)).toBe(NOW - 7 * 24 * 60 * 60_000);
  });

  it("rangeCeiling caps 'yesterday' at local midnight and leaves other ranges at now", () => {
    expect(rangeCeiling("7d", NOW)).toBe(NOW);
    expect(rangeCeiling("yesterday", NOW)).toBeLessThan(NOW);
  });
});
