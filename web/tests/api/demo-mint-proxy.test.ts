/**
 * Integration tests for the `/api/auth/demo-mint` proxy route.
 *
 * Pre-fix history: the route surfaced `gateway_unreachable` on any
 * `fetch` throw without distinguishing "gateway is down" from "the
 * mint route itself misbehaved while the gateway is healthy". The
 * 2026-05-26 onboarding-hire-an-ai-teammate F6 [P0] audit traced the
 * v1.0 demo-recording handoff break to that confusion.
 *
 * These tests pin the two behaviours that the fix introduces:
 *  1. Env-var resolution prefers the server-only `GATEWAY_URL` over
 *     the build-time-injected `NEXT_PUBLIC_GATEWAY_URL`.
 *  2. On a fetch throw, the route probes `/health` and surfaces a
 *     distinct `gateway_demo_mint_failed` code when the gateway is
 *     healthy — so the operator (and the audit log) can see that the
 *     mint route is the bug, not network reachability.
 *
 * Sibling `route.test.ts` covers the happy-path + needs_onboarding
 * derivation cases.
 */
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { NextRequest } from "next/server";
import { POST } from "@/app/api/auth/demo-mint/route";

const PRINCIPAL = "human:demo@skycot.test";
const TOKEN = "demo-session-jwt-token-value-32-chars-and-then-some-more-bytes";

interface RouteBody {
  ok?: boolean;
  needs_onboarding?: boolean;
  principal?: string;
  session_token?: string;
  error?: { code: string; message: string };
}

function jsonResponse(body: unknown, status = 200): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "content-type": "application/json" },
  });
}

function makeRequest(body: unknown): NextRequest {
  return new NextRequest("http://localhost:3000/api/auth/demo-mint", {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify(body),
  });
}

function mintOkResponse(): Response {
  return jsonResponse({
    ok: true,
    principal: PRINCIPAL,
    session_token: TOKEN,
  });
}

afterEach(() => {
  vi.restoreAllMocks();
  delete process.env.GATEWAY_URL;
  delete process.env.NEXT_PUBLIC_GATEWAY_URL;
});

describe("POST /api/auth/demo-mint — env resolution", () => {
  it("prefers server-only GATEWAY_URL over NEXT_PUBLIC_GATEWAY_URL", async () => {
    process.env.GATEWAY_URL = "http://127.0.0.1:9991";
    process.env.NEXT_PUBLIC_GATEWAY_URL = "http://127.0.0.1:9992";

    const calls: string[] = [];
    vi.spyOn(globalThis, "fetch").mockImplementation(async (input) => {
      const url = typeof input === "string" ? input : input.toString();
      calls.push(url);
      if (url.endsWith("/api/v1/demo/mint-principal")) return mintOkResponse();
      if (url.endsWith("/api/v1/team")) {
        return jsonResponse({ workers: [], total_count: 0 });
      }
      if (url.endsWith("/api/v1/verticals")) {
        return jsonResponse({ verticals: [] });
      }
      throw new Error(`unexpected fetch: ${url}`);
    });

    const res = await POST(makeRequest({ email: "demo@skycot.test" }));
    expect(res.status).toBe(200);
    const mintCall = calls.find((u) => u.endsWith("/api/v1/demo/mint-principal"));
    expect(mintCall).toBeDefined();
    expect(mintCall!.startsWith("http://127.0.0.1:9991/")).toBe(true);
    expect(mintCall!.startsWith("http://127.0.0.1:9992/")).toBe(false);
  });

  it("falls back to NEXT_PUBLIC_GATEWAY_URL when GATEWAY_URL is unset", async () => {
    process.env.NEXT_PUBLIC_GATEWAY_URL = "http://127.0.0.1:9993";

    const calls: string[] = [];
    vi.spyOn(globalThis, "fetch").mockImplementation(async (input) => {
      const url = typeof input === "string" ? input : input.toString();
      calls.push(url);
      if (url.endsWith("/api/v1/demo/mint-principal")) return mintOkResponse();
      if (url.endsWith("/api/v1/team")) {
        return jsonResponse({ workers: [], total_count: 0 });
      }
      if (url.endsWith("/api/v1/verticals")) {
        return jsonResponse({ verticals: [] });
      }
      throw new Error(`unexpected fetch: ${url}`);
    });

    const res = await POST(makeRequest({ email: "demo@skycot.test" }));
    expect(res.status).toBe(200);
    const mintCall = calls.find((u) => u.endsWith("/api/v1/demo/mint-principal"));
    expect(mintCall!.startsWith("http://127.0.0.1:9993/")).toBe(true);
  });
});

describe("POST /api/auth/demo-mint — health-probe disambiguation", () => {
  beforeEach(() => {
    process.env.NEXT_PUBLIC_GATEWAY_URL = "http://127.0.0.1:7400";
  });

  it("returns gateway_demo_mint_failed when mint throws but /health is 200", async () => {
    vi.spyOn(globalThis, "fetch").mockImplementation(async (input) => {
      const url = typeof input === "string" ? input : input.toString();
      if (url.endsWith("/api/v1/demo/mint-principal")) {
        throw new TypeError("fetch failed: socket hang up");
      }
      if (url.endsWith("/health")) {
        return jsonResponse({ status: "ok", schema_version: 83 });
      }
      throw new Error(`unexpected fetch: ${url}`);
    });

    const res = await POST(makeRequest({ email: "demo@skycot.test" }));
    expect(res.status).toBe(502);
    const body = (await res.json()) as RouteBody;
    expect(body.error?.code).toBe("gateway_demo_mint_failed");
    expect(body.error?.message).toContain("Gateway is up");
    expect(body.error?.message).toContain("socket hang up");
  });

  it("returns gateway_unreachable when both mint and /health throw", async () => {
    vi.spyOn(globalThis, "fetch").mockImplementation(async (input) => {
      const url = typeof input === "string" ? input : input.toString();
      if (url.endsWith("/api/v1/demo/mint-principal")) {
        throw new TypeError("fetch failed: ECONNREFUSED");
      }
      if (url.endsWith("/health")) {
        throw new TypeError("fetch failed: ECONNREFUSED");
      }
      throw new Error(`unexpected fetch: ${url}`);
    });

    const res = await POST(makeRequest({ email: "demo@skycot.test" }));
    expect(res.status).toBe(502);
    const body = (await res.json()) as RouteBody;
    expect(body.error?.code).toBe("gateway_unreachable");
  });

  it("returns gateway_unreachable when mint throws and /health is 5xx", async () => {
    vi.spyOn(globalThis, "fetch").mockImplementation(async (input) => {
      const url = typeof input === "string" ? input : input.toString();
      if (url.endsWith("/api/v1/demo/mint-principal")) {
        throw new TypeError("fetch failed");
      }
      if (url.endsWith("/health")) {
        return new Response("", { status: 503 });
      }
      throw new Error(`unexpected fetch: ${url}`);
    });

    const res = await POST(makeRequest({ email: "demo@skycot.test" }));
    expect(res.status).toBe(502);
    const body = (await res.json()) as RouteBody;
    expect(body.error?.code).toBe("gateway_unreachable");
  });
});

describe("POST /api/auth/demo-mint — happy-path cookie contract", () => {
  beforeEach(() => {
    process.env.NEXT_PUBLIC_GATEWAY_URL = "http://127.0.0.1:7400";
  });

  // Acceptance criteria from the brief: with the gateway on :7400
  // healthy, POST returns HTTP 200 with a `Set-Cookie` header for
  // `__Host-session`. This is the visible signup-button behaviour
  // that the F6 audit reported as broken.
  it("returns 200 + __Host-session cookie when the gateway is alive", async () => {
    vi.spyOn(globalThis, "fetch").mockImplementation(async (input) => {
      const url = typeof input === "string" ? input : input.toString();
      if (url.endsWith("/api/v1/demo/mint-principal")) return mintOkResponse();
      if (url.endsWith("/api/v1/team")) {
        return jsonResponse({ workers: [{ id: "w-1" }], total_count: 1 });
      }
      if (url.endsWith("/api/v1/verticals")) {
        return jsonResponse({ verticals: [{ id: "gtm-engineer" }] });
      }
      throw new Error(`unexpected fetch: ${url}`);
    });

    const res = await POST(makeRequest({ email: "demo+x@skycot.test" }));
    expect(res.status).toBe(200);
    const cookies = res.headers.getSetCookie();
    expect(cookies.some((c) => c.startsWith("__Host-session="))).toBe(true);
    expect(cookies.some((c) => c.startsWith("__Host-csrf="))).toBe(true);
  });
});
