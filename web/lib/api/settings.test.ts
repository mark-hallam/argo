import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  DEFAULT_NOTIFICATION_SETTINGS,
  DEFAULT_PROVIDER_SETTINGS,
  DEFAULT_WORKSPACE_SETTINGS,
  NEUTRAL_WORKSPACE_SETTINGS,
  fetchNotificationSettings,
  fetchProviderSettings,
  fetchWorkspaceSettings,
  updateNotificationSettings,
  updateProviderSettings,
  updateWorkspaceSettings,
} from "./settings";
import { ApiError } from "./types";

const ORIGINAL_FETCH = globalThis.fetch;

function clearCookies() {
  for (const pair of document.cookie.split("; ")) {
    const name = pair.split("=")[0];
    if (name) document.cookie = `${name}=; max-age=0; path=/`;
  }
}

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

describe("lib/api/settings", () => {
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
    clearCookies();
  });

  describe("workspace", () => {
    it("returns the gateway payload on 2xx", async () => {
      const payload = { ...DEFAULT_WORKSPACE_SETTINGS, name: "Real Tenant" };
      installMock(() => makeResponse({ status: 200, body: payload }));
      const result = await fetchWorkspaceSettings();
      expect(result.source).toBe("gateway");
      expect(result.value).toEqual(payload);
    });

    it("falls back to defaults on 404 in dev", async () => {
      installMock(() =>
        makeResponse({ status: 404, body: { error: { code: "x", message: "x" } } }),
      );
      const result = await fetchWorkspaceSettings();
      expect(result.source).toBe("mock-offline");
      expect(result.value).toEqual(DEFAULT_WORKSPACE_SETTINGS);
    });

    it("short-circuits to mock when auth-bypass is set", async () => {
      process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS = "1";
      const fetchMock = installMock(() => makeResponse({ status: 200, body: {} }));
      const result = await fetchWorkspaceSettings();
      expect(result.source).toBe("mock-bypass");
      expect(result.value).toEqual(DEFAULT_WORKSPACE_SETTINGS);
      expect(fetchMock).not.toHaveBeenCalled();
    });

    // Recording-mode opt-out (NO THEATER / DEC-007): with auth-bypass AND
    // demo-live both set, GET + PATCH must hit the real gateway so
    // Settings shows real, persisted values, not the bundled defaults.
    // Mirrors the canonical `isAuthBypassed() && !isDemoLive()` guard in
    // `lib/api/skills.ts`.
    it("hits the real gateway in demo-live even with auth-bypass set", async () => {
      process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS = "1";
      vi.stubEnv("NEXT_PUBLIC_SKYCOT_DEMO_LIVE", "1");
      const payload = { ...DEFAULT_WORKSPACE_SETTINGS, name: "Seeded Tenant" };
      const fetchMock = installMock(() => makeResponse({ status: 200, body: payload }));

      const read = await fetchWorkspaceSettings();
      expect(read.source).toBe("gateway");
      expect(read.value).toEqual(payload);

      const patched = await updateWorkspaceSettings(DEFAULT_WORKSPACE_SETTINGS, {
        name: "Seeded Tenant",
      });
      expect(patched.source).toBe("gateway");

      const hitWorkspace = fetchMock.mock.calls.some(([url]) =>
        String(url).includes("/api/v1/workspace"),
      );
      expect(hitWorkspace).toBe(true);
    });

    // Per-user gating (sprint, 2026-06-07): a REAL signed-in user (the
    // `skycot-session=1` marker cookie is present) must NEVER be served the
    // "SKYCOT Demo" seed — even in a deploy that sets auth-bypass. This was
    // the stranger-signup defect: a real user's Settings read "SKYCOT Demo"
    // as their workspace name. The auth-bypass mock short-circuit is for
    // ANONYMOUS demo visitors only.
    it("a REAL signed-in user is NOT auth-bypassed — hits the gateway even with auth-bypass set", async () => {
      process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS = "1";
      document.cookie = "skycot-session=1; path=/";
      const payload = { ...DEFAULT_WORKSPACE_SETTINGS, name: "Real Tenant" };
      const fetchMock = installMock(() => makeResponse({ status: 200, body: payload }));
      const result = await fetchWorkspaceSettings();
      expect(result.source).toBe("gateway");
      expect(result.value.name).toBe("Real Tenant");
      const hitWorkspace = fetchMock.mock.calls.some(([url]) =>
        String(url).includes("/api/v1/workspace"),
      );
      expect(hitWorkspace).toBe(true);
    });

    it("a REAL user falls back to BLANK fields (not 'SKYCOT Demo') on gateway 404", async () => {
      process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS = "1";
      document.cookie = "skycot-session=1; path=/";
      installMock(() =>
        makeResponse({ status: 404, body: { error: { code: "x", message: "x" } } }),
      );
      const result = await fetchWorkspaceSettings();
      expect(result.value).toEqual(NEUTRAL_WORKSPACE_SETTINGS);
      expect(result.value.name).toBe("");
      expect(result.value.name).not.toBe("SKYCOT Demo");
    });

    it("an ANONYMOUS visitor still gets the demo seed under auth-bypass", async () => {
      process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS = "1";
      // no skycot-session cookie → anonymous demo visitor
      const fetchMock = installMock(() => makeResponse({ status: 200, body: {} }));
      const result = await fetchWorkspaceSettings();
      expect(result.source).toBe("mock-bypass");
      expect(result.value).toEqual(DEFAULT_WORKSPACE_SETTINGS);
      expect(fetchMock).not.toHaveBeenCalled();
    });

    it("propagates 5xx as ApiError", async () => {
      installMock(() =>
        makeResponse({ status: 500, body: "boom", contentType: "text/plain" }),
      );
      await expect(fetchWorkspaceSettings()).rejects.toBeInstanceOf(ApiError);
    });

    it("propagates 404 in strict production", async () => {
      vi.stubEnv("NODE_ENV", "production");
      vi.stubEnv("NEXT_PUBLIC_ENV", "production");
      installMock(() =>
        makeResponse({ status: 404, body: { error: { code: "x", message: "x" } } }),
      );
      await expect(fetchWorkspaceSettings()).rejects.toBeInstanceOf(ApiError);
    });

    it("merges patch into previous in mock-offline mode", async () => {
      installMock(() =>
        makeResponse({ status: 404, body: { error: { code: "x", message: "x" } } }),
      );
      const result = await updateWorkspaceSettings(DEFAULT_WORKSPACE_SETTINGS, {
        name: "Renamed",
      });
      expect(result.source).toBe("mock-offline");
      expect(result.value.name).toBe("Renamed");
      expect(result.value.timezone).toBe(DEFAULT_WORKSPACE_SETTINGS.timezone);
    });

    it("sends the patch body as JSON to the gateway", async () => {
      const fetchMock = installMock(() =>
        makeResponse({
          status: 200,
          body: { ...DEFAULT_WORKSPACE_SETTINGS, name: "Persisted" },
        }),
      );
      const result = await updateWorkspaceSettings(DEFAULT_WORKSPACE_SETTINGS, {
        name: "Persisted",
      });
      expect(result.source).toBe("gateway");
      expect(result.value.name).toBe("Persisted");
      const gatewayCall = fetchMock.mock.calls.find(([url]) =>
        String(url).includes("/api/v1/workspace"),
      );
      expect(gatewayCall).toBeDefined();
      const [, init] = gatewayCall!;
      expect(init?.method).toBe("PATCH");
      expect(JSON.parse(String(init?.body))).toEqual({ name: "Persisted" });
    });
  });

  describe("providers", () => {
    it("returns gateway payload on 2xx", async () => {
      const payload = { ...DEFAULT_PROVIDER_SETTINGS, primary_provider: "openai" as const };
      installMock(() => makeResponse({ status: 200, body: payload }));
      const result = await fetchProviderSettings();
      expect(result.source).toBe("gateway");
      expect(result.value).toEqual(payload);
    });

    it("falls back to defaults on 404 in dev", async () => {
      installMock(() =>
        makeResponse({ status: 404, body: { error: { code: "x", message: "x" } } }),
      );
      const result = await fetchProviderSettings();
      expect(result.source).toBe("mock-offline");
      expect(result.value).toEqual(DEFAULT_PROVIDER_SETTINGS);
    });

    it("patches successfully in mock-offline mode", async () => {
      installMock(() =>
        makeResponse({ status: 404, body: { error: { code: "x", message: "x" } } }),
      );
      const result = await updateProviderSettings(DEFAULT_PROVIDER_SETTINGS, {
        primary_provider: "openai",
        allow_worker_overrides: false,
      });
      expect(result.source).toBe("mock-offline");
      expect(result.value.primary_provider).toBe("openai");
      expect(result.value.allow_worker_overrides).toBe(false);
    });
  });

  describe("notifications", () => {
    it("returns gateway payload on 2xx", async () => {
      const payload = { ...DEFAULT_NOTIFICATION_SETTINGS, daily_digest_time: "09:15" };
      installMock(() => makeResponse({ status: 200, body: payload }));
      const result = await fetchNotificationSettings();
      expect(result.source).toBe("gateway");
      expect(result.value.daily_digest_time).toBe("09:15");
    });

    it("falls back to defaults on 404 in dev", async () => {
      installMock(() =>
        makeResponse({ status: 404, body: { error: { code: "x", message: "x" } } }),
      );
      const result = await fetchNotificationSettings();
      expect(result.source).toBe("mock-offline");
      expect(result.value).toEqual(DEFAULT_NOTIFICATION_SETTINGS);
    });

    it("patches successfully in mock-offline mode", async () => {
      installMock(() =>
        makeResponse({ status: 404, body: { error: { code: "x", message: "x" } } }),
      );
      const result = await updateNotificationSettings(DEFAULT_NOTIFICATION_SETTINGS, {
        approval_escalation_minutes: 30,
        email_enabled: false,
      });
      expect(result.source).toBe("mock-offline");
      expect(result.value.approval_escalation_minutes).toBe(30);
      expect(result.value.email_enabled).toBe(false);
    });
  });
});
