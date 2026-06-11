"use client";

import { useEffect, useState } from "react";
import { Card, CardBody, CardHeader, CardTitle } from "@/components/ui/Card";
import {
  fetchWorkerRunOutput,
  type WorkerRunOutput,
} from "@/lib/api/worker-output";
import { loadLastRunSummary } from "@/components/skycot/worker/last-run-summary";

/**
 * "Output" card — shows what a teammate actually produced on a run
 * (brief `uf-spa-teammate-output-surface`).
 *
 * Before this surface the only trace of a run a SPA user saw was an
 * `/audit` chars-count. This card renders the teammate's real work
 * product — the assistant's reasoning/plan (`assistant_output.md`) and
 * the proposed-action summary (`actions.json`) — read by the gateway
 * from the run's artifact bundle, tenant-scoped, never from the SPA's
 * mock layer.
 *
 * NO THEATER: with no `jobId` (or when the run isn't the caller's), the
 * card renders an honest empty state — it never fabricates output. Pass
 * the `jobId` returned by the "Run now" response to deep-link a run.
 *
 * Remount survival: the route renders this card as a SIBLING of
 * `WorkerOverviewContent` (which owns the Run-now call), so the run's
 * `job_id` never reaches us as a prop, and the page remounts right
 * after a run. When no explicit `jobId` prop is given, we rehydrate the
 * latest run's `job_id` from the per-worker last-run summary
 * (`last-run-summary.ts`) on mount — otherwise the card shows "No runs
 * yet" immediately after a successful run.
 */
export function RunOutputCard({
  workerId,
  jobId,
}: {
  workerId: string;
  jobId?: string | null;
}) {
  // When the caller doesn't pass a jobId, rehydrate the last run's job
  // id from sessionStorage on mount. An explicit `jobId` prop always
  // wins (used by tests + any future direct deep-link).
  const [storedJobId, setStoredJobId] = useState<string | null>(null);
  useEffect(() => {
    if (jobId) return;
    setStoredJobId(loadLastRunSummary(workerId)?.jobId ?? null);
  }, [workerId, jobId]);
  const effectiveJobId = jobId ?? storedJobId;

  const [state, setState] = useState<"idle" | "loading" | "loaded" | "error">(
    jobId ? "loading" : "idle",
  );
  const [output, setOutput] = useState<WorkerRunOutput | null>(null);

  useEffect(() => {
    if (!effectiveJobId) {
      setState("idle");
      setOutput(null);
      return;
    }
    let alive = true;
    setState("loading");
    fetchWorkerRunOutput(workerId, effectiveJobId)
      .then((res) => {
        if (!alive) return;
        setOutput(res);
        setState("loaded");
      })
      .catch(() => {
        if (alive) setState("error");
      });
    return () => {
      alive = false;
    };
  }, [workerId, effectiveJobId]);

  return (
    <Card data-testid="card-run-output">
      <CardHeader>
        <CardTitle>Output</CardTitle>
        {output ? (
          <p className="text-sm text-[var(--muted-foreground)]">
            {runMeta(output)}
          </p>
        ) : null}
      </CardHeader>
      <CardBody className="pt-1">
        <RunOutputBody state={state} output={output} />
      </CardBody>
    </Card>
  );
}

/** Body branch — exported for direct unit testing of each state. */
export function RunOutputBody({
  state,
  output,
}: {
  state: "idle" | "loading" | "loaded" | "error";
  output: WorkerRunOutput | null;
}) {
  if (state === "loading") {
    return (
      <p className="text-sm text-[var(--muted-foreground)]">Loading output…</p>
    );
  }
  if (state === "error") {
    return (
      <p className="text-sm text-[var(--muted-foreground)]">
        Couldn&apos;t load this run&apos;s output.
      </p>
    );
  }
  if (!output) {
    return (
      <p
        data-testid="run-output-empty"
        className="text-sm text-[var(--muted-foreground)]"
      >
        No runs yet. Use <span className="font-medium">Run now</span> to see
        what your teammate produces here.
      </p>
    );
  }
  return (
    <div className="space-y-3">
      {output.actionCounts ? (
        <p
          data-testid="run-output-actions"
          className="text-sm text-[var(--foreground)]"
        >
          <span className="font-medium">
            {output.actionCounts.total ?? 0}
          </span>{" "}
          proposed action{(output.actionCounts.total ?? 0) === 1 ? "" : "s"}
          {output.actionCounts.pendingApproval
            ? ` · ${output.actionCounts.pendingApproval} awaiting approval`
            : ""}
        </p>
      ) : null}
      {output.assistantOutput ? (
        <pre
          data-testid="run-output-assistant"
          className="max-h-96 overflow-auto whitespace-pre-wrap rounded-[var(--radius-md)] border border-[var(--border)]/60 bg-[var(--muted)]/40 p-3 text-sm text-[var(--foreground)]"
        >
          {output.assistantOutput}
        </pre>
      ) : (
        <p className="text-sm text-[var(--muted-foreground)]">
          This run recorded no assistant output.
        </p>
      )}
    </div>
  );
}

function runMeta(output: WorkerRunOutput): string {
  const parts: string[] = [];
  if (typeof output.stepsCompleted === "number") {
    parts.push(`${output.stepsCompleted} step${output.stepsCompleted === 1 ? "" : "s"}`);
  }
  if (output.stopReason) {
    parts.push(output.stopReason.replace(/_/g, " "));
  }
  return parts.join(" · ");
}
