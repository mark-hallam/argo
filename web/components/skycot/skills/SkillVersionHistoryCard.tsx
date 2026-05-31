"use client";

import type { SkillVersionEntry, SkillVersionsResponse } from "@/lib/api/skills";

/**
 * Version-history evidence card for the `/skills/<id>` detail page (C3).
 *
 * Shows the REAL version timeline for a skill (from the `skill` table)
 * joined with its eval-delta changelog (from `skill_reeval_history`),
 * via `GET /api/v1/skills/<id-or-name>/versions`.
 *
 * REAL DATA ONLY (NO THEATER). Each row is a real installed version with
 * its real `superseded` / `lifecycle_state` flags; the eval line is the
 * LATEST re-eval run for that version with a real pass-rate delta in
 * percentage points (e.g. "v2 · +12.0pp pass-rate · 2 runs"). A version
 * with no recorded re-eval runs shows "no eval runs yet" rather than a
 * fabricated promotion arc — there is no trust-grade promotion-event
 * table to source one from.
 *
 * Empty / single-version state: when `versions` is `null` (gateway
 * offline) or the skill has a single version, the card reports that
 * honestly instead of inventing a history.
 */

export interface SkillVersionHistoryCardProps {
  /** Skill name / id used in the heading. */
  skillName: string;
  /** REAL version bundle. `null` → honest empty/offline state. */
  versions: SkillVersionsResponse | null;
  /** Override the data-testid (defaults to `skill-version-history-card`). */
  testId?: string;
}

function formatDate(iso: string): string {
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return "unknown";
  return d.toLocaleDateString("en-AU", {
    day: "numeric",
    month: "short",
    year: "numeric",
    timeZone: "UTC",
  });
}

/** Render a 0..1 pass rate as a whole-number percent. */
function formatPassRate(rate: number | null): string {
  if (rate === null) return "—";
  return `${Math.round(rate * 100)}%`;
}

/**
 * Plain-English eval line for one version. Returns `null` when the
 * version has no recorded re-eval (so the caller shows "no eval runs
 * yet" rather than a fabricated delta).
 */
function evalSummary(entry: SkillVersionEntry): string | null {
  const e = entry.eval;
  if (!e) return null;
  const runs = `${e.run_count} run${e.run_count === 1 ? "" : "s"}`;
  if (e.delta_pp === null) {
    // First (baseline) eval — no prior to diff against.
    return `${formatPassRate(e.pass_rate)} pass · baseline · ${runs}`;
  }
  const sign = e.delta_pp > 0 ? "+" : "";
  const verb = e.decision === "regressed" ? "regressed" : "pass-rate";
  return `${formatPassRate(e.pass_rate)} pass · ${sign}${e.delta_pp.toFixed(1)}pp ${verb} · ${runs}`;
}

export function SkillVersionHistoryCard({
  skillName,
  versions,
  testId,
}: SkillVersionHistoryCardProps) {
  const resolvedTestId = testId ?? "skill-version-history-card";
  const list = versions?.versions ?? [];

  // Honest empty state: no bundle (offline) or no versions at all.
  if (!versions || list.length === 0) {
    return (
      <section
        className="usage-card learn-card"
        aria-labelledby="skill-version-history-heading"
        data-testid={resolvedTestId}
        data-state="empty"
      >
        <h2 className="usage-card-title" id="skill-version-history-heading">
          Version history
        </h2>
        <p className="learn-empty" data-testid="skill-version-history-empty">
          No version history yet — <strong>{skillName}</strong> will list each version
          here, with its eval pass-rate movement, as it ships updates.
        </p>
      </section>
    );
  }

  const single = list.length === 1;
  const subheader = single
    ? "One version installed — the eval-delta changelog grows as new versions ship."
    : `${versions.version_count} versions · newest first, with each version's eval pass-rate movement.`;

  return (
    <section
      className="usage-card learn-card"
      aria-labelledby="skill-version-history-heading"
      data-testid={resolvedTestId}
      data-state="populated"
    >
      <h2 className="usage-card-title" id="skill-version-history-heading">
        Version history
      </h2>
      <p className="learn-subheader" data-testid="skill-version-history-subheader">
        {subheader}
      </p>

      <ol className="learn-timeline" data-testid="skill-version-history-timeline">
        {list.map((entry, idx) => {
          const evalLine = evalSummary(entry);
          return (
            <li
              className="learn-timeline-row"
              key={`${entry.id}-${entry.version}`}
              data-testid={`skill-version-row-${idx}`}
            >
              <span className="learn-timeline-dot" aria-hidden="true" />
              <div className="learn-timeline-body">
                <div className="learn-timeline-head">
                  <span
                    className="learn-runs-verdict"
                    data-verdict={entry.superseded ? "rejected" : "approved"}
                  >
                    {entry.superseded ? "superseded" : entry.lifecycle_state}
                  </span>
                  <span className="learn-timeline-when">
                    v{entry.version} · {formatDate(entry.created_at)}
                  </span>
                </div>
                <p className="skill-version-eval" data-testid={`skill-version-eval-${idx}`}>
                  {evalLine ?? "No eval runs yet for this version."}
                </p>
              </div>
            </li>
          );
        })}
      </ol>
    </section>
  );
}
