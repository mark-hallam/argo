import { act, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { BuilderAiProposal } from "./BuilderAiProposal";
import type { WorkflowStep } from "./types";

function streamingResponse(chunks: string[]): Response {
  const encoder = new TextEncoder();
  let i = 0;
  const stream = new ReadableStream<Uint8Array>({
    pull(controller) {
      if (i >= chunks.length) {
        controller.close();
        return;
      }
      controller.enqueue(encoder.encode(chunks[i]!));
      i += 1;
    },
  });
  return new Response(stream, { status: 200, headers: { "Content-Type": "text/plain" } });
}

const CURRENT: WorkflowStep[] = [
  { id: "old_step", kind: "skill_invoke", name: "old_step", description: "" },
];

const PROPOSAL_TOML = `[workflow]
name = "demo_flow"

[[step]]
name = "describe_intent"
kind = "prompt"

[[step]]
name = "operator_review"
kind = "approval"
`;

describe("BuilderAiProposal", () => {
  it("streams TOML and surfaces added steps in the diff", async () => {
    const fetchImpl = vi.fn().mockResolvedValue(streamingResponse([PROPOSAL_TOML]));
    const onApprove = vi.fn();
    const onCancel = vi.fn();

    render(
      <BuilderAiProposal
        prompt="add an approval gate"
        verticalId="ops"
        workerId="ops-weekly"
        currentSteps={CURRENT}
        onApprove={onApprove}
        onCancel={onCancel}
        fetchImpl={fetchImpl as unknown as typeof fetch}
      />,
    );

    await waitFor(() => {
      const diff = screen.getByTestId("builder-ai-proposal-diff");
      expect(diff.textContent).toContain("describe_intent");
      expect(diff.textContent).toContain("operator_review");
      expect(diff.textContent).toContain("old_step");
    });

    expect(fetchImpl).toHaveBeenCalledWith(
      "/api/workflows/propose",
      expect.objectContaining({ method: "POST" }),
    );
  });

  it("renders a retry CTA on non-OK response and re-runs the fetch", async () => {
    const fetchImpl = vi
      .fn()
      .mockResolvedValueOnce(new Response("upstream is down", { status: 502 }))
      .mockResolvedValueOnce(streamingResponse([PROPOSAL_TOML]));

    render(
      <BuilderAiProposal
        prompt="convert to autonomous"
        verticalId="ops"
        workerId="ops-weekly"
        currentSteps={CURRENT}
        onApprove={vi.fn()}
        onCancel={vi.fn()}
        fetchImpl={fetchImpl as unknown as typeof fetch}
      />,
    );

    const retry = await screen.findByTestId("builder-ai-proposal-retry");
    await act(async () => {
      retry.click();
    });

    await waitFor(() => {
      expect(screen.queryByTestId("builder-ai-proposal-error")).toBeNull();
    });
    expect(fetchImpl).toHaveBeenCalledTimes(2);
  });

  it("approve fires the parent callback with parsed steps", async () => {
    const fetchImpl = vi.fn().mockResolvedValue(streamingResponse([PROPOSAL_TOML]));
    const onApprove = vi.fn();

    render(
      <BuilderAiProposal
        prompt="add a notify step"
        verticalId="ops"
        workerId="ops-weekly"
        currentSteps={CURRENT}
        onApprove={onApprove}
        onCancel={vi.fn()}
        fetchImpl={fetchImpl as unknown as typeof fetch}
      />,
    );

    const approve = await screen.findByTestId("builder-ai-proposal-approve");
    await waitFor(() => {
      expect(approve).not.toBeDisabled();
    });
    await act(async () => {
      approve.click();
    });
    expect(onApprove).toHaveBeenCalledTimes(1);
    const passed = onApprove.mock.calls[0][0] as WorkflowStep[];
    expect(passed.map((s) => s.id)).toEqual(["describe_intent", "operator_review"]);
  });
});
