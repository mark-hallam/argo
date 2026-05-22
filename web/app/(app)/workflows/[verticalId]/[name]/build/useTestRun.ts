"use client";

import { useCallback, useEffect, useRef, useState } from "react";

export interface CompletedTestRun {
  startedAt: number;
  endedAt: number;
  status: "complete" | "failed";
  transportError: string | null;
  steps: StepInput[];
  stepStates: Record<string, StepRunState>;
  summary: RunSummary | null;
}

/**
 * Optional dependency injection for the server-side persist
 * call. Defaults are wired to `web/lib/api/test-runs.ts` so production
 * callers don't have to thread anything through; tests pass a stub to
 * assert the POST fires without touching `fetch`.
 */
export interface TestRunPersistDeps {
  persist?: (args: {
    verticalId: string;
    name: string;
    run: CompletedTestRun;
  }) => Promise<unknown>;
}

/**
 * Drives the builder's "Test run" overlay.
 *
 * Opens an NDJSON stream against
 * `POST /api/workflows/<vertical>/<name>/test-run` (which proxies the
 * gateway or simulates if the gateway is offline) and folds events
 * into per-step state + a run summary. The hook owns an
 * AbortController so unmount / re-trigger cancels the in-flight
 * fetch, preventing the leaked-stream class of bug called out in the
 * brief's acceptance criteria.
 */

export type StepRunStatus = "pending" | "running" | "success" | "error";

export interface StepRunState {
  status: StepRunStatus;
  durationMs?: number;
  tokens?: number;
  response?: string;
  error?: string;
}

export interface RunSummary {
  ok: number;
  fail: number;
  durationMs: number;
  tokens?: number;
  costUsd?: number;
}

export type RunPhase = "idle" | "running" | "complete" | "failed";

export interface StepInput {
  id: string;
  name: string;
  kind: string;
}

export interface UseTestRunResult {
  phase: RunPhase;
  stepStates: Record<string, StepRunState>;
  summary: RunSummary | null;
  transportError: string | null;
  start: (failStepId?: string | null) => void;
  reset: () => void;
}

interface RunEvent {
  type?: string;
  stepId?: string;
  ok?: boolean;
  durationMs?: number;
  tokens?: number;
  response?: string;
  error?: string;
  fail?: number;
  costUsd?: number;
}

function emptyStates(steps: StepInput[]): Record<string, StepRunState> {
  const out: Record<string, StepRunState> = {};
  for (const s of steps) out[s.id] = { status: "pending" };
  return out;
}

export interface UseTestRunOptions {
  /**
   * Fires once when a run transitions to a terminal phase
   * (`complete`, `failed`, or transport-error). The history panel
   * subscribes to this so it can record the run for replay after the
   * overlay closes. Callback is invoked with a snapshot of the inputs
   * + final step states + summary as of completion.
   */
  onComplete?: (run: CompletedTestRun) => void;
  /**
   * Server-side persist override (test seam — production callers
   * leave undefined and the hook routes through
   * `web/lib/api/test-runs.ts::persistTestRun`).
   */
  persistDeps?: TestRunPersistDeps;
  /**
   * Toggle the server-side persist POST. Defaults to `true` so the
   * builder records every completed run; pass `false` in fixtures
   * that should not touch the gateway.
   */
  persistOnComplete?: boolean;
}

export function useTestRun(
  verticalId: string,
  name: string,
  steps: StepInput[],
  options: UseTestRunOptions = {},
): UseTestRunResult {
  const [phase, setPhase] = useState<RunPhase>("idle");
  const [stepStates, setStepStates] = useState<Record<string, StepRunState>>(
    () => emptyStates(steps),
  );
  const [summary, setSummary] = useState<RunSummary | null>(null);
  const [transportError, setTransportError] = useState<string | null>(null);

  const abortRef = useRef<AbortController | null>(null);
  const mountedRef = useRef(true);
  // Keep onComplete + the latest snapshot in refs so the completion
  // emitter sees the freshest closure even when the parent re-renders
  // mid-stream. Without these refs the emit would capture stale values
  // and the history entry would lose its final step states.
  const onCompleteRef = useRef(options.onComplete);
  onCompleteRef.current = options.onComplete;
  const runStartRef = useRef<number | null>(null);
  const stepsRef = useRef<StepInput[]>(steps);
  stepsRef.current = steps;
  const stepStatesRef = useRef<Record<string, StepRunState>>(stepStates);
  stepStatesRef.current = stepStates;
  const summaryRef = useRef<RunSummary | null>(summary);
  summaryRef.current = summary;
  const transportErrorRef = useRef<string | null>(transportError);
  transportErrorRef.current = transportError;
  const completionEmittedRef = useRef(false);

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      abortRef.current?.abort();
      abortRef.current = null;
    };
  }, []);

  const reset = useCallback(() => {
    abortRef.current?.abort();
    abortRef.current = null;
    completionEmittedRef.current = false;
    runStartRef.current = null;
    setPhase("idle");
    setStepStates(emptyStates(steps));
    setSummary(null);
    setTransportError(null);
  }, [steps]);

  // Emit the on-completion event exactly once per run. Watching phase
  // via effect (rather than calling onComplete inline at each terminal
  // setPhase site) guarantees React has committed the latest summary
  // + stepStates before we snapshot them.
  useEffect(() => {
    if (phase !== "complete" && phase !== "failed") return;
    if (completionEmittedRef.current) return;
    completionEmittedRef.current = true;
    const snapshot: CompletedTestRun = {
      startedAt: runStartRef.current ?? Date.now(),
      endedAt: Date.now(),
      status: phase,
      transportError: transportErrorRef.current,
      steps: stepsRef.current,
      stepStates: stepStatesRef.current,
      summary: summaryRef.current,
    };
    const cb = onCompleteRef.current;
    if (cb) cb(snapshot);
    if (options.persistOnComplete !== false) {
      const persist =
        options.persistDeps?.persist ?? defaultPersistTestRun;
      void persist({ verticalId, name, run: snapshot }).catch((err) => {
        // Best-effort — persist failures must not break the in-memory
        // overlay. Swallow + console-log so the test seam can assert
        // the call fired without a noisy throw.
        if (typeof console !== "undefined") {
          // eslint-disable-next-line no-console
          console.warn("[useTestRun] persist failed", err);
        }
      });
    }
  }, [phase, verticalId, name, options.persistOnComplete, options.persistDeps]);

  const applyEvent = useCallback((evt: RunEvent) => {
    if (!mountedRef.current) return;
    if (evt.type === "step:start" && evt.stepId) {
      setStepStates((prev) => ({
        ...prev,
        [evt.stepId!]: { ...prev[evt.stepId!], status: "running" },
      }));
      return;
    }
    if (evt.type === "step:end" && evt.stepId) {
      setStepStates((prev) => ({
        ...prev,
        [evt.stepId!]: {
          status: evt.ok ? "success" : "error",
          durationMs: evt.durationMs,
          tokens: evt.tokens,
          response: evt.response,
          error: evt.error,
        },
      }));
      return;
    }
    if (evt.type === "run:summary") {
      const ok = typeof evt.ok === "number" ? (evt.ok as number) : 0;
      const fail = typeof evt.fail === "number" ? evt.fail : 0;
      setSummary({
        ok,
        fail,
        durationMs: evt.durationMs ?? 0,
        tokens: evt.tokens,
        costUsd: evt.costUsd,
      });
      setPhase(fail > 0 ? "failed" : "complete");
    }
  }, []);

  const start = useCallback(
    (failStepId: string | null = null) => {
      if (steps.length === 0) return;
      abortRef.current?.abort();
      const controller = new AbortController();
      abortRef.current = controller;
      completionEmittedRef.current = false;
      runStartRef.current = Date.now();
      setPhase("running");
      setStepStates(emptyStates(steps));
      setSummary(null);
      setTransportError(null);

      (async () => {
        try {
          const response = await fetch(
            `/api/workflows/${encodeURIComponent(verticalId)}/${encodeURIComponent(name)}/test-run`,
            {
              method: "POST",
              headers: { "Content-Type": "application/json" },
              body: JSON.stringify({ steps, failStepId }),
              signal: controller.signal,
            },
          );
          if (!response.ok || !response.body) {
            if (!mountedRef.current) return;
            setTransportError(
              `Test run failed to start (HTTP ${response.status}).`,
            );
            setPhase("failed");
            return;
          }
          const reader = response.body.getReader();
          const decoder = new TextDecoder();
          let buffer = "";
          while (true) {
            const { done, value } = await reader.read();
            if (done) break;
            buffer += decoder.decode(value, { stream: true });
            let newlineIdx = buffer.indexOf("\n");
            while (newlineIdx !== -1) {
              const line = buffer.slice(0, newlineIdx).trim();
              buffer = buffer.slice(newlineIdx + 1);
              if (line.length > 0) {
                try {
                  applyEvent(JSON.parse(line) as RunEvent);
                } catch {
                  // Ignore malformed line; stream continues.
                }
              }
              newlineIdx = buffer.indexOf("\n");
            }
          }
          if (buffer.trim().length > 0) {
            try {
              applyEvent(JSON.parse(buffer.trim()) as RunEvent);
            } catch {
              // Ignore trailing partial event.
            }
          }
          if (!mountedRef.current) return;
          setPhase((prev) => (prev === "running" ? "complete" : prev));
        } catch (err) {
          if ((err as { name?: string }).name === "AbortError") return;
          if (!mountedRef.current) return;
          setTransportError(
            err instanceof Error
              ? err.message
              : "Unknown error while streaming run events.",
          );
          setPhase("failed");
        }
      })();
    },
    [verticalId, name, steps, applyEvent],
  );

  return { phase, stepStates, summary, transportError, start, reset };
}

/**
 * Lazily-loaded production persist binding. Kept out of the
 * top-level import graph so unit tests that stub `options.persistDeps`
 * don't pay the import cost of `@/lib/api/client`.
 */
async function defaultPersistTestRun(args: {
  verticalId: string;
  name: string;
  run: CompletedTestRun;
}): Promise<unknown> {
  const mod = await import("@/lib/api/test-runs");
  return mod.persistTestRun(args);
}
