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

  // FIX 4: conversation / decision items used to be cast straight to types
  // declaring required `href: string`, but a fresh worker omits per-item
  // `href`. The mappers now default every field — `href` is derived from the
  // item id (or "" when absent), never `undefined`.
  it("normalizes conversation/decision items missing per-item href (no undefined)", async () => {
    installMock(() =>
      makeResponse({
        namespace_id: "ns/x",
        facts: [],
        active_conversations: [
          // Snake-case id, NO href — the case that produced the lying type.
          {
            conversation_id: "conv-abc-123",
            counterparty_handle: "@buyer",
            topic: "refund",
            last_message_at: "2026-05-30T10:00:00+00:00",
          },
          // No id and no href at all → href must be "" (not undefined).
          { topic: "no id here" },
        ],
        recent_decisions: [
          {
            decision_id: "dec-xyz-9",
            label: "Approved refund",
            outcome: "approved",
            decided_at: "2026-05-30T11:00:00+00:00",
          },
          // Unknown outcome string → narrowed to "refused"; no id → href "".
          { label: "weird", outcome: "escalated" },
        ],
        voice_profile: { tone: "warm", formality: "professional" },
      }),
    );

    const { memory, source } = await fetchMemory("w3");
    expect(source).toBe("gateway");

    expect(memory.activeConversations).toHaveLength(2);
    const [c0, c1] = memory.activeConversations;
    // Derived audit drill-in from the id (dashes stripped), never undefined.
    expect(c0.conversationId).toBe("conv-abc-123");
    expect(c0.href).toBe("/audit/action/convabc123");
    expect(typeof c0.href).toBe("string");
    // No id → empty-string href, and the other required fields default too.
    expect(c1.href).toBe("");
    expect(c1.conversationId).toBe("");
    expect(c1.counterpartyHandle).toBe("");

    expect(memory.recentDecisions).toHaveLength(2);
    const [d0, d1] = memory.recentDecisions;
    expect(d0.decisionId).toBe("dec-xyz-9");
    expect(d0.href).toBe("/audit/action/decxyz9");
    expect(d0.outcome).toBe("approved");
    // Unknown outcome narrows to "refused"; missing id → "" href.
    expect(d1.outcome).toBe("refused");
    expect(d1.href).toBe("");

    // The whole point: not a single undefined href across either list.
    for (const c of memory.activeConversations) {
      expect(c.href).not.toBeUndefined();
    }
    for (const d of memory.recentDecisions) {
      expect(d.href).not.toBeUndefined();
    }
  });
});
