import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { fetchWorker, WorkerNotFoundError } from "./workers";
import { SAMPLE_WORKERS_DETAIL } from "@/lib/sample-workers-detail";

const ORIGINAL_FETCH = globalThis.fetch;
const MARIAH = SAMPLE_WORKERS_DETAIL.find((worker) => worker.name === "Mariah")!;

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
    body === undefined
      ? null
      : typeof body === "string"
        ? body
        : JSON.stringify(body),
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

describe("lib/api/workers", () => {
  const originalEnv = { ...process.env };

  beforeEach(() => {
    process.env.NEXT_PUBLIC_GATEWAY_URL = "http://gw.test";
    vi.stubEnv("NODE_ENV", "test");
    delete process.env.NEXT_PUBLIC_ENV;
    delete process.env.ARGO_GATEWAY_AUTH_BYPASS;
  });

  afterEach(() => {
    globalThis.fetch = ORIGINAL_FETCH;
    process.env = { ...originalEnv };
    vi.unstubAllEnvs();
    vi.restoreAllMocks();
  });

  it("returns a bare worker response on the 2xx path", async () => {
    installMock(() => makeResponse({ body: MARIAH }));

    await expect(fetchWorker(MARIAH.id)).resolves.toEqual(MARIAH);
  });

  it("unwraps gateway { worker } envelopes", async () => {
    installMock(() => makeResponse({ body: { worker: MARIAH } }));

    await expect(fetchWorker(MARIAH.id)).resolves.toEqual(MARIAH);
  });

  it("falls back to bundled sample data for demo workers on dev 404", async () => {
    installMock((req) => {
      expect(req.url).toBe(`http://gw.test/api/v1/workers/${MARIAH.id}`);
      return makeResponse({
        status: 404,
        body: { error: { code: "not_found", message: "worker not found" } },
      });
    });

    await expect(fetchWorker(MARIAH.id)).resolves.toEqual(MARIAH);
  });

  it("throws WorkerNotFoundError for unknown ids after dev 404", async () => {
    installMock(() =>
      makeResponse({
        status: 404,
        body: { error: { code: "not_found", message: "worker not found" } },
      }),
    );

    await expect(
      fetchWorker("018f67d0-0000-7000-8000-000000009999"),
    ).rejects.toBeInstanceOf(WorkerNotFoundError);
  });
});
