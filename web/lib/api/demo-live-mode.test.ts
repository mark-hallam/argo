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
import type { SkillGraph } from "@/lib/types/skill-ir";

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

const TEST_GRAPH: SkillGraph = {
  entry: "n1",
  nodes: [
    {
      id: "n1",
      label: "Draft response",
      kind: { type: "action", instruction: "Draft the response." },
      annotations: {},
    },
  ],
  edges: [],
  variables: [],
  metadata: { name: "Demo-live skill" },
};

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

  it("carries the gateway hash_chain_anchor through provenance normalisation", async () => {
    // A meta-event row (e.g. a "Run now" worker.run) has no
    // model/skill/policy provenance but does carry a real chain anchor
    // from its self_hash. The normaliser must preserve it so the SPA's
    // ProvenanceBlock renders the "Chain anchor" row instead of the
    // "Provenance unavailable" empty state.
    process.env.NEXT_PUBLIC_SKYCOT_DEMO_LIVE = "1";
    apiGetMock.mockResolvedValueOnce({
      summary: {
        id: "act_run_1",
        timestamp: "2026-05-30T00:00:00Z",
        worker: { id: "w-run", name: "Mariah" },
        action_type: "worker.run",
        verb: "ran now",
        status: "succeeded",
        channel: "web",
      },
      decision_chain: [
        { n: 1, title: "Manual trigger", body: "Run triggered by human:mark.", details: {} },
      ],
      provenance: {
        model: { id: "", version: "" },
        prompt_template: { id: "", hash: "" },
        skill: { id: "", version: "" },
        memory_snapshot: { id: "" },
        policy: { id: "", version: "", hash: "" },
        hash_chain_anchor: "deadbeefcafef00d",
      },
    });
    const { fetchAuditDetail } = await import("@/lib/api/audit-detail");
    const detail = await fetchAuditDetail("act_run_1");
    expect(detail.provenance.hash_chain_anchor).toBe("deadbeefcafef00d");
    // The pre-composed manual-trigger chain survives too.
    expect(detail.decisionChain[0]?.title).toBe("Manual trigger");
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

describe("demo-live recording-mode opt-out — elevated view", () => {
  const body = {
    surface: "audit",
    categories: ["unredacted-email" as const],
    reason: "Investigating the exact audit evidence for a customer escalation.",
    duration: "1h" as const,
  };

  it("auth-bypass alone issues a local grant; gateway is NOT called", async () => {
    const { submitElevatedViewRequest } = await import("@/lib/api/elevated-view");
    const result = await submitElevatedViewRequest(body);
    expect(apiPostMock).not.toHaveBeenCalled();
    expect(result).toEqual({ ok: true, status: "auto" });
  });

  it("auth-bypass + DEMO_LIVE=1 fires POST /api/v1/audit/elevated-view-request", async () => {
    process.env.NEXT_PUBLIC_SKYCOT_DEMO_LIVE = "1";
    apiPostMock.mockResolvedValueOnce({ ok: true, requestId: "ev_live_1", status: "auto" });
    const { submitElevatedViewRequest } = await import("@/lib/api/elevated-view");
    const result = await submitElevatedViewRequest(body);
    expect(apiPostMock).toHaveBeenCalledTimes(1);
    expect(apiPostMock.mock.calls[0][0]).toBe("/api/v1/audit/elevated-view-request");
    expect(apiPostMock.mock.calls[0][1]).toEqual(body);
    expect(result.requestId).toBe("ev_live_1");
  });

  it("auth-bypass + DEMO_LIVE=1 propagates gateway 404 instead of granting locally", async () => {
    process.env.NEXT_PUBLIC_SKYCOT_DEMO_LIVE = "1";
    const { ApiError } = await import("@/lib/api/types");
    apiPostMock.mockRejectedValueOnce(new ApiError(404, "not found"));
    const { submitElevatedViewRequest } = await import("@/lib/api/elevated-view");
    await expect(submitElevatedViewRequest(body)).rejects.toMatchObject({ status: 404 });
  });
});

describe("demo-live recording-mode opt-out — ghostwriter", () => {
  const prompt = "Draft a skill that triages inbound customer escalations for owners.";

  it("auth-bypass alone returns a mock draft; gateway is NOT called", async () => {
    const { draftSkillFromPrompt } = await import("@/lib/api/ghostwriter");
    const result = await draftSkillFromPrompt(prompt);
    expect(apiPostMock).not.toHaveBeenCalled();
    expect(result.source).toBe("mock-bypass");
  });

  it("auth-bypass + DEMO_LIVE=1 fires POST /api/v1/skills/draft-from-prompt", async () => {
    process.env.NEXT_PUBLIC_SKYCOT_DEMO_LIVE = "1";
    apiPostMock.mockResolvedValueOnce({
      skillId: "skill_live_ghostwriter",
      version: "1.0.0",
      irHash: "sha256:live",
      graph: TEST_GRAPH,
    });
    const { draftSkillFromPrompt } = await import("@/lib/api/ghostwriter");
    const result = await draftSkillFromPrompt(prompt);
    expect(apiPostMock).toHaveBeenCalledTimes(1);
    expect(apiPostMock.mock.calls[0][0]).toBe("/api/v1/skills/draft-from-prompt");
    expect(apiPostMock.mock.calls[0][1]).toEqual({ prompt });
    expect(result.source).toBe("gateway");
  });

  it("auth-bypass + DEMO_LIVE=1 propagates gateway 404 instead of returning mock IR", async () => {
    process.env.NEXT_PUBLIC_SKYCOT_DEMO_LIVE = "1";
    const { ApiError } = await import("@/lib/api/types");
    apiPostMock.mockRejectedValueOnce(new ApiError(404, "not found"));
    const { draftSkillFromPrompt } = await import("@/lib/api/ghostwriter");
    await expect(draftSkillFromPrompt(prompt)).rejects.toMatchObject({ status: 404 });
  });
});

describe("demo-live recording-mode opt-out — Skill IR", () => {
  it("auth-bypass alone returns bundled Skill IR; gateway is NOT called", async () => {
    const { fetchSkillIr } = await import("@/lib/api/skill-ir");
    const result = await fetchSkillIr("skill_screen_applications");
    expect(apiGetMock).not.toHaveBeenCalled();
    expect(result.source).toBe("mock-bypass");
  });

  it("auth-bypass + DEMO_LIVE=1 fires GET /api/v1/skills/<id>/ir", async () => {
    process.env.NEXT_PUBLIC_SKYCOT_DEMO_LIVE = "1";
    apiGetMock.mockResolvedValueOnce({
      graph: TEST_GRAPH,
      version: "v1",
      irHash: "sha256:live-ir",
    });
    const { fetchSkillIr } = await import("@/lib/api/skill-ir");
    const result = await fetchSkillIr("skill_live", "v1");
    expect(apiGetMock).toHaveBeenCalledTimes(1);
    expect(apiGetMock.mock.calls[0][0]).toBe("/api/v1/skills/skill_live/ir?version=v1");
    expect(result.source).toBe("gateway");
  });

  it("auth-bypass + DEMO_LIVE=1 maps gateway 404 to the honest missing-IR state", async () => {
    process.env.NEXT_PUBLIC_SKYCOT_DEMO_LIVE = "1";
    const { ApiError } = await import("@/lib/api/types");
    apiGetMock.mockRejectedValueOnce(new ApiError(404, "not found"));
    const { fetchSkillIr, SkillIrNotFoundError } = await import("@/lib/api/skill-ir");
    await expect(fetchSkillIr("skill_missing")).rejects.toBeInstanceOf(SkillIrNotFoundError);
  });
});

describe("demo-live recording-mode opt-out — Skill IR save/export/import", () => {
  it("auth-bypass + DEMO_LIVE=1 saves Skill IR through the gateway", async () => {
    process.env.NEXT_PUBLIC_SKYCOT_DEMO_LIVE = "1";
    apiPostMock.mockResolvedValueOnce({
      skillId: "skill_live",
      version: "v2",
      irHash: "sha256:saved",
      graph: TEST_GRAPH,
      abTestScheduled: false,
      abTestRunId: null,
    });
    const { saveSkillIr } = await import("@/lib/api/skill-ir-save");
    const result = await saveSkillIr("skill_live", TEST_GRAPH);
    expect(apiPostMock).toHaveBeenCalledTimes(1);
    expect(apiPostMock.mock.calls[0][0]).toBe("/api/v1/skills/skill_live/ir");
    expect(apiPostMock.mock.calls[0][1]).toEqual({ graph: TEST_GRAPH });
    expect(result.version).toBe("v2");
  });

  it("auth-bypass + DEMO_LIVE=1 exports Skill IR through the gateway", async () => {
    process.env.NEXT_PUBLIC_SKYCOT_DEMO_LIVE = "1";
    const bundle = {
      schemaVersion: "skycot.skill_ir.bundle.v1",
      sourceSkillId: "skill_live",
      version: "v2",
      irHash: "sha256:saved",
      graph: TEST_GRAPH,
    };
    apiGetMock.mockResolvedValueOnce(bundle);
    const { exportSkillIr } = await import("@/lib/api/skill-ir-export");
    const result = await exportSkillIr("skill_live", "v2", TEST_GRAPH, "sha256:local");
    expect(apiGetMock).toHaveBeenCalledTimes(1);
    expect(apiGetMock.mock.calls[0][0]).toBe("/api/v1/skills/skill_live/ir/export?version=v2");
    expect(result).toEqual(bundle);
  });

  it("auth-bypass + DEMO_LIVE=1 imports Skill IR through the gateway", async () => {
    process.env.NEXT_PUBLIC_SKYCOT_DEMO_LIVE = "1";
    const bundle = {
      schemaVersion: "skycot.skill_ir.bundle.v1" as const,
      sourceSkillId: "skill_live",
      version: "v2",
      irHash: "sha256:saved",
      graph: TEST_GRAPH,
    };
    apiPostMock.mockResolvedValueOnce({
      skillId: "skill_target",
      version: "v3",
      irHash: "sha256:imported",
      importedHash: "sha256:saved",
      graph: TEST_GRAPH,
      abTestScheduled: false,
      abTestRunId: null,
    });
    const { importSkillIr } = await import("@/lib/api/skill-ir-export");
    const result = await importSkillIr("skill_target", bundle);
    expect(apiPostMock).toHaveBeenCalledTimes(1);
    expect(apiPostMock.mock.calls[0][0]).toBe("/api/v1/skills/skill_target/ir/import");
    expect(apiPostMock.mock.calls[0][1]).toEqual({ bundle });
    expect(result.skillId).toBe("skill_target");
  });
});
