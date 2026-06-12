"use client";

import { useEffect, useState } from "react";
import { Card, CardBody, CardHeader, CardTitle } from "@/components/ui/Card";
import {
  axisTone,
  fetchWorkerEvalReplay,
  toAxisRows,
  type EvalAxisRow,
  type WorkerEvalReplay,
} from "@/lib/api/worker-evals-replay";
import { hasRealSession, isDemoLive } from "@/lib/skycot/demo-mode";
import { cn } from "@/lib/utils";

/**
 * Eval-replay "Performance" card for one worker.
 *
 * Renders the 6-axis teammate-eval scorecard the gateway produces at
 * `GET /api/v1/workers/<id>/evals/replay` — the merged headline pass
 * rate, each axis mean as a bar against the pass threshold, and the
 * weakest axis ("points the next improvement"). It also surfaces how
 * many of the scores came from REAL replayed production interactions
 * vs the hand-authored synthetic golden set.
 *
 * NO THEATER: the route grades the teammate only on rows that exist, so
 * a worker with no production interactions AND no synthetic scenarios
 * (`merged.total === 0`) renders an honest empty state — never a
 * fabricated score. The card is mock-gated like the other worker
 * surfaces: it only fetches against a real session (or the demo-live
 * gateway), so a fixture/anonymous session never round-trips the route.
 *
 * Fetch shape mirrors `RunOutputCard` (useState/useEffect, no TanStack
 * Query) so the body is unit-testable without a QueryClientProvider.
 */
export function EvalScorecard({ workerId }: { workerId: string }) {
  const enabled = hasRealSession() || isDemoLive();
  const [state, setState] = useState<"idle" | "loading" | "loaded" | "error">(
    enabled ? "loading" : "idle",
  );
  const [data, setData] = useState<WorkerEvalReplay | null>(null);

  useEffect(() => {
    if (!enabled) {
      setState("idle");
      setData(null);
      return;
    }
    let alive = true;
    setState("loading");
    fetchWorkerEvalReplay(workerId)
      .then((res) => {
        if (!alive) return;
        setData(res);
        setState("loaded");
      })
      .catch(() => {
        if (alive) setState("error");
      });
    return () => {
      alive = false;
    };
  }, [workerId, enabled]);

  return (
    <Card data-testid="card-eval-scorecard">
      <CardHeader>
        <CardTitle>Performance evals</CardTitle>
        {state === "loaded" && data ? (
          <p className="text-sm text-[var(--muted-foreground)]">
            {data.merged.passed}/{data.merged.total} scenarios passed
            <span className="opacity-60">
              {" "}
              · {Math.round(data.merged.passRate * 100)}% pass rate
            </span>
          </p>
        ) : null}
      </CardHeader>
      <CardBody className="pt-1">
        <EvalScorecardBody state={state} data={data} />
      </CardBody>
    </Card>
  );
}

/**
 * Presentational body. Split out (mirrors `RunOutputBody`) so unit
 * tests can mount the loaded / empty / error states directly without a
 * fetch round-trip.
 */
export function EvalScorecardBody({
  state,
  data,
}: {
  state: "idle" | "loading" | "loaded" | "error";
  data: WorkerEvalReplay | null;
}) {
  if (state === "idle") {
    return (
      <p
        data-testid="eval-scorecard-idle"
        className="text-sm text-[var(--muted-foreground)]"
      >
        Performance evals appear once this teammate is running in a live
        workspace.
      </p>
    );
  }
  if (state === "error") {
    return (
      <p
        data-testid="eval-scorecard-error"
        className="text-sm text-[var(--muted-foreground)]"
      >
        Couldn&apos;t load the eval scorecard.
      </p>
    );
  }
  if (state === "loading" || !data) {
    return (
      <p
        data-testid="eval-scorecard-loading"
        className="text-sm text-[var(--muted-foreground)]"
      >
        Scoring recent work…
      </p>
    );
  }

  const rows = toAxisRows(data.merged);
  if (data.merged.total === 0 || rows.length === 0) {
    return (
      <p
        data-testid="eval-scorecard-empty"
        className="text-sm text-[var(--muted-foreground)]"
      >
        No eval runs yet for this teammate. Scores appear as they handle real
        work.
      </p>
    );
  }

  return (
    <div data-testid="eval-scorecard-axes" className="space-y-3">
      <ul role="list" className="space-y-3">
        {rows.map((row) => (
          <AxisBar key={row.axis} row={row} threshold={data.merged.threshold} />
        ))}
      </ul>
      <EvalScorecardFooter data={data} />
    </div>
  );
}

const TONE_BAR: Record<"success" | "warning", string> = {
  success: "bg-emerald-500",
  warning: "bg-amber-500",
};

function AxisBar({ row, threshold }: { row: EvalAxisRow; threshold: number }) {
  const tone = axisTone(row.mean, threshold);
  const pct = Math.max(0, Math.min(100, Math.round(row.mean * 100)));
  return (
    <li
      data-testid={`eval-axis-${row.axis}`}
      data-tone={tone}
      data-passed={row.passed}
      className="space-y-1"
    >
      <div className="flex items-center justify-between text-sm">
        <span className="font-medium text-[var(--foreground)]">{row.label}</span>
        <span className="tabular-nums text-[var(--muted-foreground)]">
          {row.mean.toFixed(2)}
        </span>
      </div>
      <div className="h-2 w-full overflow-hidden rounded-full bg-[var(--muted)]">
        <div
          className={cn("h-full rounded-full", TONE_BAR[tone])}
          style={{ width: `${pct}%` }}
        />
      </div>
    </li>
  );
}

/**
 * Honest provenance footer: how many scores came from REAL replayed
 * production interactions vs the synthetic golden set, plus the weakest
 * axis to improve next. When zero production rows exist the line says so
 * (NO THEATER — the synthetic-only score is labelled as such).
 */
function EvalScorecardFooter({ data }: { data: WorkerEvalReplay }) {
  const { production, merged, threshold } = data;
  const weakest = merged.weakestAxis;
  return (
    <div
      data-testid="eval-scorecard-footer"
      className="space-y-1 border-t border-[var(--border)]/60 pt-3 text-xs text-[var(--muted-foreground)]"
    >
      <p data-testid="eval-scorecard-provenance">
        {production.scored > 0
          ? `${production.scored} score${production.scored === 1 ? "" : "s"} from replayed real work`
          : "Synthetic golden scenarios only — no replayed real work yet"}
        <span className="opacity-70"> · threshold {threshold.toFixed(2)}</span>
      </p>
      {weakest ? (
        <p data-testid="eval-scorecard-weakest">
          Weakest axis: <span className="font-medium">{weakest.axis}</span> (
          {weakest.mean.toFixed(2)})
        </p>
      ) : null}
    </div>
  );
}
