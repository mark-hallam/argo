"use client";

import { KpiSparkline } from "@/components/ui/charts";
import { TrustGradeChip } from "@/components/skycot/skills/TrustGradeChip";
import type {
  SkillRecentRun,
  SkillTelemetry,
  SkillTrustHistoryEvent,
} from "@/lib/sample-skill-telemetry";

/**
 * Learning-loop evidence card for the `/skills/<id>` detail page.
 *
 * Backs the v1.0-demo Scene 4 narration ("the skill marketplace ships,
 * and your skills learn") — the camera drills into a skill and needs
 * to see HOW it got to its current trust grade, not just the static
 * "Trust · A" chip the detail page used to render in isolation.
 *
 * Three sub-blocks, top-to-bottom:
 *
 *   1. Trust-grade history — vertical timeline of promotions with
 *      date, sample size, and a one-line verdict-summary per event.
 *   2. Adoption trend — 30-point `runs/day` sparkline, oldest → newest.
 *   3. Recent runs — last three runs as (worker, verdict, latency).
 *
 * Empty state: when no telemetry is recorded for the skill (a fresh
 * marketplace install with zero runs), the card swaps the three
 * sub-blocks for an "earns A through real workers" message so we
 * never fabricate trust history.
 */

export interface SkillLearningLoopCardProps {
  /** Skill name / id used as the heading anchor. */
  skillName: string;
  /** Telemetry bundle. `null` → empty state. */
  telemetry: SkillTelemetry | null;
  /** Override the data-testid (defaults to `skill-learning-loop-card`). */
  testId?: string;
}

function formatAwardedDate(iso: string): string {
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return "unknown date";
  return d.toLocaleDateString("en-AU", {
    day: "numeric",
    month: "short",
    year: "numeric",
    timeZone: "UTC",
  });
}

function formatRunTime(iso: string): string {
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return "unknown";
  const now = new Date();
  const diffMs = now.getTime() - d.getTime();
  const day = 24 * 60 * 60 * 1000;
  if (diffMs < day) {
    return d.toLocaleTimeString("en-AU", {
      hour: "numeric",
      minute: "2-digit",
      timeZone: "UTC",
      hour12: false,
    }) + " UTC";
  }
  return d.toLocaleDateString("en-AU", {
    day: "numeric",
    month: "short",
    timeZone: "UTC",
  });
}

function formatLatency(ms: number): string {
  if (ms < 1000) return `${ms}ms`;
  return `${(ms / 1000).toFixed(1)}s`;
}

function TrustHistoryTimeline({
  events,
}: {
  events: SkillTrustHistoryEvent[];
}) {
  return (
    <ol
      className="learn-timeline"
      data-testid="skill-learning-loop-history"
    >
      {events.map((event, idx) => (
        <li
          className="learn-timeline-row"
          key={`${event.grade}-${event.awarded_at}`}
          data-testid={`skill-learning-loop-history-${idx}`}
        >
          <span className="learn-timeline-dot" aria-hidden="true" />
          <div className="learn-timeline-body">
            <div className="learn-timeline-head">
              <TrustGradeChip grade={event.grade} testId={`skill-learning-loop-history-grade-${idx}`} />
              <span className="learn-timeline-when">
                {formatAwardedDate(event.awarded_at)} · {event.sample_size} run
                {event.sample_size === 1 ? "" : "s"}
              </span>
            </div>
            <p className="learn-timeline-summary">{event.verdict_summary}</p>
          </div>
        </li>
      ))}
    </ol>
  );
}

function RecentRunsList({ runs }: { runs: SkillRecentRun[] }) {
  return (
    <ul
      className="learn-runs"
      data-testid="skill-learning-loop-runs"
      role="list"
    >
      {runs.map((run, idx) => (
        <li
          className="learn-runs-row"
          key={`${run.worker_id}-${run.ran_at}`}
          data-testid={`skill-learning-loop-run-${idx}`}
        >
          <span className="learn-runs-worker">{run.worker_name}</span>
          <span
            className="learn-runs-verdict"
            data-verdict={run.verdict}
          >
            {run.verdict}
          </span>
          <span className="learn-runs-latency">{formatLatency(run.latency_ms)}</span>
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

  if (telemetry === null) {
    return (
      <section
        className="usage-card learn-card"
        aria-labelledby="skill-learning-loop-heading"
        data-testid={resolvedTestId}
        data-state="empty"
      >
        <h2 className="usage-card-title" id="skill-learning-loop-heading">
          Learning loop · Trust grade B
        </h2>
        <p
          className="learn-empty"
          data-testid="skill-learning-loop-empty"
        >
          No runs yet — <strong>{skillName}</strong> ships at Trust B and earns A
          through real workers. The history, adoption trend, and recent runs
          appear here as soon as your workers start using this skill.
        </p>
      </section>
    );
  }

  const subheader =
    `Earned ${telemetry.current_grade === "published_verified" || telemetry.current_grade === "builtin" ? "A" : telemetry.current_grade === "tenant" ? "B" : ""}` +
    ` on ${formatAwardedDate(telemetry.current_grade_awarded_at)}` +
    ` after ${telemetry.total_successful_runs} successful run${telemetry.total_successful_runs === 1 ? "" : "s"}` +
    ` across ${telemetry.distinct_workers} worker${telemetry.distinct_workers === 1 ? "" : "s"}.`;

  const headerLetter =
    telemetry.current_grade === "published_verified" || telemetry.current_grade === "builtin"
      ? "A"
      : telemetry.current_grade === "tenant"
        ? "B"
        : telemetry.current_grade === "published_community"
          ? "C"
          : "—";

  return (
    <section
      className="usage-card learn-card"
      aria-labelledby="skill-learning-loop-heading"
      data-testid={resolvedTestId}
      data-state="populated"
    >
      <h2 className="usage-card-title" id="skill-learning-loop-heading">
        Learning loop · Trust grade {headerLetter}
      </h2>
      <p
        className="learn-subheader"
        data-testid="skill-learning-loop-subheader"
      >
        {subheader}
      </p>

      <div className="learn-block">
        <h3 className="learn-block-title">Trust-grade history</h3>
        <TrustHistoryTimeline events={telemetry.trust_history} />
      </div>

      <div className="learn-block">
        <div className="learn-block-head">
          <h3 className="learn-block-title">Adoption trend</h3>
          <span className="learn-block-aside">last {telemetry.adoption_trend.length} days · runs/day</span>
        </div>
        <div
          className="learn-sparkline"
          data-testid="skill-learning-loop-sparkline"
        >
          <KpiSparkline
            data={telemetry.adoption_trend}
            tone="success"
            width={520}
            height={56}
            ariaLabel={`Adoption trend for ${skillName} — ${telemetry.adoption_trend.length}-day runs/day series`}
          />
        </div>
      </div>

      <div className="learn-block">
        <h3 className="learn-block-title">Recent runs</h3>
        <RecentRunsList runs={telemetry.recent_runs} />
      </div>
    </section>
  );
}
