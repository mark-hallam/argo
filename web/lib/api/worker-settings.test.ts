import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  budgetPercent,
  budgetTone,
  getSettings,
  updateBudget,
  WorkerSettingsNotFoundError,
} from "./worker-settings";
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

describe("lib/api/worker-settings · getSettings", () => {
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

  it("an ANONYMOUS dev under auth-bypass gets the mock settings, no network", async () => {
    process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS = "1";
    const fetchMock = installMock(() => makeResponse({ status: 500, body: "nope" }));

    const result = await getSettings("wkr-1");

    expect(result.source).toBe("mock-bypass");
    expect(result.settings.channels.primary).toBe("slack");
    expect(fetchMock).not.toHaveBeenCalled();
  });

  it("a REAL signed-in user is never served the bypass mock — they hit the gateway", async () => {
    process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS = "1";
    document.cookie = "skycot-session=1; path=/";
    const fetchMock = installMock(() =>
      makeResponse({
        status: 200,
        body: {
          channels: { all: ["email"], primary: "email" },
          budget: { daily_usd: 9, monthly_usd: 120 },
          autonomy: { level: "aggressive" },
        },
      }),
    );

    const result = await getSettings("wkr-1");

    expect(result.source).toBe("gateway");
    expect(result.settings.channels.primary).toBe("email");
    expect(result.settings.budget.dailyCapUsd).toBe(9);
    expect(result.settings.autonomy.level).toBe("aggressive");
    expect(fetchMock).toHaveBeenCalled();
  });

  it("maps the gateway snake_case envelope into the camelCase WorkerSettings shape", async () => {
    installMock(() =>
      makeResponse({
        status: 200,
        body: {
          channels: { all: ["slack", "email"], primary: "slack" },
          quiet_hours: { start: "22:00", end: "07:00", tz: "Australia/Melbourne" },
          budget: { daily_usd: 12, monthly_usd: 200, used_today_usd: 3.5 },
          autonomy: { level: "conservative" },
          advanced: { tenant_id: "tnt_x", namespace_id: "ns_y" },
        },
      }),
    );

    const result = await getSettings("wkr-1");

    expect(result.source).toBe("gateway");
    expect(result.settings.quietHours.timezone).toBe("Australia/Melbourne");
    expect(result.settings.budget.usedTodayUsd).toBe(3.5);
    expect(result.settings.advanced.tenantId).toBe("tnt_x");
    expect(result.settings.advanced.namespaceId).toBe("ns_y");
  });

  it("falls back to mock-offline for an anonymous dev on a gateway 404", async () => {
    installMock(() =>
      makeResponse({ status: 404, body: { error: { code: "not_found", message: "x" } } }),
    );

    const result = await getSettings("wkr-1");

    expect(result.source).toBe("mock-offline");
  });

  it("a REAL signed-in user surfaces a 404 as WorkerSettingsNotFoundError, NOT the mock", async () => {
    document.cookie = "skycot-session=1; path=/";
    installMock(() =>
      makeResponse({ status: 404, body: { error: { code: "not_found", message: "x" } } }),
    );

    await expect(getSettings("wkr-ghost")).rejects.toBeInstanceOf(
      WorkerSettingsNotFoundError,
    );
  });

  it("propagates a 5xx outage as ApiError", async () => {
    installMock(() => makeResponse({ status: 500, body: "boom", contentType: "text/plain" }));

    let caught: unknown;
    try {
      await getSettings("wkr-1");
    } catch (err) {
      caught = err;
    }
    expect(caught).toBeInstanceOf(ApiError);
    expect((caught as ApiError).status).toBe(500);
  });

  it("updateBudget PATCHes the budget endpoint and reports the gateway source", async () => {
    const seen: string[] = [];
    installMock((req) => {
      seen.push(`${req.method} ${req.url}`);
      return makeResponse({ status: 200, body: {} });
    });

    const result = await updateBudget("wkr-1", { dailyCapUsd: 7, monthlyCapUsd: 90 });

    expect(result.source).toBe("gateway");
    expect(seen.some((s) => s.includes("PATCH") && s.includes("/settings/budget"))).toBe(
      true,
    );
  });
});

describe("lib/api/worker-settings · budget helpers", () => {
  it("budgetPercent clamps to 0..100 and never returns NaN", () => {
    expect(budgetPercent(5, 10)).toBe(50);
    expect(budgetPercent(20, 10)).toBe(100);
    expect(budgetPercent(-5, 10)).toBe(0);
    expect(budgetPercent(5, 0)).toBe(0);
    expect(budgetPercent(Number.NaN, 10)).toBe(0);
  });

  it("budgetTone escalates muted → default → warning → destructive", () => {
    expect(budgetTone(10)).toBe("muted");
    expect(budgetTone(50)).toBe("default");
    expect(budgetTone(80)).toBe("warning");
    expect(budgetTone(95)).toBe("destructive");
  });
});
