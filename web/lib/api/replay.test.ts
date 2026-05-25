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
});
