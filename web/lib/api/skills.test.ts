import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

// `installSkillToWorkers` resolves its default target roster via
// `fetchTeam`. Mock the team module so the install tests can control the
// roster without standing up the full team fetcher contract.
vi.mock("./team", () => ({ fetchTeam: vi.fn() }));

import {
  fetchLearnedThisMonth,
  fetchSkills,
  fetchSkillUsage,
  installSkillToWorkers,
  resolveSkillDisplayMetadata,
  SEED_SKILLS,
  type SkillRow,
} from "./skills";
import { fetchTeam } from "./team";
import { ApiError } from "./types";

const mockedFetchTeam = vi.mocked(fetchTeam);

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
    delete process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS;
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
    process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS = "1";
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

  it("returns learned seed ids when the learned-this-month endpoint is unavailable", async () => {
    installMock(() =>
      makeResponse({
        status: 404,
        body: { error: { code: "not_found", message: "missing" } },
      }),
    );

    const result = await fetchLearnedThisMonth();

    expect(result.source).toBe("mock-offline");
    expect(result.learned.count).toBe(2);
    expect(result.learned.skill_ids).toEqual(["support.triage", "lead.enrich"]);
  });

  it("seeds the pricing.competitor_scrape description with the runbook phrase as a substring", () => {
    const seed = SEED_SKILLS.find((s) => s.name === "pricing.competitor_scrape");
    expect(seed).toBeDefined();
    expect(seed!.description.toLowerCase()).toContain("competitor pricing scrape");
  });

  it("varies seed usage provenance across SKYCOT, org, and learned skills", async () => {
    installMock(() =>
      makeResponse({
        status: 404,
        body: { error: { code: "not_found", message: "missing" } },
      }),
    );

    const [skycot, org, learned] = await Promise.all([
      fetchSkillUsage("approval.route"),
      fetchSkillUsage("deal.qualify"),
      fetchSkillUsage("support.triage"),
    ]);

    expect(skycot.usage.author_label).toBe("Authored by SKYCOT");
    expect(org.usage.author_label).toBe("Authored by your org");
    expect(learned.usage.author_label).toBe("Learned by Mariah · 2026-05-23");
    expect(learned.usage.workers_using).toEqual([
      {
        worker_id: "wkr-mariah",
        name: "Mariah",
        learned_at: "2026-05-23T09:15:00.000Z",
      },
    ]);
  });

  describe("resolveSkillDisplayMetadata", () => {
    it("returns seed name + description when the skill id is known", () => {
      const meta = resolveSkillDisplayMetadata("pricing.competitor_scrape");
      expect(meta.title).toBe("pricing.competitor_scrape");
      expect(meta.description).toBe(
        SEED_SKILLS.find((s) => s.name === "pricing.competitor_scrape")!.description,
      );
    });

    it("falls back to the skill id and a generic description when unknown", () => {
      const meta = resolveSkillDisplayMetadata("brand.new.skill");
      expect(meta.title).toBe("brand.new.skill");
      expect(meta.description).toContain("brand.new.skill");
    });
  });
});

const SKILL_UUID = "11111111-1111-1111-1111-111111111111";

function teamWith(...ids: string[]) {
  return {
    workers: ids.map((id) => ({ id, name: id })),
    source: "gateway",
  } as unknown as Awaited<ReturnType<typeof fetchTeam>>;
}

describe("lib/api/skills · installSkillToWorkers", () => {
  const originalEnv = { ...process.env };

  beforeEach(() => {
    process.env.NEXT_PUBLIC_GATEWAY_URL = "http://gw.test";
    vi.stubEnv("NODE_ENV", "test");
    delete process.env.NEXT_PUBLIC_ENV;
    delete process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS;
    mockedFetchTeam.mockReset();
  });

  afterEach(() => {
    globalThis.fetch = ORIGINAL_FETCH;
    process.env = { ...originalEnv };
    vi.unstubAllEnvs();
    vi.restoreAllMocks();
  });

  it("POSTs the skill_id to each resolved worker and reports the gateway source", async () => {
    mockedFetchTeam.mockResolvedValue(teamWith("w1", "w2"));
    const seen: Array<{ url: string; body: string }> = [];
    installMock((req) => {
      seen.push({ url: req.url, body: "" });
      return makeResponse({ status: 201, body: { worker_id: "w", skill_id: SKILL_UUID } });
    });

    const result = await installSkillToWorkers(SKILL_UUID);

    expect(result.source).toBe("gateway");
    expect(result.workerIds).toEqual(["w1", "w2"]);
    // One POST per worker, each hitting the real worker-skills endpoint.
    const skillPosts = seen.filter((s) => s.url.includes("/skills"));
    expect(skillPosts).toHaveLength(2);
    expect(skillPosts[0].url).toContain("/api/v1/workers/w1/skills");
    expect(skillPosts[1].url).toContain("/api/v1/workers/w2/skills");
  });

  it("sends the JSON body { skill_id } the gateway add_response expects", async () => {
    mockedFetchTeam.mockResolvedValue(teamWith("w1"));
    let captured: unknown = null;
    const fetchMock = vi.fn(async (input: RequestInfo | URL, init?: RequestInit) => {
      const url =
        typeof input === "string" ? input : input instanceof URL ? input.toString() : input.url;
      if (url.endsWith("/api/auth/session")) {
        return makeResponse({ status: 200, body: {} });
      }
      captured = init?.body ? JSON.parse(init.body as string) : null;
      return makeResponse({ status: 201, body: {} });
    });
    globalThis.fetch = fetchMock as unknown as typeof fetch;

    await installSkillToWorkers(SKILL_UUID, ["w1"]);

    expect(captured).toEqual({ skill_id: SKILL_UUID });
  });

  it("uses the explicit worker list without calling fetchTeam", async () => {
    installMock(() => makeResponse({ status: 201, body: {} }));
    const result = await installSkillToWorkers(SKILL_UUID, ["only-one"]);
    expect(result.workerIds).toEqual(["only-one"]);
    expect(mockedFetchTeam).not.toHaveBeenCalled();
  });

  it("short-circuits to a mock success under the auth-bypass flag (no network)", async () => {
    process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS = "1";
    const fetchMock = installMock(() => makeResponse({ status: 200, body: {} }));

    const result = await installSkillToWorkers(SKILL_UUID);

    expect(result.source).toBe("mock-bypass");
    expect(result.workerIds.length).toBeGreaterThan(0);
    expect(fetchMock).not.toHaveBeenCalled();
    expect(mockedFetchTeam).not.toHaveBeenCalled();
  });

  it("falls back to a mock success on a 404 from a gateway without the endpoint", async () => {
    mockedFetchTeam.mockResolvedValue(teamWith("w1"));
    installMock(() =>
      makeResponse({
        status: 404,
        body: { error: { code: "not_found", message: "no such worker route" } },
      }),
    );

    const result = await installSkillToWorkers(SKILL_UUID);

    expect(result.source).toBe("mock-offline");
    expect(result.workerIds).toEqual(["w1"]);
  });

  it("throws a 409 when there are no workers to install to", async () => {
    mockedFetchTeam.mockResolvedValue(teamWith());
    let caught: unknown;
    try {
      await installSkillToWorkers(SKILL_UUID);
    } catch (err) {
      caught = err;
    }
    expect(caught).toBeInstanceOf(ApiError);
    expect((caught as ApiError).status).toBe(409);
    expect((caught as ApiError).message).toContain("hire a teammate");
  });

  it("propagates a 5xx install failure as ApiError (real outage, no mock swallow)", async () => {
    installMock(() => makeResponse({ status: 500, body: "boom", contentType: "text/plain" }));
    let caught: unknown;
    try {
      await installSkillToWorkers(SKILL_UUID, ["w1"]);
    } catch (err) {
      caught = err;
    }
    expect(caught).toBeInstanceOf(ApiError);
    expect((caught as ApiError).status).toBe(500);
  });
});
