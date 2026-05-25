import { afterEach, describe, expect, it, vi } from "vitest";
import { NextRequest } from "next/server";
import { POST } from "./route";

const GATEWAY = "http://127.0.0.1:7400";
const PRINCIPAL = "human:demo@skycot.com";
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
});

describe("POST /api/auth/demo-mint — needs_onboarding derivation", () => {
  it("returns needs_onboarding=false when the tenant has workers + a vertical (seeded)", async () => {
    const fetchSpy = vi
      .spyOn(globalThis, "fetch")
      .mockImplementation(async (input) => {
        const url = typeof input === "string" ? input : input.toString();
        if (url.endsWith("/api/v1/demo/mint-principal")) return mintOkResponse();
        if (url.endsWith("/api/v1/team")) {
          return jsonResponse({
            workers: Array.from({ length: 5 }, (_, i) => ({ id: `w-${i}` })),
            total_count: 5,
          });
        }
        if (url.endsWith("/api/v1/verticals")) {
          return jsonResponse({
            verticals: [{ id: "gtm-engineer", name: "GTM Engineer", workflow_count: 3 }],
          });
        }
        throw new Error(`unexpected fetch: ${url}`);
      });

    const res = await POST(makeRequest({ email: "demo@skycot.com" }));
    expect(res.status).toBe(200);
    const body = (await res.json()) as RouteBody;
    expect(body.ok).toBe(true);
    expect(body.needs_onboarding).toBe(false);
    expect(body.principal).toBe(PRINCIPAL);
    expect(body.session_token).toBe(TOKEN);

    // Sanity: gateway probes ran with Bearer auth from the freshly minted token.
    const teamCall = fetchSpy.mock.calls.find(([input]) =>
      String(input).endsWith("/api/v1/team"),
    );
    expect(teamCall).toBeDefined();
    const teamInit = teamCall?.[1] as RequestInit | undefined;
    expect(
      (teamInit?.headers as Record<string, string>)?.Authorization,
    ).toBe(`Bearer ${TOKEN}`);
  });

  it("returns needs_onboarding=true for a never-seeded tenant (empty roster + no verticals)", async () => {
    vi.spyOn(globalThis, "fetch").mockImplementation(async (input) => {
      const url = typeof input === "string" ? input : input.toString();
      if (url.endsWith("/api/v1/demo/mint-principal")) return mintOkResponse();
      if (url.endsWith("/api/v1/team")) {
        return jsonResponse({ workers: [], total_count: 0 });
      }
      if (url.endsWith("/api/v1/verticals")) {
        return jsonResponse({ verticals: [] });
      }
      throw new Error(`unexpected fetch: ${url}`);
    });

    const res = await POST(makeRequest({ email: "fresh@skycot.com" }));
    expect(res.status).toBe(200);
    const body = (await res.json()) as RouteBody;
    expect(body.ok).toBe(true);
    expect(body.needs_onboarding).toBe(true);
  });

  it("defaults to needs_onboarding=true when only workers are present (no vertical installed)", async () => {
    vi.spyOn(globalThis, "fetch").mockImplementation(async (input) => {
      const url = typeof input === "string" ? input : input.toString();
      if (url.endsWith("/api/v1/demo/mint-principal")) return mintOkResponse();
      if (url.endsWith("/api/v1/team")) {
        return jsonResponse({ workers: [{ id: "w-1" }], total_count: 1 });
      }
      if (url.endsWith("/api/v1/verticals")) {
        return jsonResponse({ verticals: [] });
      }
      throw new Error(`unexpected fetch: ${url}`);
    });

    const res = await POST(makeRequest({ email: "partial@skycot.com" }));
    expect(res.status).toBe(200);
    const body = (await res.json()) as RouteBody;
    expect(body.needs_onboarding).toBe(true);
  });

  it("falls back to needs_onboarding=true when a gateway probe fails", async () => {
    vi.spyOn(globalThis, "fetch").mockImplementation(async (input) => {
      const url = typeof input === "string" ? input : input.toString();
      if (url.endsWith("/api/v1/demo/mint-principal")) return mintOkResponse();
      if (url.endsWith("/api/v1/team")) {
        return new Response("", { status: 500 });
      }
      if (url.endsWith("/api/v1/verticals")) {
        return jsonResponse({ verticals: [{ id: "gtm-engineer" }] });
      }
      throw new Error(`unexpected fetch: ${url}`);
    });

    const res = await POST(makeRequest({ email: "broken@skycot.com" }));
    expect(res.status).toBe(200);
    const body = (await res.json()) as RouteBody;
    expect(body.needs_onboarding).toBe(true);
  });

  it("rejects requests without an email", async () => {
    const res = await POST(makeRequest({}));
    expect(res.status).toBe(400);
    const body = (await res.json()) as RouteBody;
    expect(body.error?.code).toBe("missing_email");
  });

  it("surfaces gateway errors when mint-principal fails", async () => {
    vi.spyOn(globalThis, "fetch").mockImplementation(async (input) => {
      const url = typeof input === "string" ? input : input.toString();
      if (url.endsWith("/api/v1/demo/mint-principal")) {
        return new Response(JSON.stringify({ error: "demo mode off" }), {
          status: 404,
          headers: { "content-type": "application/json" },
        });
      }
      throw new Error(`unexpected fetch: ${url}`);
    });

    const res = await POST(makeRequest({ email: "demo@skycot.com" }));
    expect(res.status).toBe(404);
    const body = (await res.json()) as RouteBody;
    expect(body.error?.code).toBe("demo_mode_off");
  });

  it("sets the __Host-session and __Host-csrf cookies on success", async () => {
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

    const res = await POST(makeRequest({ email: "demo@skycot.com" }));
    const cookies = res.headers.getSetCookie();
    expect(cookies.some((c) => c.startsWith("__Host-session="))).toBe(true);
    expect(cookies.some((c) => c.startsWith("__Host-csrf="))).toBe(true);
  });

  // Gateway returns the team body in the form { workers: [...], total_count: N }
  // — derive from `total_count` when present, but also tolerate older shapes
  // that send `workers` only (mirrors `lib/api/team.ts`'s `Array.isArray` fallback).
  it("derives needs_onboarding from `workers.length` when `total_count` is absent", async () => {
    vi.spyOn(globalThis, "fetch").mockImplementation(async (input) => {
      const url = typeof input === "string" ? input : input.toString();
      if (url.endsWith("/api/v1/demo/mint-principal")) return mintOkResponse();
      if (url.endsWith("/api/v1/team")) {
        return jsonResponse({ workers: [{ id: "w-1" }, { id: "w-2" }] });
      }
      if (url.endsWith("/api/v1/verticals")) {
        return jsonResponse({ verticals: [{ id: "gtm-engineer" }] });
      }
      throw new Error(`unexpected fetch: ${url}`);
    });

    const res = await POST(makeRequest({ email: "legacy@skycot.com" }));
    const body = (await res.json()) as RouteBody;
    expect(body.needs_onboarding).toBe(false);
  });
});
