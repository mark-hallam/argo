import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { draftResponse } from "./marketing";
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

describe("lib/api/marketing", () => {
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

  it("posts a reply-mode request and returns the Allow draft", async () => {
    const fetchMock = installMock((req) => {
      expect(req.url).toContain("/api/v1/marketing/draft-response");
      return makeResponse({
        status: 200,
        body: {
          agent: "Marcus",
          responsibility: "marketing",
          mode: "reply",
          channel: "x",
          draft: "Autocomplete doesn't own a backlog. Teammates do. That's the gap.",
          disposition: "Allow",
          source: "llm",
          model: "claude-sonnet-4-6",
          approval_id: "01J-approval",
          tenant: "workspace",
          tokens: { input: 120, output: 40 },
        },
      });
    });
    const res = await draftResponse({
      mode: "reply",
      channel: "x",
      source: "AI agents are just fancy autocomplete.",
    });
    expect(res.agent).toBe("Marcus");
    expect(res.mode).toBe("reply");
    expect(res.channel).toBe("x");
    expect(res.disposition).toBe("Allow");
    expect(res.approval_id).toBe("01J-approval");

    // The request body must carry mode + channel + source, not topic.
    const sent = fetchMock.mock.calls.find(([url]) =>
      String(url).includes("/api/v1/marketing/draft-response"),
    );
    const body = JSON.parse(String((sent?.[1] as RequestInit)?.body));
    expect(body.request.mode).toBe("reply");
    expect(body.request.channel).toBe("x");
    expect(body.request.source).toBe("AI agents are just fancy autocomplete.");
    expect(body.request.topic).toBeUndefined();
  });

  it("posts an article-mode request with topic, not source", async () => {
    const fetchMock = installMock(() =>
      makeResponse({
        status: 200,
        body: {
          agent: "Marcus",
          responsibility: "marketing",
          mode: "article",
          channel: "linkedin",
          draft: "Ideas are the moat. — Mark + the team",
          disposition: "Allow",
          source: "llm",
        },
      }),
    );
    const res = await draftResponse({
      mode: "article",
      channel: "linkedin",
      topic: "ideas are the human moat",
    });
    expect(res.channel).toBe("linkedin");
    const sent = fetchMock.mock.calls.find(([url]) =>
      String(url).includes("/api/v1/marketing/draft-response"),
    );
    const body = JSON.parse(String((sent?.[1] as RequestInit)?.body));
    expect(body.request.topic).toBe("ideas are the human moat");
    expect(body.request.source).toBeUndefined();
  });

  it("surfaces an Escalate (held-for-review) draft", async () => {
    installMock(() =>
      makeResponse({
        status: 200,
        body: {
          agent: "Marcus",
          responsibility: "marketing",
          mode: "article",
          channel: "linkedin",
          draft: "[Held for founder review — this draft touched a guarded topic (revenue)…]",
          disposition: "Escalate",
          source: "llm",
        },
      }),
    );
    const res = await draftResponse({ mode: "article", channel: "linkedin", topic: "our numbers" });
    expect(res.disposition).toBe("Escalate");
    expect(res.draft).toContain("Held for founder review");
  });

  it("throws ApiError on a malformed payload", async () => {
    installMock(() => makeResponse({ status: 200, body: { agent: "Marcus" } }));
    await expect(
      draftResponse({ mode: "reply", channel: "x", source: "hi" }),
    ).rejects.toBeInstanceOf(ApiError);
  });

  it("throws ApiError on a 503 from the gateway", async () => {
    installMock(() =>
      makeResponse({ status: 503, body: { error: "marketing llm transport: down" } }),
    );
    await expect(
      draftResponse({ mode: "reply", channel: "x", source: "hi" }),
    ).rejects.toBeInstanceOf(ApiError);
  });
});
