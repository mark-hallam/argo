import { act, renderHook } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import type { ReactNode } from "react";

// Mock the API module so the batch hook's underlying `submitDecision`
// POSTs are observable + don't touch `fetch`. `newIdempotencyKey` keeps
// its real implementation so we exercise the actual key generation.
// `vi.hoisted` is required because the `vi.mock` factory is hoisted above
// any top-level `const`.
const { submitDecisionMock } = vi.hoisted(() => ({ submitDecisionMock: vi.fn() }));
vi.mock("@/lib/api/approvals", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@/lib/api/approvals")>();
  return { ...actual, submitDecision: submitDecisionMock };
});

import { useBatchSubmitDecisions } from "./use-approvals";
import type { ApprovalDecisionRequest } from "@/lib/api/approvals";

function wrapper({ children }: { children: ReactNode }) {
  const qc = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  return <QueryClientProvider client={qc}>{children}</QueryClientProvider>;
}

afterEach(() => {
  submitDecisionMock.mockReset();
});

describe("useBatchSubmitDecisions — idempotency", () => {
  it("stamps a distinct idempotency key on every decision in the batch", async () => {
    submitDecisionMock.mockImplementation(async (req: ApprovalDecisionRequest) => ({
      ok: true,
      approvalId: req.approvalId,
      decision: req.decision,
      source: "live" as const,
    }));

    const { result } = renderHook(() => useBatchSubmitDecisions(), { wrapper });

    await act(async () => {
      await result.current.submit([
        { approvalId: "ap_1", decision: "approve" },
        { approvalId: "ap_2", decision: "approve" },
        { approvalId: "ap_3", decision: "approve" },
      ]);
    });

    expect(submitDecisionMock).toHaveBeenCalledTimes(3);
    const keys = submitDecisionMock.mock.calls.map(
      ([req]) => (req as ApprovalDecisionRequest).idempotencyKey,
    );
    // Every decision carries a key, and they're all distinct.
    expect(keys.every((k) => typeof k === "string" && k.length > 0)).toBe(true);
    expect(new Set(keys).size).toBe(3);
  });

  it("preserves a caller-supplied idempotency key instead of overwriting it", async () => {
    submitDecisionMock.mockImplementation(async (req: ApprovalDecisionRequest) => ({
      ok: true,
      approvalId: req.approvalId,
      decision: req.decision,
      source: "live" as const,
    }));

    const { result } = renderHook(() => useBatchSubmitDecisions(), { wrapper });
    await act(async () => {
      await result.current.submit([
        { approvalId: "ap_keep", decision: "approve", idempotencyKey: "preset" },
      ]);
    });

    expect(submitDecisionMock).toHaveBeenCalledTimes(1);
    const [req] = submitDecisionMock.mock.calls[0];
    expect((req as ApprovalDecisionRequest).idempotencyKey).toBe("preset");
  });

  it("fires exactly one submit per approvalId (no duplicate within a batch)", async () => {
    submitDecisionMock.mockImplementation(async (req: ApprovalDecisionRequest) => ({
      ok: true,
      approvalId: req.approvalId,
      decision: req.decision,
      source: "live" as const,
    }));

    const { result } = renderHook(() => useBatchSubmitDecisions(), { wrapper });
    await act(async () => {
      await result.current.submit([
        { approvalId: "ap_a", decision: "approve" },
        { approvalId: "ap_b", decision: "approve" },
      ]);
    });

    const ids = submitDecisionMock.mock.calls.map(
      ([req]) => (req as ApprovalDecisionRequest).approvalId,
    );
    expect(ids.sort()).toEqual(["ap_a", "ap_b"]);
  });
});
