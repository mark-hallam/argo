/**
 * Regression suite for the Scene-6 demo-gate fix: the replay client
 * must NOT fabricate a forensic outcome for action ids the gateway
 * 404s on. Mock outcomes are gated on (demo OR auth-bypass) AND a
 * canonical TABLE entry — every other path returns a real `failed`
 * envelope so the page renders `<FailedView>`.
 */
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const apiPostMock = vi.fn();
vi.mock("@/lib/api/client", () => ({
  apiPost: (...args: unknown[]) => apiPostMock(...args),
}));

const ORIGINAL_ENV = { ...process.env };

beforeEach(() => {
  apiPostMock.mockReset();
  process.env = {
    ...ORIGINAL_ENV,
    NODE_ENV: "development",
  };
  delete process.env.NEXT_PUBLIC_ENV;
  delete process.env.NEXT_PUBLIC_SKYCOT_DEMO_MODE;
  delete process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS;
});

afterEach(() => {
  process.env = ORIGINAL_ENV;
});

describe("replayAction — Scene-6 fabrication gate", () => {
  it("returns a real `failed` envelope for an unknown 32-char-hex id when the gateway 404s", async () => {
    const { replayAction } = await import("@/lib/api/replay");
    const { ApiError } = await import("@/lib/api/types");
    apiPostMock.mockRejectedValueOnce(new ApiError(404, "Not found"));

    const outcome = await replayAction("01234567890abcdef0123456789abcde");

    expect(outcome.kind).toBe("failed");
    if (outcome.kind === "failed") {
      expect(outcome.reason).toMatch(/seeded demo personas/i);
    }
  });

  it("does NOT synthesise a `drifted-misaligned` outcome from the URL", async () => {
    // The pre-fix code hashed actionId.charCodeAt(last) % 3 — char 'd'
    // (code 100) % 3 = 1 (drifted-aligned); char 'e' (101) % 3 = 2
    // (drifted-misaligned). Verify neither path is reachable for an
    // unknown id in any non-strict-prod env.
    const { replayAction } = await import("@/lib/api/replay");
    const { ApiError } = await import("@/lib/api/types");
    apiPostMock.mockRejectedValue(new ApiError(404, "Not found"));

    for (const id of [
      "01234567890abcdef0123456789abcde",
      "ffffffffffffffffffffffffffffffff",
      "act_test_obviously_fake_999",
    ]) {
      const outcome = await replayAction(id);
      expect(outcome.kind).toBe("failed");
    }
  });

  it("returns the canonical sample for a TABLE id in demo mode (gateway 404)", async () => {
    process.env.NEXT_PUBLIC_SKYCOT_DEMO_MODE = "1";
    const { replayAction } = await import("@/lib/api/replay");
    const { ApiError } = await import("@/lib/api/types");
    apiPostMock.mockRejectedValueOnce(new ApiError(404, "Not found"));

    const outcome = await replayAction("act_01J0PRIYA_001");

    expect(outcome.kind).toBe("success");
    if (outcome.kind === "success") {
      expect(outcome.verdict).toBe("reproducible");
    }
  });

  it("returns the canonical sample for a TABLE id under auth-bypass without calling the gateway", async () => {
    process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS = "1";
    const { replayAction } = await import("@/lib/api/replay");

    const outcome = await replayAction("act_01J0MARIAH_001");

    expect(apiPostMock).not.toHaveBeenCalled();
    expect(outcome.kind).toBe("success");
    if (outcome.kind === "success") {
      expect(outcome.verdict).toBe("drifted-misaligned");
    }
  });

  it("returns `failed` for an unknown id under auth-bypass (no gateway call)", async () => {
    process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS = "1";
    const { replayAction } = await import("@/lib/api/replay");

    const outcome = await replayAction("01234567890abcdef0123456789abcde");

    expect(apiPostMock).not.toHaveBeenCalled();
    expect(outcome.kind).toBe("failed");
  });

  it("re-throws non-404 errors in non-strict-prod (5xx is not a 'missing' signal)", async () => {
    const { replayAction } = await import("@/lib/api/replay");
    const { ApiError } = await import("@/lib/api/types");
    apiPostMock.mockRejectedValueOnce(new ApiError(503, "Service unavailable"));

    await expect(
      replayAction("act_01J0PRIYA_001"),
    ).rejects.toBeInstanceOf(ApiError);
  });

  it("returns the gateway envelope on success (no mock interposition)", async () => {
    const { replayAction } = await import("@/lib/api/replay");
    const live = {
      kind: "success" as const,
      verdict: "reproducible" as const,
      result: {
        originalOutput: { kind: "text" as const, body: "live original" },
        replayedOutput: { kind: "text" as const, body: "live original" },
        provenanceDiff: {
          model: { changed: false },
          promptTemplate: { changed: false },
          skill: { changed: false },
          memorySnapshot: { changed: false },
          policy: { changed: false },
        },
      },
    };
    apiPostMock.mockResolvedValueOnce(live);

    const outcome = await replayAction("01234567890abcdef0123456789abcde");

    expect(outcome).toEqual(live);
  });

  it("adapts a gateway hash `same` verdict to a reproducible success", async () => {
    const { replayAction } = await import("@/lib/api/replay");
    apiPostMock.mockResolvedValueOnce({
      audit_id: "audit_123",
      seq: 1840,
      kind: "http.request",
      verdict: "same",
      expected_self_hash: "hash_same",
      actual_self_hash: "hash_same",
      diff: {},
    });

    const outcome = await replayAction("audit_123");

    expect(outcome.kind).toBe("success");
    if (outcome.kind === "success") {
      expect(outcome.verdict).toBe("reproducible");
      expect(outcome.result.originalOutput.body).toContain("hash_same");
      expect(outcome.result.replayedOutput.body).toContain("hash_same");
      expect(outcome.result.provenanceDiff.model.changed).toBe(false);
      expect(outcome.result.provenanceDiff.policy.changed).toBe(false);
    }
  });

  it("adapts a gateway hash `drifted` verdict to a drifted success with diff details", async () => {
    const { replayAction } = await import("@/lib/api/replay");
    apiPostMock.mockResolvedValueOnce({
      audit_id: "audit_456",
      seq: 1841,
      kind: "http.request",
      verdict: "drifted",
      expected_self_hash: "hash_before",
      actual_self_hash: "hash_after",
      diff: {
        skill: { from: "skill v1", to: "skill v2" },
      },
    });

    const outcome = await replayAction("audit_456");

    expect(outcome.kind).toBe("success");
    if (outcome.kind === "success") {
      expect(outcome.verdict).toBe("drifted-aligned");
      expect(outcome.result.originalOutput.body).toContain("hash_before");
      expect(outcome.result.replayedOutput.body).toContain("hash_after");
      expect(outcome.result.provenanceDiff.skill).toEqual({
        changed: true,
        from: "skill v1",
        to: "skill v2",
      });
    }
  });

  it("marks policy drift from the gateway hash envelope as drifted-misaligned", async () => {
    const { replayAction } = await import("@/lib/api/replay");
    apiPostMock.mockResolvedValueOnce({
      verdict: "drifted",
      expected_self_hash: "hash_before",
      actual_self_hash: "hash_after",
      diff: {
        policy: { from: "policy v1", to: "policy v2" },
      },
    });

    const outcome = await replayAction("audit_789");

    expect(outcome.kind).toBe("success");
    if (outcome.kind === "success") {
      expect(outcome.verdict).toBe("drifted-misaligned");
      expect(outcome.result.provenanceDiff.policy.changed).toBe(true);
    }
  });

  it("maps unknown gateway verdicts to unavailable instead of leak-detected", async () => {
    const { replayAction } = await import("@/lib/api/replay");
    apiPostMock.mockResolvedValueOnce({
      verdict: "mystery",
      expected_self_hash: "hash_before",
      actual_self_hash: "hash_after",
      diff: {},
    });

    const outcome = await replayAction("audit_unknown");

    expect(outcome.kind).toBe("unavailable");
    if (outcome.kind === "unavailable" && outcome.reason === "unrecognized-envelope") {
      expect(outcome.reason).toBe("unrecognized-envelope");
      expect(outcome.details).toMatch(/unknown verdict/i);
    }
  });

  it("maps unrecognized replay envelopes to unavailable instead of leak-detected", async () => {
    const { replayAction } = await import("@/lib/api/replay");
    apiPostMock.mockResolvedValueOnce({ ok: true });

    const outcome = await replayAction("audit_bad_shape");

    expect(outcome.kind).toBe("unavailable");
    if (outcome.kind === "unavailable" && outcome.reason === "unrecognized-envelope") {
      expect(outcome.reason).toBe("unrecognized-envelope");
      expect(outcome.details).toMatch(/unrecognized envelope/i);
    }
  });
});
