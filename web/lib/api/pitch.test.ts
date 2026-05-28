import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { askPitch } from "./pitch";
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

function installMock(responder: (req: Request) => Response) {
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

describe("lib/api/pitch", () => {
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

  it("returns the gateway payload on a canonical hit", async () => {
    installMock(() =>
      makeResponse({
        status: 200,
        body: {
          matched: true,
          entry_id: "tech-stack",
          answer: "Rust single-binary core.",
          declared_claim_classes: ["architecture"],
          disposition: "Allow",
          block_reason: null,
          channel: "Rehearsal",
          latency_ms: 1,
        },
      }),
    );
    const res = await askPitch({ question: "What's the tech stack?" });
    expect(res.matched).toBe(true);
    expect(res.entry_id).toBe("tech-stack");
    expect(res.disposition).toBe("Allow");
    expect(res.channel).toBe("Rehearsal");
  });

  it("returns matched=false on a graceful miss", async () => {
    installMock(() =>
      makeResponse({
        status: 200,
        body: {
          matched: false,
          answer: "I'll get back to you on that one — let me check with the founder.",
          disposition: "Escalate",
          channel: "Rehearsal",
          latency_ms: 1,
        },
      }),
    );
    const res = await askPitch({ question: "Will you sell to military buyers?" });
    expect(res.matched).toBe(false);
    expect(res.disposition).toBe("Escalate");
    expect(res.answer).toContain("founder");
  });

  it("surfaces a Block disposition with a reason", async () => {
    installMock(() =>
      makeResponse({
        status: 200,
        body: {
          matched: true,
          entry_id: "valuation-question",
          answer: "Mark handles financial specifics directly.",
          declared_claim_classes: ["valuation"],
          disposition: "Block",
          block_reason: "forbidden_class: defer-to-mark-financial",
          channel: "WebEmbed",
          latency_ms: 1,
        },
      }),
    );
    const res = await askPitch({ question: "What's the valuation?", channel: "WebEmbed" });
    expect(res.disposition).toBe("Block");
    expect(res.block_reason).toContain("defer-to-mark-financial");
  });

  it("throws ApiError on a malformed payload", async () => {
    installMock(() =>
      makeResponse({
        status: 200,
        body: { answer: "missing-disposition-field" },
      }),
    );
    await expect(askPitch({ question: "x" })).rejects.toBeInstanceOf(ApiError);
  });

  it("propagates 5xx as ApiError (no swallow)", async () => {
    installMock(() =>
      makeResponse({
        status: 503,
        body: "gateway down",
        contentType: "text/plain",
      }),
    );
    await expect(askPitch({ question: "x" })).rejects.toBeInstanceOf(ApiError);
  });

  it("defaults the channel to Rehearsal when unspecified", async () => {
    let captured: unknown = null;
    installMock((req) => {
      void req
        .clone()
        .text()
        .then((body) => {
          captured = body;
        });
      return makeResponse({
        status: 200,
        body: {
          matched: false,
          answer: "fallback",
          disposition: "Escalate",
          channel: "Rehearsal",
          latency_ms: 0,
        },
      });
    });
    await askPitch({ question: "Hi" });
    await new Promise((r) => setTimeout(r, 0));
    expect(typeof captured).toBe("string");
    expect(captured as string).toContain('"channel":"Rehearsal"');
  });
});
