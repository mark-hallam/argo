"use client";

import { Card, CardBody, CardHeader, CardTitle } from "@/components/ui/Card";
import { useWorkerScorecard } from "@/lib/hooks/use-worker-scorecard";
import { goalTone, type ScorecardGoal } from "@/lib/api/scorecard";
import { cn } from "@/lib/utils";

/**
 * Goal-vs-actual scorecard for one worker.
 *
 * Each row shows the operator's declared target, the measured actual,
 * attainment %, and a met / at-risk pill. Goals with no countable
 * audit source yet show the honest "measurement not wired" note and
 * NO number — the surface never fabricates an actual (NO THEATER).
 */
export function ScorecardCard({ workerId }: { workerId: string }) {
  const query = useWorkerScorecard(workerId);
  const data = query.data;

  return (
    <Card data-testid="card-scorecard">
      <CardHeader>
        <CardTitle>Goals vs actual</CardTitle>
        {data ? (
          <p className="text-sm text-[var(--muted-foreground)]">
            {data.summary.goals_met}/{data.summary.goals_measured} measured goals met
            <span className="opacity-60"> · last {data.window_days} days</span>
          </p>
        ) : null}
      </CardHeader>
      <CardBody className="pt-1">
        {query.isError ? (
          <p className="text-sm text-[var(--muted-foreground)]">
            Couldn&apos;t load this scorecard.
          </p>
        ) : query.isLoading ? (
          <p className="text-sm text-[var(--muted-foreground)]">Loading goals…</p>
        ) : !data || data.goals.length === 0 ? (
          <p className="text-sm text-[var(--muted-foreground)]">
            No goals defined for this teammate yet.
          </p>
        ) : (
          <ul role="list" className="divide-y divide-[var(--border)]/60">
            {data.goals.map((g) => (
              <GoalRow key={g.name} goal={g} />
            ))}
          </ul>
        )}
      </CardBody>
    </Card>
  );
}

const TONE_PILL: Record<"success" | "warning" | "muted", string> = {
  success: "bg-emerald-100 text-emerald-800 border-emerald-300/60",
  warning: "bg-amber-100 text-amber-900 border-amber-300/60",
  muted: "bg-[var(--muted)] text-[var(--muted-foreground)] border-[var(--border)]",
};

function GoalRow({ goal }: { goal: ScorecardGoal }) {
  const tone = goalTone(goal);
  const pillLabel = !goal.measured
    ? "Not wired"
    : goal.status === "met"
      ? "Met"
      : "At risk";
  return (
    <li
      data-testid={`goal-row-${goal.name}`}
      data-tone={tone}
      data-measured={goal.measured}
      className="grid grid-cols-[1fr_auto] items-center gap-x-4 gap-y-1 py-3"
    >
      <div className="min-w-0">
        <p className="truncate text-sm font-medium text-[var(--foreground)]">
          {goal.label}
        </p>
        {goal.measured ? (
          <p className="text-xs text-[var(--muted-foreground)]">
            Target {goal.target.display} · actual {goal.actual?.display}
            {typeof goal.attainment_pct === "number" ? (
              <span className="opacity-80"> · {goal.attainment_pct}%</span>
            ) : null}
          </p>
        ) : (
          <p className="text-xs text-[var(--muted-foreground)]">
            Target {goal.target.display} · {goal.note ?? "measurement not wired"}
          </p>
        )}
      </div>
      <span
        className={cn(
          "justify-self-end rounded-full border px-2 py-0.5 text-xs font-medium",
          TONE_PILL[tone],
        )}
      >
        {pillLabel}
      </span>
    </li>
  );
}
