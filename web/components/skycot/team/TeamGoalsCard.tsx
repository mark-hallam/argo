"use client";

import { useTeamScorecard } from "@/lib/hooks/use-worker-scorecard";

/**
 * Team "goals hit this week" rollup.
 *
 * Reads `GET /api/v1/team/scorecard` (the trailing 7 days) and shows
 * how many measured goals the whole team met. NO THEATER: it counts
 * only goals with a real countable audit source — unmeasured targets
 * are excluded from the denominator, so the ratio is never inflated by
 * goals that have no measurement wired yet.
 *
 * Renders nothing until there is at least one measured goal across the
 * team, so a brand-new workspace (every target unmeasured) shows no
 * fabricated "0/0" banner.
 */
export function TeamGoalsCard() {
  const query = useTeamScorecard();
  const data = query.data;
  if (!data || data.summary.goals_measured === 0) return null;

  const { goals_met, goals_measured } = data.summary;
  return (
    <div
      data-testid="team-goals-rollup"
      className="mx-auto max-w-5xl px-4 sm:px-6 lg:px-8 pt-6"
    >
      <div className="flex items-center justify-between gap-4 rounded-[var(--radius-lg)] border border-[var(--border)] bg-[var(--card)]/60 px-5 py-4">
        <div>
          <p className="text-sm font-semibold text-[var(--foreground)]">
            Goals hit this week
          </p>
          <p className="text-xs text-[var(--muted-foreground)]">
            Across {data.summary.workers}{" "}
            {data.summary.workers === 1 ? "teammate" : "teammates"} · last{" "}
            {data.window_days} days
          </p>
        </div>
        <p className="text-2xl font-bold tabular-nums text-[var(--foreground)]">
          {goals_met}
          <span className="text-base font-medium text-[var(--muted-foreground)]">
            /{goals_measured}
          </span>
        </p>
      </div>
    </div>
  );
}
