import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  fetchApprovals,
  newIdempotencyKey,
  submitDecision,
  submitUndo,
} from "./approvals";
import { ApiError } from "./types";
import { SAMPLE_APPROVALS } from "@/lib/sample-approvals";

// `toastForbidden` in `./approvals` calls `await import("sonner")`. Mocking
// the module keeps the lazy import resolvable in unit tests and lets us
// assert the role-denied toast surface fires with the right copy. The
// `vi.hoisted` indirection is required because `vi.mock` factories run
// before any top-level `const` initializers.
const { toastErrorMock } = vi.hoisted(() => ({ toastErrorMock: vi.fn() }));
vi.mock("sonner", () => ({
  toast: {
    error: toastErrorMock,
    success: vi.fn(),
    info: vi.fn(),
    warning: vi.fn(),
    loading: vi.fn(),
    promise: vi.fn(),
    dismiss: vi.fn(),
  },
}));

const ORIGINAL_FETCH = globalThis.fetch;
const ORIGINAL_BYPASS = process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS;

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
    delete process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS;
    toastErrorMock.mockClear();
  });
  afterEach(() => {
    globalThis.fetch = ORIGINAL_FETCH;
    if (ORIGINAL_BYPASS === undefined) {
      delete process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS;
    } else {
      process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS = ORIGINAL_BYPASS;
    }
    vi.unstubAllEnvs();
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

  it("returns empty inbox on 404 outside demo mode (real users see fresh-workspace state)", async () => {
    installMock(() => makeResponse({ status: 404, body: "" }));

    const result = await fetchApprovals();
    expect(result.source).toBe("live");
    expect(result.items).toEqual([]);
  });

  it("falls back to mock dataset on 404 when NEXT_PUBLIC_SKYCOT_DEMO_MODE=1", async () => {
    vi.stubEnv("NEXT_PUBLIC_SKYCOT_DEMO_MODE", "1");
    installMock(() => makeResponse({ status: 404, body: "" }));

    const result = await fetchApprovals();
    expect(result.source).toBe("mock-offline");
    expect(result.items).toEqual(SAMPLE_APPROVALS);

    // Mutation mock-offline fallback is unchanged — it is the audit-
    // trail launch-blocker guard that propagates 5xx in strict prod.
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

  // Pairs with mission-rbac-approvals-decide-role-gate (#1956): a
  // viewer or below who tries to decide gets a 403 from the gateway.
  // The client must (a) surface a plain-language "you don't have
  // permission" toast and (b) rethrow so the useSubmitDecision
  // `onError` handler can roll the optimistic mutation back — silently
  // resolving here would corrupt the approvals audit chain.
  it("surfaces a sonner toast and rethrows on 403 from /decide", async () => {
    installMock(() => makeResponse({ status: 403, body: { error: "forbidden" } }));

    await expect(
      submitDecision({ approvalId: "ap_403", decision: "approve" }),
    ).rejects.toBeInstanceOf(ApiError);

    // toastForbidden uses a fire-and-forget dynamic import; flush
    // pending microtasks so the `.then` chain runs before we assert.
    await new Promise((resolve) => setTimeout(resolve, 0));

    expect(toastErrorMock).toHaveBeenCalledTimes(1);
    const [title, opts] = toastErrorMock.mock.calls[0];
    expect(title).toMatch(/permission to approve/i);
    expect(opts).toMatchObject({ description: expect.stringMatching(/admin/i) });
  });

  it("surfaces a sonner toast and rethrows on 403 from /undo", async () => {
    installMock(() => makeResponse({ status: 403, body: { error: "forbidden" } }));

    await expect(submitUndo("ap_403_undo")).rejects.toBeInstanceOf(ApiError);

    await new Promise((resolve) => setTimeout(resolve, 0));

    expect(toastErrorMock).toHaveBeenCalledTimes(1);
    const [title] = toastErrorMock.mock.calls[0];
    expect(title).toMatch(/permission to undo/i);
  });

  // Day-1 unblock FIX 2 (NO-THEATER on the flagship loop): the gateway's
  // /decide envelope carries `execution: "dispatched" | "failed"` —
  // whether the approved action actually ran. submitDecision must pass
  // the two known values through, and degrade anything else (absent /
  // unrecognised) to undefined so the UI never renders a wrong claim.
  it("passes execution: 'dispatched' through from the /decide envelope", async () => {
    installMock(() =>
      makeResponse({
        status: 200,
        body: {
          ok: true,
          approval_id: "ap_exec",
          decision: "approve",
          execution: "dispatched",
        },
      }),
    );

    const result = await submitDecision({ approvalId: "ap_exec", decision: "approve" });
    expect(result.execution).toBe("dispatched");
    expect(result.source).toBe("live");
  });

  it("passes execution: 'failed' through from the /decide envelope", async () => {
    installMock(() =>
      makeResponse({
        status: 200,
        body: {
          ok: true,
          approval_id: "ap_exec",
          decision: "approve",
          execution: "failed",
        },
      }),
    );

    const result = await submitDecision({ approvalId: "ap_exec", decision: "approve" });
    expect(result.execution).toBe("failed");
  });

  it("omits execution when the envelope lacks it or carries an unknown value", async () => {
    installMock(() =>
      makeResponse({
        status: 200,
        body: { ok: true, approval_id: "ap_old", decision: "approve" },
      }),
    );
    const absent = await submitDecision({ approvalId: "ap_old", decision: "approve" });
    expect(absent.execution).toBeUndefined();

    installMock(() =>
      makeResponse({
        status: 200,
        body: {
          ok: true,
          approval_id: "ap_new",
          decision: "approve",
          execution: "queued-v2-shape",
        },
      }),
    );
    const unknown = await submitDecision({ approvalId: "ap_new", decision: "approve" });
    expect(unknown.execution).toBeUndefined();
  });

  it("short-circuits to mock when NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=1 (no fetch made)", async () => {
    process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS = "1";
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

  // Recording-mode opt-out (NO THEATER / DEC-007): with auth-bypass AND
  // demo-live both set, the inbox + decide + undo must hit the real
  // gateway so the approvals the camera sees are seeded server rows, not
  // the bundled SAMPLE_APPROVALS constant. Mirrors the canonical
  // `isAuthBypassed() && !isDemoLive()` guard in `lib/api/skills.ts`.
  it("hits the real gateway in demo-live even with auth-bypass set", async () => {
    process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS = "1";
    vi.stubEnv("NEXT_PUBLIC_SKYCOT_DEMO_LIVE", "1");
    const fetchMock = installMock((req) => {
      if (req.url.endsWith("/api/v1/approvals")) {
        return makeResponse({
          status: 200,
          body: {
            items: [
              {
                id: "ap_seeded_1",
                worker: { id: "w1", name: "Mariah", avatar_url: null },
                verb: "wants to send a message",
                detail: "Seeded row",
                consequence_if_yes: "msg sent",
                consequence_if_no: "msg held",
                channel: "slack",
                received_at: "2026-05-12T10:00:00Z",
                decision: null,
              },
            ],
            next_cursor: null,
          },
        });
      }
      if (req.url.endsWith("/decide")) {
        return makeResponse({
          status: 200,
          body: { ok: true, approval_id: "ap_seeded_1", decision: "approve" },
        });
      }
      if (req.url.endsWith("/undo")) {
        return makeResponse({
          status: 200,
          body: { ok: true, approval_id: "ap_seeded_1" },
        });
      }
      return makeResponse({ status: 500 });
    });

    const list = await fetchApprovals();
    expect(list.source).toBe("live");
    expect(list.items).toHaveLength(1);
    expect(list.items[0]).toMatchObject({ id: "ap_seeded_1", worker: { name: "Mariah" } });

    const decision = await submitDecision({
      approvalId: "ap_seeded_1",
      decision: "approve",
    });
    expect(decision.source).toBe("live");

    const undo = await submitUndo("ap_seeded_1");
    expect(undo.source).toBe("live");

    const hitGateway = (suffix: string) =>
      fetchMock.mock.calls.some((call) => {
        const input = call[0];
        const url =
          typeof input === "string"
            ? input
            : input instanceof URL
              ? input.toString()
              : (input as Request).url;
        return url.endsWith(suffix);
      });
    expect(hitGateway("/api/v1/approvals")).toBe(true);
    expect(hitGateway("/api/v1/approvals/ap_seeded_1/decide")).toBe(true);
    expect(hitGateway("/api/v1/approvals/ap_seeded_1/undo")).toBe(true);
  });

  // Batch "Approve all" idempotency: threading a per-attempt key lets the
  // gateway dedupe a double-submitted decision so the audit chain records
  // it exactly once.
  it("forwards idempotency_key in the /decide body when supplied", async () => {
    let capturedBody: Record<string, unknown> | null = null;
    installMock(async (req) => {
      if (req.url.endsWith("/decide")) {
        capturedBody = req.body ? JSON.parse(await req.text()) : null;
        return makeResponse({
          status: 200,
          body: { ok: true, approval_id: "ap_idem", decision: "approve" },
        });
      }
      return makeResponse({ status: 500 });
    });

    await submitDecision({
      approvalId: "ap_idem",
      decision: "approve",
      idempotencyKey: "idem-key-123",
    });
    expect(capturedBody).toEqual({
      decision: "approve",
      idempotency_key: "idem-key-123",
    });
  });

  it("omits idempotency_key from the /decide body on the single-card path (none supplied)", async () => {
    let capturedBody: Record<string, unknown> | null = null;
    installMock(async (req) => {
      if (req.url.endsWith("/decide")) {
        capturedBody = req.body ? JSON.parse(await req.text()) : null;
        return makeResponse({
          status: 200,
          body: { ok: true, approval_id: "ap_solo", decision: "refuse" },
        });
      }
      return makeResponse({ status: 500 });
    });

    await submitDecision({ approvalId: "ap_solo", decision: "refuse" });
    expect(capturedBody).toEqual({ decision: "refuse" });
    expect(capturedBody).not.toHaveProperty("idempotency_key");
  });
});

describe("newIdempotencyKey", () => {
  it("returns a non-empty, unique-per-call string", () => {
    const a = newIdempotencyKey();
    const b = newIdempotencyKey();
    expect(typeof a).toBe("string");
    expect(a.length).toBeGreaterThan(0);
    expect(a).not.toBe(b);
  });
});
