import { describe, expect, it, beforeEach, afterEach, vi } from "vitest";
import { act, renderHook, waitFor } from "@testing-library/react";
import { useTestRun, type StepInput } from "./useTestRun";

const STEPS: StepInput[] = [
  { id: "s1", name: "Trigger", kind: "trigger" },
  { id: "s2", name: "Classify", kind: "skill_invoke" },
];

function streamFromLines(lines: string[]): ReadableStream<Uint8Array> {
  const encoder = new TextEncoder();
  return new ReadableStream<Uint8Array>({
    start(controller) {
      for (const line of lines) {
        controller.enqueue(encoder.encode(line + "\n"));
      }
      controller.close();
    },
  });
}

describe("useTestRun", () => {
  const fetchSpy = vi.spyOn(globalThis, "fetch");

  beforeEach(() => {
    fetchSpy.mockReset();
  });

  afterEach(() => {
    fetchSpy.mockReset();
  });

  it("folds NDJSON step events into per-step state and a final summary", async () => {
    fetchSpy.mockResolvedValueOnce(
      new Response(
        streamFromLines([
          JSON.stringify({ type: "step:start", stepId: "s1" }),
          JSON.stringify({
            type: "step:end",
            stepId: "s1",
            ok: true,
            durationMs: 410,
            response: "Trigger fired",
          }),
          JSON.stringify({ type: "step:start", stepId: "s2" }),
          JSON.stringify({
            type: "step:end",
            stepId: "s2",
            ok: true,
            durationMs: 620,
            tokens: 280,
            response: "Skill returned",
          }),
          JSON.stringify({
            type: "run:summary",
            ok: 2,
            fail: 0,
            durationMs: 1030,
            tokens: 280,
            costUsd: 0.0014,
          }),
        ]),
        { status: 200 },
      ),
    );

    const { result } = renderHook(() => useTestRun("support", "triage", STEPS));
    expect(result.current.phase).toBe("idle");

    act(() => {
      result.current.start();
    });

    await waitFor(() => expect(result.current.phase).toBe("complete"));
    expect(result.current.stepStates.s1.status).toBe("success");
    expect(result.current.stepStates.s2.tokens).toBe(280);
    expect(result.current.summary?.ok).toBe(2);
    expect(result.current.summary?.costUsd).toBe(0.0014);
  });

  it("marks a step as error when the gateway emits a failed step:end event", async () => {
    fetchSpy.mockResolvedValueOnce(
      new Response(
        streamFromLines([
          JSON.stringify({ type: "step:start", stepId: "s1" }),
          JSON.stringify({
            type: "step:end",
            stepId: "s1",
            ok: false,
            durationMs: 200,
            error: "boom",
          }),
          JSON.stringify({
            type: "run:summary",
            ok: 0,
            fail: 1,
            durationMs: 200,
          }),
        ]),
        { status: 200 },
      ),
    );

    const { result } = renderHook(() => useTestRun("support", "triage", STEPS));
    act(() => result.current.start("s1"));
    await waitFor(() => expect(result.current.phase).toBe("failed"));
    expect(result.current.stepStates.s1.status).toBe("error");
    expect(result.current.stepStates.s1.error).toBe("boom");
  });

  it("aborts the in-flight stream on unmount (no leaked reader)", async () => {
    let abortedSignal: AbortSignal | undefined;
    fetchSpy.mockImplementationOnce((_url, init) => {
      abortedSignal = init?.signal ?? undefined;
      return new Promise<Response>(() => {
        // Pending forever; relies on AbortSignal to settle.
      });
    });

    const { result, unmount } = renderHook(() =>
      useTestRun("support", "triage", STEPS),
    );
    act(() => result.current.start());
    expect(abortedSignal?.aborted).toBe(false);
    unmount();
    expect(abortedSignal?.aborted).toBe(true);
  });

  it("surfaces a transport error when the HTTP response is non-OK", async () => {
    fetchSpy.mockResolvedValueOnce(
      new Response("nope", { status: 502 }),
    );

    const { result } = renderHook(() => useTestRun("support", "triage", STEPS));
    act(() => result.current.start());
    await waitFor(() => expect(result.current.phase).toBe("failed"));
    expect(result.current.transportError).toContain("502");
  });
});
