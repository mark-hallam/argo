import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { classifyRole } from "./genesis";
import { ApiError } from "./types";
import { STUB_QUESTIONS } from "@/lib/hire/genesis-stub";

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

function installMock(
  gatewayResponder: (req: Request) => Response | Promise<Response>,
) {
  const fetchMock = vi.fn((input: RequestInfo | URL, init?: RequestInit) => {
    const url =
      typeof input === "string"
        ? input
        : input instanceof URL
          ? input.toString()
          : input.url;
    if (url.endsWith("/api/auth/session")) {
      return Promise.resolve(makeResponse({ status: 200, body: {} }));
    }
    const req = new Request(url, init);
    return Promise.resolve(gatewayResponder(req));
  });
  globalThis.fetch = fetchMock as unknown as typeof fetch;
  return fetchMock;
}

const VALID_GATEWAY_RESPONSE = {
  classification: "marketplace-seller",
  questions: [
    {
      id: "q1",
      text: "How many messages do you get per day?",
      answer_type: "single-select",
      options: ["0–10", "10–50", "50+"],
    },
    {
      id: "q2",
      text: "What's the typical response you'd send?",
      answer_type: "free-text",
    },
  ],
};

describe("lib/api/genesis", () => {
  beforeEach(() => {
    process.env.NEXT_PUBLIC_GATEWAY_URL = "http://gw.test";
  });
  afterEach(() => {
    globalThis.fetch = ORIGINAL_FETCH;
    vi.restoreAllMocks();
  });

  it("returns gateway questions on 2xx happy path", async () => {
    installMock((req) => {
      if (req.url.endsWith("/api/v1/genesis/classify")) {
        return makeResponse({ status: 200, body: VALID_GATEWAY_RESPONSE });
      }
      return makeResponse({ status: 500 });
    });

    const result = await classifyRole({ problem: "I sell stuff on FB marketplace" });
    expect(result.source).toBe("gateway");
    expect(result.classification).toBe("marketplace-seller");
    expect(result.questions).toHaveLength(2);
    expect(result.questions[0]?.id).toBe("q1");
  });

  it("falls back to STUB_QUESTIONS on 404 (gateway not wired yet)", async () => {
    installMock(() => makeResponse({ status: 404, body: "" }));

    const result = await classifyRole({ problem: "anything" });
    expect(result.source).toBe("stub");
    expect(result.questions).toEqual([...STUB_QUESTIONS]);
  });

  it("re-throws ApiError on 5xx so caller can retry", async () => {
    installMock(() =>
      makeResponse({ status: 503, body: "boom", contentType: "text/plain" }),
    );
    await expect(classifyRole({ problem: "x" })).rejects.toBeInstanceOf(ApiError);
  });

  it("falls back to STUB_QUESTIONS on malformed response", async () => {
    installMock(() =>
      makeResponse({
        status: 200,
        body: { questions: [{ id: "bad", text: "missing answer_type" }] },
      }),
    );

    const result = await classifyRole({ problem: "x" });
    expect(result.source).toBe("stub");
    expect(result.questions).toEqual([...STUB_QUESTIONS]);
  });

  it("re-throws on network error (TypeError from fetch) so caller can retry", async () => {
    const fetchMock = vi.fn((input: RequestInfo | URL) => {
      const url = typeof input === "string" ? input : (input as Request | URL).toString();
      if (url.endsWith("/api/auth/session")) {
        return Promise.resolve(makeResponse({ status: 200, body: {} }));
      }
      return Promise.reject(new TypeError("network unreachable"));
    });
    globalThis.fetch = fetchMock as unknown as typeof fetch;

    await expect(classifyRole({ problem: "x" })).rejects.toThrow(TypeError);
  });

  it("rejects single-select questions with missing options as malformed", async () => {
    installMock(() =>
      makeResponse({
        status: 200,
        body: {
          questions: [
            { id: "q1", text: "Pick one", answer_type: "single-select" },
          ],
        },
      }),
    );

    const result = await classifyRole({ problem: "x" });
    expect(result.source).toBe("stub");
  });

  it("rejects responses with too many questions (>3) as malformed", async () => {
    installMock(() =>
      makeResponse({
        status: 200,
        body: {
          questions: [
            { id: "q1", text: "a", answer_type: "yes-no" },
            { id: "q2", text: "b", answer_type: "yes-no" },
            { id: "q3", text: "c", answer_type: "yes-no" },
            { id: "q4", text: "d", answer_type: "yes-no" },
          ],
        },
      }),
    );
    const result = await classifyRole({ problem: "x" });
    expect(result.source).toBe("stub");
  });
});
