import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { fetchMemory } from "./worker-memory";

const ORIGINAL_FETCH = globalThis.fetch;

function makeResponse(body: unknown, status = 200): Response {
  const headers = new Headers();
  headers.set("content-type", "application/json");
  return new Response(JSON.stringify(body), {
    status,
    statusText: `Status ${status}`,
    headers,
  });
}

function installMock(responder: (url: string) => Response) {
  const fetchMock = vi.fn((input: RequestInfo | URL) => {
    const url =
      typeof input === "string" ? input : input instanceof URL ? input.toString() : input.url;
    if (url.endsWith("/api/auth/session")) {
      return Promise.resolve(makeResponse({}));
    }
    return Promise.resolve(responder(url));
  });
  globalThis.fetch = fetchMock as unknown as typeof fetch;
  return fetchMock;
}

describe("lib/api/worker-memory normalizer", () => {
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

  it("maps snake_case fact fields to camelCase so the date is parseable", async () => {
    // Regression for the LIVE walkthrough "Invalid Date": a gateway
    // payload whose facts use the legacy snake_case `fact_id` /
    // `created_at` must still normalize to a camelCase `createdAt` the
    // Memory tab can `new Date(...)` without producing "Invalid Date".
    installMock(() =>
      makeResponse({
        namespace_id: "ns/x",
        facts: [
          { fact_id: "f1", text: "Bill Northwind NET-30", created_at: "2026-05-30T10:00:00+00:00" },
        ],
        active_conversations: [],
        recent_decisions: [],
        voice_profile: { tone: "warm", formality: "professional", lexicon: [] },
      }),
    );

    const { memory, source } = await fetchMemory("w1");
    expect(source).toBe("gateway");
    expect(memory.facts).toHaveLength(1);
    const fact = memory.facts[0];
    expect(fact.factId).toBe("f1");
    expect(fact.createdAt).toBe("2026-05-30T10:00:00+00:00");
    // The whole point: `new Date(createdAt)` is a valid instant.
    expect(Number.isNaN(new Date(fact.createdAt).getTime())).toBe(false);
  });

  it("passes through camelCase fact fields unchanged", async () => {
    installMock(() =>
      makeResponse({
        namespaceId: "ns/x",
        facts: [
          { factId: "f2", text: "Repost Tuesdays", createdAt: "2026-05-29T09:00:00+00:00" },
        ],
        activeConversations: [],
        recentDecisions: [],
        voiceProfile: { tone: "neutral", formality: 50, signaturePhrases: [] },
      }),
    );

    const { memory } = await fetchMemory("w2");
    expect(memory.facts[0].factId).toBe("f2");
    expect(memory.facts[0].createdAt).toBe("2026-05-29T09:00:00+00:00");
  });
});
