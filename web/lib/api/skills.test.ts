import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { fetchSkills, SEED_SKILLS, type SkillRow } from "./skills";
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

const LIVE_SKILL: SkillRow = {
  id: "uuid-live-1",
  name: "approval.route",
  version: "0.4.2",
  description: "Live skill from the gateway.",
  layer: "workspace",
  provenance_source: "published:verified:team",
  trust_grade: "published_verified",
};

describe("lib/api/skills", () => {
  const originalEnv = { ...process.env };

  beforeEach(() => {
    process.env.NEXT_PUBLIC_GATEWAY_URL = "http://gw.test";
    vi.stubEnv("NODE_ENV", "test");
    delete process.env.NEXT_PUBLIC_ENV;
    delete process.env.NEXT_PUBLIC_GATEWAY_AUTH_BYPASS;
  });

  afterEach(() => {
    globalThis.fetch = ORIGINAL_FETCH;
    process.env = { ...originalEnv };
    vi.unstubAllEnvs();
    vi.restoreAllMocks();
  });

  it("unwraps the gateway envelope on 2xx happy path", async () => {
    const payload = { risk: "medium", risk_defaulted: true, count: 1, skills: [LIVE_SKILL] };
    installMock(() => makeResponse({ status: 200, body: payload }));
    const result = await fetchSkills();
    expect(result.source).toBe("gateway");
    expect(result.skills).toEqual([LIVE_SKILL]);
  });

  it("falls back to the seed catalog when the gateway returns no skills", async () => {
    installMock(() =>
      makeResponse({
        status: 200,
        body: { risk: "medium", risk_defaulted: true, count: 0, skills: [] },
      }),
    );
    const result = await fetchSkills();
    expect(result.source).toBe("mock-offline");
    expect(result.skills).toBe(SEED_SKILLS);
  });

  it("falls back to the seed catalog on 404 from a gateway without /skills", async () => {
    installMock(() =>
      makeResponse({
        status: 404,
        body: { error: { code: "not_found", message: "no skills endpoint" } },
      }),
    );
    const result = await fetchSkills();
    expect(result.source).toBe("mock-offline");
    expect(result.skills).toBe(SEED_SKILLS);
  });

  it("propagates 5xx as ApiError so callers see real outages", async () => {
    installMock(() =>
      makeResponse({ status: 503, body: "gateway down", contentType: "text/plain" }),
    );
    let caught: unknown;
    try {
      await fetchSkills();
    } catch (err) {
      caught = err;
    }
    expect(caught).toBeInstanceOf(ApiError);
    expect((caught as ApiError).status).toBe(503);
  });

  it("returns SEED_SKILLS immediately when the auth-bypass flag is set", async () => {
    process.env.NEXT_PUBLIC_GATEWAY_AUTH_BYPASS = "1";
    const fetchMock = installMock(() => makeResponse({ status: 200, body: {} }));
    const result = await fetchSkills();
    expect(result.source).toBe("mock-bypass");
    expect(result.skills).toBe(SEED_SKILLS);
    expect(fetchMock).not.toHaveBeenCalled();
  });

  it("propagates 404 in strict production (no mock swallow)", async () => {
    vi.stubEnv("NODE_ENV", "production");
    process.env.NEXT_PUBLIC_ENV = "production";
    installMock(() =>
      makeResponse({
        status: 404,
        body: { error: { code: "not_found", message: "missing" } },
      }),
    );
    let caught: unknown;
    try {
      await fetchSkills();
    } catch (err) {
      caught = err;
    }
    expect(caught).toBeInstanceOf(ApiError);
    expect((caught as ApiError).status).toBe(404);
  });
});
