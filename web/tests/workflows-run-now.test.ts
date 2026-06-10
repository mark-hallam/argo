import { beforeEach, describe, expect, it, vi } from "vitest";

/**
 * Unit pins for the seed publish-then-run bridge backing the workflow
 * detail page's "Run now" action on seed-source workflows.
 *
 * Brief: rebase-fix-workflow-detail-run-now-disabled-when-source-not-gateway
 * (re-implements closed PR #2358 against current main).
 *
 * The detail page enables Run-now even when `source !== "gateway"`. For a
 * seed-source workflow the gateway has no row yet, so `publishSeedWorkflowToHead`
 * serializes the seed steps to `workflow.toml`, saves a draft, and promotes
 * it to head before the page executes it — keeping the run honest (NO
 * THEATER, DEC-007/DEC-029) rather than dispatching a mock.
 */

const mocks = vi.hoisted(() => ({
  saveDraft: vi.fn(),
  publishHead: vi.fn(),
  encodeStepsToToml: vi.fn(),
}));

vi.mock("@/lib/api/workflows-save-publish", () => ({
  saveDraft: mocks.saveDraft,
  publishHead: mocks.publishHead,
}));

vi.mock("@/app/(app)/workflows/[verticalId]/[name]/build/step-encoder", () => ({
  encodeStepsToToml: mocks.encodeStepsToToml,
}));

import { publishSeedWorkflowToHead } from "@/lib/api/workflows";

const SEED = {
  description: "Verify refund eligibility, check ledger, route high-value to manager.",
  steps: [
    { id: "step-1", kind: "trigger" as const, name: "refund_requested", description: "" },
    { id: "step-2", kind: "approval" as const, name: "manager_signoff", description: "" },
  ],
};

const ENCODED_TOML = '[workflow]\nname = "approve_refund"\n';

describe("publishSeedWorkflowToHead", () => {
  beforeEach(() => {
    mocks.saveDraft.mockReset();
    mocks.publishHead.mockReset();
    mocks.encodeStepsToToml.mockReset();
    mocks.encodeStepsToToml.mockReturnValue(ENCODED_TOML);
  });

  it("encodes the seed, saves a draft, then promotes it to head", async () => {
    mocks.saveDraft.mockResolvedValue({ kind: "ok", version_id: "ver-uuid", version: 1 });
    mocks.publishHead.mockResolvedValue({ head_version: 4 });

    const head = await publishSeedWorkflowToHead("support", "approve_refund", SEED);

    expect(mocks.encodeStepsToToml).toHaveBeenCalledWith(SEED.steps, {
      workflowName: "approve_refund",
      workflowDescription: SEED.description,
    });
    expect(mocks.saveDraft).toHaveBeenCalledWith({
      vertical_id: "support",
      name: "approve_refund",
      definition_toml: ENCODED_TOML,
    });
    expect(mocks.publishHead).toHaveBeenCalledWith({
      vertical_id: "support",
      name: "approve_refund",
      version_id: "ver-uuid",
    });
    expect(head).toBe(4);
  });

  it("skips the promote and runs the live head when the gateway reports a conflict", async () => {
    mocks.saveDraft.mockResolvedValue({
      kind: "conflict",
      current_version: 7,
      current_head_definition_toml: "[workflow]\nname = \"approve_refund\"\n",
    });

    const head = await publishSeedWorkflowToHead("support", "approve_refund", SEED);

    expect(mocks.publishHead).not.toHaveBeenCalled();
    expect(head).toBe(7);
  });

  it("propagates a save failure so the caller's run-now error layer surfaces it", async () => {
    mocks.saveDraft.mockRejectedValue(new Error("gateway 503"));

    await expect(
      publishSeedWorkflowToHead("support", "approve_refund", SEED),
    ).rejects.toThrow("gateway 503");
    expect(mocks.publishHead).not.toHaveBeenCalled();
  });
});
