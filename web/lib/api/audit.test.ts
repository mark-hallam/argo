import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { fetchAuditFeed } from "./audit";
import type { AuditFeedPage, AuditRow } from "@/lib/types/audit";

const ORIGINAL_FETCH = globalThis.fetch;
const ORIGINAL_GATEWAY = process.env.NEXT_PUBLIC_GATEWAY_URL;
const ORIGINAL_BYPASS = process.env.ARGO_GATEWAY_AUTH_BYPASS;

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
      typeof input === "string"
        ? input
        : input instanceof URL
          ? input.toString()
          : input.url;
    if (url.endsWith("/api/auth/session")) {
      return Promise.resolve(makeResponse({ status: 200, body: {} }));
    }
    const req = new Request(url, init);
    return Promise.resolve(responder(req));
  });
  globalThis.fetch = fetchMock as unknown as typeof fetch;
  return fetchMock;
}

function makeAuditRow(overrides: Partial<AuditRow> = {}): AuditRow {
  return {
    id: "audit_1",
    timestamp: "2026-05-24T07:30:00Z",
    worker: { id: "wkr-live-1", name: "Compliance Worker" },
    actionType: "hipaa_compliance_check",
    verb: "ran HIPAA compliance check",
    status: "succeeded",
    channel: "web",
    dataClassification: "restricted",
    ...overrides,
  };
}

describe("lib/api/audit", () => {
  beforeEach(() => {
    process.env.NEXT_PUBLIC_GATEWAY_URL = "http://gw.test";
    vi.stubEnv("NODE_ENV", "test");
    delete process.env.NEXT_PUBLIC_ENV;
    delete process.env.ARGO_GATEWAY_AUTH_BYPASS;
  });

  afterEach(() => {
    globalThis.fetch = ORIGINAL_FETCH;
    if (ORIGINAL_GATEWAY === undefined) delete process.env.NEXT_PUBLIC_GATEWAY_URL;
    else process.env.NEXT_PUBLIC_GATEWAY_URL = ORIGINAL_GATEWAY;
    if (ORIGINAL_BYPASS === undefined) delete process.env.ARGO_GATEWAY_AUTH_BYPASS;
    else process.env.ARGO_GATEWAY_AUTH_BYPASS = ORIGINAL_BYPASS;
    vi.unstubAllEnvs();
    vi.restoreAllMocks();
  });

  it("unwraps the gateway {items, next_cursor} envelope and maps snake_case row fields", async () => {
    installMock((req) => {
      expect(req.url).toBe("http://gw.test/api/v1/audit?range=today");
      return makeResponse({
        status: 200,
        body: {
          items: [
            {
              id: "audit_live_1",
              timestamp: "2026-05-24T07:30:00Z",
              worker: { id: "wkr-live-1", name: "Compliance Worker" },
              action_type: "hipaa_compliance_check",
              verb: "ran HIPAA compliance check",
              status: "succeeded",
              approver: null,
              channel: "web",
              cost: null,
              data_classification: "restricted",
              decider_role: "admin",
            },
          ],
          next_cursor: "cursor-page-2",
        },
      });
    });

    const result = await fetchAuditFeed({ range: "today" });

    expect(result.nextCursor).toBe("cursor-page-2");
    expect(result.totalPages).toBeUndefined();
    expect(result.rows).toHaveLength(1);
    expect(result.rows[0]).toMatchObject({
      id: "audit_live_1",
      actionType: "hipaa_compliance_check",
      dataClassification: "restricted",
      decider_role: "admin",
    });
    expect(result.rows[0].approver).toBeUndefined();
    expect(result.rows[0].cost).toBeUndefined();
  });

  it("keeps the older AuditFeedPage shape working for tests and legacy gateways", async () => {
    const legacyPage: AuditFeedPage = {
      rows: [makeAuditRow({ id: "audit_legacy_1", actionType: "memory_written" })],
      nextCursor: null,
      totalPages: 2,
    };
    installMock(() => makeResponse({ status: 200, body: legacyPage }));

    const result = await fetchAuditFeed({ range: "today" });

    expect(result).toEqual(legacyPage);
  });
});
