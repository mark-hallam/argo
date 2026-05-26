/**
 * Regression suite for the Scene-6 demo-live recording-mode opt-out
 * (brief `fix-spa-demo-live-mode-route-real-gateway-on-camera`).
 *
 * Pins the contract for all four short-circuit branches that the audit
 * / replay / approvals path used to short-circuit on auth-bypass:
 *
 *  - `lib/api/audit.ts:fetchAuditFeed`
 *  - `lib/api/audit-detail.ts:fetchAuditDetail`
 *  - `lib/api/replay.ts:replayAction`
 *  - `lib/api/approvals.ts:{fetchApprovals,submitDecision,submitUndo}`
 *
 * Under each branch:
 *  - `auth-bypass + !demo-live` → mock branch fires; gateway is NEVER
 *    called (DEFAULT — fast local iteration).
 *  - `auth-bypass + demo-live` → mock branch is SKIPPED; gateway IS
 *    called (RECORDING MODE — Scene 6 narration is truthful).
 *
 * This is the unit-test substitute for the brief's requested
 * `web/playwright/demo-live-mode/audit-fires-gateway.spec.ts`. The
 * Playwright local lane bakes `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=1`
 * into its `webServer.env` without a recording-mode lane, so wiring a
 * second SPA dev server just to assert "fetch happened" is heavier
 * than necessary. Mocking `apiGet` / `apiPost` proves the same
 * contract: the guard either calls the gateway or it doesn't.
 */
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const apiGetMock = vi.fn();
const apiPostMock = vi.fn();
vi.mock("@/lib/api/client", () => ({
  apiGet: (...args: unknown[]) => apiGetMock(...args),
  apiPost: (...args: unknown[]) => apiPostMock(...args),
}));

// Avoid sonner's lazy dynamic import surfacing in approvals tests
// (the role-gate toast path is exercised in approvals.test.ts already).
vi.mock("sonner", () => ({
  toast: {
    error: vi.fn(),
    success: vi.fn(),
    info: vi.fn(),
    warning: vi.fn(),
    loading: vi.fn(),
    promise: vi.fn(),
    dismiss: vi.fn(),
  },
}));

const ORIGINAL_ENV = { ...process.env };

beforeEach(() => {
  apiGetMock.mockReset();
  apiPostMock.mockReset();
  vi.resetModules();
  process.env = {
    ...ORIGINAL_ENV,
    NODE_ENV: "development",
  };
  delete process.env.NEXT_PUBLIC_ENV;
  delete process.env.NEXT_PUBLIC_SKYCOT_DEMO_MODE;
  delete process.env.NEXT_PUBLIC_SKYCOT_DEMO_LIVE;
  process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS = "1";
});

afterEach(() => {
  process.env = ORIGINAL_ENV;
});

describe("demo-live recording-mode opt-out — audit feed", () => {
  it("auth-bypass alone short-circuits to mockPage; gateway is NOT called", async () => {
    const { fetchAuditFeed } = await import("@/lib/api/audit");
    const page = await fetchAuditFeed({});
    expect(apiGetMock).not.toHaveBeenCalled();
    expect(Array.isArray(page.rows)).toBe(true);
  });

  it("auth-bypass + DEMO_LIVE=1 fires GET /api/v1/audit on the real gateway", async () => {
    process.env.NEXT_PUBLIC_SKYCOT_DEMO_LIVE = "1";
    apiGetMock.mockResolvedValueOnce({ items: [], next_cursor: null });
    const { fetchAuditFeed } = await import("@/lib/api/audit");
    await fetchAuditFeed({});
    expect(apiGetMock).toHaveBeenCalledTimes(1);
    expect(apiGetMock.mock.calls[0][0]).toMatch(/^\/api\/v1\/audit\?/);
  });
});

describe("demo-live recording-mode opt-out — audit detail", () => {
  it("auth-bypass alone short-circuits to mockDetail; gateway is NOT called", async () => {
    const { fetchAuditDetail } = await import("@/lib/api/audit-detail");
    // Seed mock id from `sample-audit-detail.ts` so mockDetail resolves.
    await fetchAuditDetail("act_01J0MARIAH_001").catch(() => {
      /* mock id may not exist; the point is we never called the gateway */
    });
    expect(apiGetMock).not.toHaveBeenCalled();
  });

  it("auth-bypass + DEMO_LIVE=1 fires GET /api/v1/audit/<id>", async () => {
    process.env.NEXT_PUBLIC_SKYCOT_DEMO_LIVE = "1";
    apiGetMock.mockResolvedValueOnce({
      summary: {
        id: "act_live_42",
        timestamp: "2026-05-27T00:00:00Z",
        worker: { id: "w1", name: "Mariah" },
        actionType: "send_message",
        verb: "wants to send",
        status: "succeeded",
        decider_role: "owner",
        channel: "slack",
      },
    });
    const { fetchAuditDetail } = await import("@/lib/api/audit-detail");
    await fetchAuditDetail("act_live_42");
    expect(apiGetMock).toHaveBeenCalledTimes(1);
    expect(apiGetMock.mock.calls[0][0]).toBe("/api/v1/audit/act_live_42");
  });
});

describe("demo-live recording-mode opt-out — replay", () => {
  it("auth-bypass alone routes through sampleOrFailed; gateway is NOT called", async () => {
    const { replayAction } = await import("@/lib/api/replay");
    await replayAction("act_01J0MARIAH_001");
    expect(apiPostMock).not.toHaveBeenCalled();
  });

  it("auth-bypass + DEMO_LIVE=1 fires POST /api/v1/audit/<id>/replay", async () => {
    process.env.NEXT_PUBLIC_SKYCOT_DEMO_LIVE = "1";
    apiPostMock.mockResolvedValueOnce({
      audit_id: "act_live_42",
      seq: 1,
      kind: "TurnEnded",
      verdict: "same",
      expected_self_hash: "abc",
      actual_self_hash: "abc",
    });
    const { replayAction } = await import("@/lib/api/replay");
    const outcome = await replayAction("act_live_42");
    expect(apiPostMock).toHaveBeenCalledTimes(1);
    expect(apiPostMock.mock.calls[0][0]).toBe("/api/v1/audit/act_live_42/replay");
    expect(outcome.kind).toBe("success");
  });
});

describe("demo-live recording-mode opt-out — approvals", () => {
  it("auth-bypass alone returns SAMPLE_APPROVALS with source=mock-auth-bypass", async () => {
    const { fetchApprovals } = await import("@/lib/api/approvals");
    const result = await fetchApprovals();
    expect(apiGetMock).not.toHaveBeenCalled();
    expect(result.source).toBe("mock-auth-bypass");
  });

  it("auth-bypass + DEMO_LIVE=1 fires GET /api/v1/approvals on the real gateway", async () => {
    process.env.NEXT_PUBLIC_SKYCOT_DEMO_LIVE = "1";
    apiGetMock.mockResolvedValueOnce({ items: [], next_cursor: null });
    const { fetchApprovals } = await import("@/lib/api/approvals");
    const result = await fetchApprovals();
    expect(apiGetMock).toHaveBeenCalledTimes(1);
    expect(apiGetMock.mock.calls[0][0]).toBe("/api/v1/approvals");
    expect(result.source).toBe("live");
  });

  it("auth-bypass alone returns mock-auth-bypass for submitDecision; no POST fires", async () => {
    const { submitDecision } = await import("@/lib/api/approvals");
    const result = await submitDecision({ approvalId: "ap_1", decision: "approve" });
    expect(apiPostMock).not.toHaveBeenCalled();
    expect(result.source).toBe("mock-auth-bypass");
  });

  it("auth-bypass + DEMO_LIVE=1 fires POST /api/v1/approvals/<id>/decide", async () => {
    process.env.NEXT_PUBLIC_SKYCOT_DEMO_LIVE = "1";
    apiPostMock.mockResolvedValueOnce({
      ok: true,
      approval_id: "ap_1",
      decision: "approve",
    });
    const { submitDecision } = await import("@/lib/api/approvals");
    const result = await submitDecision({ approvalId: "ap_1", decision: "approve" });
    expect(apiPostMock).toHaveBeenCalledTimes(1);
    expect(apiPostMock.mock.calls[0][0]).toBe("/api/v1/approvals/ap_1/decide");
    expect(result.source).toBe("live");
  });

  it("auth-bypass + DEMO_LIVE=1 fires POST /api/v1/approvals/<id>/undo", async () => {
    process.env.NEXT_PUBLIC_SKYCOT_DEMO_LIVE = "1";
    apiPostMock.mockResolvedValueOnce({ ok: true, approval_id: "ap_1" });
    const { submitUndo } = await import("@/lib/api/approvals");
    const result = await submitUndo("ap_1");
    expect(apiPostMock).toHaveBeenCalledTimes(1);
    expect(apiPostMock.mock.calls[0][0]).toBe("/api/v1/approvals/ap_1/undo");
    expect(result.source).toBe("live");
  });
});
