import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { fetchCohortDetail, fetchCohorts } from "./cohorts";
import { ApiError } from "./types";
import { SAMPLE_COHORT_SUMMARIES, SAMPLE_COHORT_DETAILS } from "@/lib/sample-cohorts";

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

describe("lib/api/cohorts", () => {
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

  it("fetchCohorts unwraps the envelope on 2xx happy path", async () => {
    const payload = { cohorts: SAMPLE_COHORT_SUMMARIES.slice(0, 1), total_count: 1 };
    installMock(() => makeResponse({ status: 200, body: payload }));
    const result = await fetchCohorts();
    expect(result.source).toBe("gateway");
    expect(result.cohorts).toEqual(payload.cohorts);
  });

  it("fetchCohorts falls back to SAMPLE on 404", async () => {
    installMock(() =>
      makeResponse({
        status: 404,
        body: { error: { code: "not_found", message: "no cohorts endpoint" } },
      }),
    );
    const result = await fetchCohorts();
    expect(result.source).toBe("mock-offline");
    expect(result.cohorts).toBe(SAMPLE_COHORT_SUMMARIES);
  });

  it("fetchCohorts propagates 5xx as ApiError (no mock swallow)", async () => {
    installMock(() =>
      makeResponse({ status: 503, body: "gateway down", contentType: "text/plain" }),
    );
    let caught: unknown;
    try {
      await fetchCohorts();
    } catch (err) {
      caught = err;
    }
    expect(caught).toBeInstanceOf(ApiError);
    expect((caught as ApiError).status).toBe(503);
  });

  it("fetchCohortDetail returns the mock when bypass flag is set", async () => {
    process.env.NEXT_PUBLIC_GATEWAY_AUTH_BYPASS = "1";
    const fetchMock = installMock(() => makeResponse({ status: 200, body: {} }));
    const result = await fetchCohortDetail("support::trusted");
    expect(result.source).toBe("mock-bypass");
    expect(result.detail).toBe(SAMPLE_COHORT_DETAILS["support::trusted"]);
    expect(fetchMock).not.toHaveBeenCalled();
  });
});
