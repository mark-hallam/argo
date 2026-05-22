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

  it("posts the completed run to the gateway via persistDeps", async () => {
    // Stream `claude-followup-pr-2021-test-run-gateway-persist` —
    // optimistic-record-then-persist: the in-memory overlay completes
    // first, then the persist POST fires once per terminal phase.
    fetchSpy.mockResolvedValueOnce(
      new Response(
        streamFromLines([
          JSON.stringify({ type: "step:start", stepId: "s1" }),
          JSON.stringify({
            type: "step:end",
            stepId: "s1",
            ok: true,
            durationMs: 410,
          }),
          JSON.stringify({ type: "step:start", stepId: "s2" }),
          JSON.stringify({
            type: "step:end",
            stepId: "s2",
            ok: true,
            durationMs: 620,
          }),
          JSON.stringify({
            type: "run:summary",
            ok: 2,
            fail: 0,
            durationMs: 1030,
          }),
        ]),
        { status: 200 },
      ),
    );

    const persist = vi.fn().mockResolvedValue({ id: "row-1" });
    const { result } = renderHook(() =>
      useTestRun("support", "triage", STEPS, {
        persistDeps: { persist },
      }),
    );
    act(() => result.current.start());
    await waitFor(() => expect(result.current.phase).toBe("complete"));
    await waitFor(() => expect(persist).toHaveBeenCalledTimes(1));
    const arg = persist.mock.calls[0][0];
    expect(arg.verticalId).toBe("support");
    expect(arg.name).toBe("triage");
    expect(arg.run.status).toBe("complete");
    expect(arg.run.summary?.ok).toBe(2);
    expect(arg.run.stepStates.s1.status).toBe("success");
  });

  it("does not throw when persist rejects (best-effort)", async () => {
    // Stream `claude-followup-pr-2021-test-run-gateway-persist` —
    // persist failures must not break the in-memory overlay.
    fetchSpy.mockResolvedValueOnce(
      new Response(
        streamFromLines([
          JSON.stringify({ type: "step:start", stepId: "s1" }),
          JSON.stringify({
            type: "step:end",
            stepId: "s1",
            ok: true,
            durationMs: 100,
          }),
          JSON.stringify({
            type: "run:summary",
            ok: 1,
            fail: 0,
            durationMs: 100,
          }),
        ]),
        { status: 200 },
      ),
    );

    const persist = vi.fn().mockRejectedValue(new Error("network"));
    const warnSpy = vi.spyOn(console, "warn").mockImplementation(() => {});
    const { result } = renderHook(() =>
      useTestRun("support", "triage", STEPS, {
        persistDeps: { persist },
      }),
    );
    act(() => result.current.start());
    await waitFor(() => expect(result.current.phase).toBe("complete"));
    await waitFor(() => expect(persist).toHaveBeenCalledTimes(1));
    // The hook must remain in `complete`, not bubble the rejection.
    expect(result.current.phase).toBe("complete");
    warnSpy.mockRestore();
  });

  it("skips the persist call when persistOnComplete=false", async () => {
    fetchSpy.mockResolvedValueOnce(
      new Response(
        streamFromLines([
          JSON.stringify({ type: "step:start", stepId: "s1" }),
          JSON.stringify({
            type: "step:end",
            stepId: "s1",
            ok: true,
            durationMs: 50,
          }),
          JSON.stringify({
            type: "run:summary",
            ok: 1,
            fail: 0,
            durationMs: 50,
          }),
        ]),
        { status: 200 },
      ),
    );

    const persist = vi.fn().mockResolvedValue({});
    const { result } = renderHook(() =>
      useTestRun("support", "triage", STEPS, {
        persistDeps: { persist },
        persistOnComplete: false,
      }),
    );
    act(() => result.current.start());
    await waitFor(() => expect(result.current.phase).toBe("complete"));
    expect(persist).not.toHaveBeenCalled();
  });
});
