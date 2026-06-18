import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { apiGet } from "./client";
import {
  clearTenantOverride,
  getTenantOverride,
  resolveTenantId,
  setTenantOverride,
} from "./tenant";

const ORIGINAL_FETCH = globalThis.fetch;

function makeResponse({
  status = 200,
  body,
}: {
  status?: number;
  body?: unknown;
}): Response {
  const headers = new Headers();
  headers.set("content-type", "application/json");
  return new Response(body === undefined ? null : JSON.stringify(body), {
    status,
    statusText: `Status ${status}`,
    headers,
  });
}

function installMock(
  sessionBody: unknown,
  gatewayResponder: (req: Request) => Response,
) {
  const fetchMock = vi.fn((input: RequestInfo | URL, init?: RequestInit) => {
    const url =
      typeof input === "string"
        ? input
        : input instanceof URL
          ? input.toString()
          : input.url;
    if (url.endsWith("/api/auth/session")) {
      return Promise.resolve(makeResponse({ status: 200, body: sessionBody }));
    }
    if (url.endsWith("/api/principal")) {
      return Promise.resolve(makeResponse({ status: 200, body: { principal: "human:test@example.com" } }));
    }
    const req = new Request(url, init);
    return Promise.resolve(gatewayResponder(req));
  });
  globalThis.fetch = fetchMock as unknown as typeof fetch;
  return fetchMock;
}

describe("lib/api/tenant resolver", () => {
  beforeEach(() => {
    clearTenantOverride();
  });
  afterEach(() => {
    clearTenantOverride();
    globalThis.fetch = ORIGINAL_FETCH;
    vi.restoreAllMocks();
  });

  describe("resolveTenantId precedence", () => {
    it("returns undefined when no override and no session.tenantId", () => {
      expect(resolveTenantId({})).toBeUndefined();
    });

    it("returns session.tenantId when no override is set", () => {
      expect(resolveTenantId({ tenantId: "t-from-session" })).toBe(
        "t-from-session",
      );
    });

    it("override wins over session.tenantId", () => {
      setTenantOverride("t-from-switcher");
      expect(resolveTenantId({ tenantId: "t-from-session" })).toBe(
        "t-from-switcher",
      );
    });

    it("setTenantOverride('') is treated as undefined (clear)", () => {
      setTenantOverride("");
      expect(getTenantOverride()).toBeUndefined();
      expect(resolveTenantId({ tenantId: "t-from-session" })).toBe(
        "t-from-session",
      );
    });

    it("empty session.tenantId is treated as absent", () => {
      expect(resolveTenantId({ tenantId: "" })).toBeUndefined();
    });

    it("clearTenantOverride drops the pinned id", () => {
      setTenantOverride("t-1");
      expect(getTenantOverride()).toBe("t-1");
      clearTenantOverride();
      expect(getTenantOverride()).toBeUndefined();
      expect(resolveTenantId({})).toBeUndefined();
    });
  });

  describe("X-Argo-Tenant-Id wiring through client.request", () => {
    beforeEach(() => {
      process.env.NEXT_PUBLIC_GATEWAY_URL = "http://gw.test";
    });

    it("omits the header when no override and session has no tenantId", async () => {
      const seen: { tenant: string | null }[] = [];
      installMock({}, (req) => {
        seen.push({ tenant: req.headers.get("X-Argo-Tenant-Id") });
        return makeResponse({ status: 200, body: {} });
      });
      await apiGet("/api/v1/x");
      expect(seen).toHaveLength(1);
      expect(seen[0].tenant).toBeNull();
    });

    it("stamps the header from session.tenantId when present", async () => {
      const seen: { tenant: string | null }[] = [];
      installMock({ tenantId: "tenant-from-jwt" }, (req) => {
        seen.push({ tenant: req.headers.get("X-Argo-Tenant-Id") });
        return makeResponse({ status: 200, body: {} });
      });
      await apiGet("/api/v1/x");
      expect(seen[0].tenant).toBe("tenant-from-jwt");
    });

    it("stamps the override even when session.tenantId is unset", async () => {
      const seen: { tenant: string | null }[] = [];
      installMock({}, (req) => {
        seen.push({ tenant: req.headers.get("X-Argo-Tenant-Id") });
        return makeResponse({ status: 200, body: {} });
      });
      setTenantOverride("tenant-switcher-pick");
      await apiGet("/api/v1/x");
      expect(seen[0].tenant).toBe("tenant-switcher-pick");
    });

    it("override wins over session.tenantId on the wire", async () => {
      const seen: { tenant: string | null }[] = [];
      installMock({ tenantId: "tenant-from-jwt" }, (req) => {
        seen.push({ tenant: req.headers.get("X-Argo-Tenant-Id") });
        return makeResponse({ status: 200, body: {} });
      });
      setTenantOverride("tenant-switcher-pick");
      await apiGet("/api/v1/x");
      expect(seen[0].tenant).toBe("tenant-switcher-pick");
    });

    it("caller-supplied X-Argo-Tenant-Id header is preserved (resolver does not overwrite)", async () => {
      const seen: { tenant: string | null }[] = [];
      installMock({ tenantId: "tenant-from-jwt" }, (req) => {
        seen.push({ tenant: req.headers.get("X-Argo-Tenant-Id") });
        return makeResponse({ status: 200, body: {} });
      });
      setTenantOverride("tenant-switcher-pick");
      await apiGet("/api/v1/x", {
        headers: { "X-Argo-Tenant-Id": "caller-supplied-id" },
      });
      expect(seen[0].tenant).toBe("caller-supplied-id");
    });
  });
});
