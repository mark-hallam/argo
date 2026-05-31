"use client";

import { KpiSparkline } from "@/components/ui/charts";
import type { SkillEvalRun, SkillTelemetry } from "@/lib/api/skills";

/**
 * Learning-loop evidence card for the `/skills/<id>` detail page.
 *
 * Backs the v1.0-demo Scene 4 narration ("the skill marketplace ships,
 * and your skills learn") — the camera drills into a skill and sees HOW
 * its quality has trended over real eval re-runs and how its adoption
 * has grown, not just the static "Trust · A" chip.
 *
 * REAL DATA ONLY (NO THEATER). The telemetry comes from the gateway
 * `GET /api/v1/skills/<id>/telemetry` endpoint, which sources every
 * field from a live table (`skill_reeval_history`, `worker_skill`, the
 * skill's provenance). The card RETIRED the hardcoded
 * `sample-skill-telemetry.ts` fixture (the old "184 runs across 11
 * workers" MOCK). Fields the fixture invented — per-run latency and a
 * per-run worker name — are gone, because the underlying tables carry
 * no such data; surfacing them would be theater.
 *
 * Three sub-blocks, top-to-bottom:
 *
 *   1. Eval re-run history — vertical timeline of scheduled eval
 *      re-runs with version, pass rate, and the stable/regressed
 *      decision. (Not trust-grade promotions — there is no promotion
 *      table to source those from.)
 *   2. Adoption trend — cumulative `workers who learned this skill`
 *      sparkline, oldest → newest.
 *   3. Recent runs — the newest three eval re-runs.
 *
 * Empty state: when `telemetry` is `null` (a fresh skill with zero
 * recorded runs, or the gateway is offline), the card swaps the three
 * sub-blocks for an "earns A through real workers" message so we never
 * fabricate history.
 */

export interface SkillLearningLoopCardProps {
  /** Skill name / id used as the heading anchor. */
  skillName: string;
  /** REAL telemetry bundle. `null` → honest empty state. */
  telemetry: SkillTelemetry | null;
  /** Override the data-testid (defaults to `skill-learning-loop-card`). */
  testId?: string;
}

/** Map a canonical TrustGrade discriminant to the A/B/C card letter. */
function gradeLetter(grade: string): string {
  if (grade === "published_verified" || grade === "builtin") return "A";
  if (grade === "tenant") return "B";
  if (grade === "published_community") return "C";
  return "—";
}

function formatRunTime(iso: string): string {
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return "unknown";
  const now = new Date();
  const diffMs = now.getTime() - d.getTime();
  const day = 24 * 60 * 60 * 1000;
  if (diffMs < day) {
    return (
      d.toLocaleTimeString("en-AU", {
        hour: "numeric",
        minute: "2-digit",
        timeZone: "UTC",
        hour12: false,
      }) + " UTC"
    );
  }
  return d.toLocaleDateString("en-AU", {
    day: "numeric",
    month: "short",
    timeZone: "UTC",
  });
}

/** Render a 0..1 pass rate as a whole-number percent (e.g. `0.91` → `91%`). */
function formatPassRate(rate: number | null | undefined): string {
  if (rate === null || rate === undefined) return "—";
  return `${Math.round(rate * 100)}%`;
}

/** Operator-facing label for a real eval decision. */
function decisionLabel(decision: SkillEvalRun["decision"]): string {
  switch (decision) {
    case "regressed":
      return "regressed";
    case "no_baseline":
      return "baseline";
    case "stable":
    default:
      return "stable";
  }
}

function EvalHistoryTimeline({ runs }: { runs: SkillEvalRun[] }) {
  return (
    <ol className="learn-timeline" data-testid="skill-learning-loop-history">
      {runs.map((run, idx) => (
        <li
          className="learn-timeline-row"
          key={`${run.version}-${run.ran_at}-${idx}`}
          data-testid={`skill-learning-loop-history-${idx}`}
        >
          <span className="learn-timeline-dot" aria-hidden="true" />
          <div className="learn-timeline-body">
            <div className="learn-timeline-head">
              <span
                className="learn-runs-verdict"
                data-verdict={run.decision === "regressed" ? "rejected" : "approved"}
              >
                {decisionLabel(run.decision)}
              </span>
              <span className="learn-timeline-when">
                {run.version} · {formatPassRate(run.pass_rate)} pass ·{" "}
                {formatRunTime(run.ran_at)}
              </span>
            </div>
          </div>
        </li>
      ))}
    </ol>
  );
}

function RecentRunsList({ runs }: { runs: SkillEvalRun[] }) {
  return (
    <ul className="learn-runs" data-testid="skill-learning-loop-runs" role="list">
      {runs.map((run, idx) => (
        <li
          className="learn-runs-row learn-runs-row-eval"
          key={`${run.version}-${run.ran_at}-${idx}`}
          data-testid={`skill-learning-loop-run-${idx}`}
        >
          <span className="learn-runs-worker">{run.version}</span>
          <span className="learn-runs-verdict" data-verdict={run.decision === "regressed" ? "rejected" : "approved"}>
            {decisionLabel(run.decision)}
          </span>
          <span className="learn-runs-latency">{formatPassRate(run.pass_rate)} pass</span>
          <span className="learn-runs-when">{formatRunTime(run.ran_at)}</span>
        </li>
      ))}
    </ul>
  );
}

export function SkillLearningLoopCard({
  skillName,
  telemetry,
  testId,
}: SkillLearningLoopCardProps) {
  const resolvedTestId = testId ?? "skill-learning-loop-card";

  // Honest empty state: null telemetry OR a real bundle with zero runs.
  const hasRuns = telemetry !== null && telemetry.run_count > 0;

  if (!hasRuns) {
    return (
      <section
        className="usage-card learn-card"
        aria-labelledby="skill-learning-loop-heading"
        data-testid={resolvedTestId}
        data-state="empty"
      >
        <h2 className="usage-card-title" id="skill-learning-loop-heading">
          Learning loop · Trust grade {telemetry ? gradeLetter(telemetry.current_grade) : "B"}
        </h2>
        <p className="learn-empty" data-testid="skill-learning-loop-empty">
          No runs yet — <strong>{skillName}</strong> earns its trust grade through real
          workers. The eval history, adoption trend, and recent runs appear here as soon
          as your workers start using this skill.
        </p>
      </section>
    );
  }

  const letter = gradeLetter(telemetry.current_grade);
  const subheader =
    `Trust grade ${letter} · ${telemetry.run_count} eval re-run` +
    `${telemetry.run_count === 1 ? "" : "s"}` +
    ` across ${telemetry.distinct_workers} worker${telemetry.distinct_workers === 1 ? "" : "s"}.`;

  return (
    <section
      className="usage-card learn-card"
      aria-labelledby="skill-learning-loop-heading"
      data-testid={resolvedTestId}
      data-state="populated"
    >
      <h2 className="usage-card-title" id="skill-learning-loop-heading">
        Learning loop · Trust grade {letter}
      </h2>
      <p className="learn-subheader" data-testid="skill-learning-loop-subheader">
        {subheader}
      </p>

      {telemetry.eval_history.length > 0 && (
        <div className="learn-block">
          <h3 className="learn-block-title">Eval re-run history</h3>
          <EvalHistoryTimeline runs={telemetry.eval_history} />
        </div>
      )}

      {telemetry.adoption_trend.length > 0 && (
        <div className="learn-block">
          <div className="learn-block-head">
            <h3 className="learn-block-title">Adoption trend</h3>
            <span className="learn-block-aside">
              last {telemetry.adoption_trend.length} days · workers
            </span>
          </div>
          <div className="learn-sparkline" data-testid="skill-learning-loop-sparkline">
            <KpiSparkline
              data={telemetry.adoption_trend}
              tone="success"
              width={520}
              height={56}
              ariaLabel={`Adoption trend for ${skillName} — ${telemetry.adoption_trend.length}-day cumulative worker series`}
            />
          </div>
        </div>
      )}

      {telemetry.recent_runs.length > 0 && (
        <div className="learn-block">
          <h3 className="learn-block-title">Recent runs</h3>
          <RecentRunsList runs={telemetry.recent_runs} />
        </div>
      )}
    </section>
  );
}
