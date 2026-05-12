import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  fetchApprovals,
  submitDecision,
  submitUndo,
} from "./approvals";
import { ApiError } from "./types";
import { SAMPLE_APPROVALS } from "@/lib/sample-approvals";

const ORIGINAL_FETCH = globalThis.fetch;
const ORIGINAL_BYPASS = process.env.NEXT_PUBLIC_GATEWAY_AUTH_BYPASS;

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

/**
 * Stub fetch — answers `/api/auth/session` so the client wrapper's
 * bearer-token lookup resolves to "no token," and delegates the
 * gateway call to the responder.
 */
function installMock(gatewayResponder: (req: Request) => Response | Promise<Response>) {
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

describe("lib/api/approvals", () => {
  beforeEach(() => {
    process.env.NEXT_PUBLIC_GATEWAY_URL = "http://gw.test";
    delete process.env.NEXT_PUBLIC_GATEWAY_AUTH_BYPASS;
  });
  afterEach(() => {
    globalThis.fetch = ORIGINAL_FETCH;
    if (ORIGINAL_BYPASS === undefined) {
      delete process.env.NEXT_PUBLIC_GATEWAY_AUTH_BYPASS;
    } else {
      process.env.NEXT_PUBLIC_GATEWAY_AUTH_BYPASS = ORIGINAL_BYPASS;
    }
    vi.restoreAllMocks();
  });

  it("maps snake_case gateway rows to camelCase ApprovalItem on happy path", async () => {
    installMock((req) => {
      if (req.url.endsWith("/api/v1/approvals")) {
        return makeResponse({
          status: 200,
          body: {
            items: [
              {
                id: "ap_live_1",
                worker: { id: "w1", name: "Mariah", avatar_url: null },
                verb: "wants to send a message",
                detail: "Hello world",
                consequence_if_yes: "msg sent",
                consequence_if_no: "msg held",
                reason: "policy",
                channel: "slack",
                received_at: "2026-05-12T10:00:00Z",
                decision: null,
              },
            ],
            next_cursor: null,
          },
        });
      }
      return makeResponse({ status: 500 });
    });

    const result = await fetchApprovals();
    expect(result.source).toBe("live");
    expect(result.items).toHaveLength(1);
    expect(result.items[0]).toMatchObject({
      id: "ap_live_1",
      worker: { name: "Mariah" },
      consequenceIfYes: "msg sent",
      consequenceIfNo: "msg held",
      receivedAt: "2026-05-12T10:00:00Z",
      channel: "slack",
      decision: null,
    });
  });

  it("falls back to mock dataset on 404 with source=mock-offline", async () => {
    installMock(() => makeResponse({ status: 404, body: "" }));

    const result = await fetchApprovals();
    expect(result.source).toBe("mock-offline");
    expect(result.items).toEqual(SAMPLE_APPROVALS);

    const decision = await submitDecision({
      approvalId: "ap_x",
      decision: "approve",
    });
    expect(decision).toEqual({
      ok: true,
      approvalId: "ap_x",
      decision: "approve",
      source: "mock-offline",
    });

    const undo = await submitUndo("ap_x");
    expect(undo).toEqual({ ok: true, approvalId: "ap_x", source: "mock-offline" });
  });

  it("propagates ApiError on 500 (no fallback)", async () => {
    installMock(() =>
      makeResponse({ status: 500, body: "boom", contentType: "text/plain" }),
    );

    await expect(fetchApprovals()).rejects.toBeInstanceOf(ApiError);
    await expect(
      submitDecision({ approvalId: "ap_y", decision: "refuse" }),
    ).rejects.toBeInstanceOf(ApiError);
    await expect(submitUndo("ap_y")).rejects.toBeInstanceOf(ApiError);
  });

  it("short-circuits to mock when NEXT_PUBLIC_GATEWAY_AUTH_BYPASS=1 (no fetch made)", async () => {
    process.env.NEXT_PUBLIC_GATEWAY_AUTH_BYPASS = "1";
    const fetchMock = installMock(() => {
      throw new Error("auth-bypass must not call fetch");
    });

    const list = await fetchApprovals();
    expect(list).toEqual({
      items: SAMPLE_APPROVALS,
      source: "mock-auth-bypass",
    });

    const decision = await submitDecision({
      approvalId: "ap_z",
      decision: "approve",
    });
    expect(decision).toEqual({
      ok: true,
      approvalId: "ap_z",
      decision: "approve",
      source: "mock-auth-bypass",
    });

    const undo = await submitUndo("ap_z");
    expect(undo).toEqual({
      ok: true,
      approvalId: "ap_z",
      source: "mock-auth-bypass",
    });

    expect(fetchMock).not.toHaveBeenCalled();
  });
});
