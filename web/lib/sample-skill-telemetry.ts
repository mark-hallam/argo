/**
 * Sample skill-telemetry fixture for the `/skills/<id>` learning-loop
 * card (`SkillLearningLoopCard`).
 *
 * TODO(follow-up): wire to a real gateway endpoint —
 * `GET /api/v1/skills/<name>/telemetry` returning
 * `{ trust_history: [...], adoption_trend: number[], recent_runs: [...] }`.
 * Once the endpoint lands, replace consumers' fixture pulls with
 * `fetchSkillTelemetry` in `web/lib/api/skills.ts`. See the brief
 * `fix-skill-detail-learning-loop-telemetry-claim-4-evidence` for the
 * dressing-side-fixture allowance (per `feedback_no_theater_in_demo`,
 * acceptable while the camera does not drill into receipts).
 *
 * Shape mirrors the (planned) gateway envelope so the swap is
 * file-local. Keys/values are seed data — not real runs.
 */

/** One trust-grade promotion event, oldest → newest. */
export interface SkillTrustHistoryEvent {
  /** Canonical `argo_skill::TrustGrade` discriminant. */
  grade: string;
  /** ISO-8601 timestamp of the promotion. */
  awarded_at: string;
  /** Sample size that supported the promotion. */
  sample_size: number;
  /** One-line verdict-summary shown under the timestamp. */
  verdict_summary: string;
}

/** Last 3 runs for the "recent runs" sub-block. */
export interface SkillRecentRun {
  worker_id: string;
  worker_name: string;
  /** `approved` / `rejected` / `auto` etc. — short verdict noun. */
  verdict: string;
  /** Latency in milliseconds end-to-end. */
  latency_ms: number;
  /** ISO-8601 timestamp of the run. */
  ran_at: string;
}

/**
 * Telemetry bundle for one skill. `adoption_trend` is a `runs/day`
 * series in oldest → newest order (typically 30 points).
 */
export interface SkillTelemetry {
  /** Skill name (URL slug, matches `SkillRow.name`). */
  skill_name: string;
  /** Most-recent trust grade — the headline letter on the card. */
  current_grade: string;
  /** Date the current grade was awarded (one-line subheader). */
  current_grade_awarded_at: string;
  /** Cumulative successful-run count behind the current grade. */
  total_successful_runs: number;
  /** Distinct workers behind the current grade. */
  distinct_workers: number;
  /** Trust-grade history, oldest → newest. */
  trust_history: SkillTrustHistoryEvent[];
  /** Adoption trend — 30-point `runs/day` series, oldest → newest. */
  adoption_trend: number[];
  /** Last 3 runs, newest first. */
  recent_runs: SkillRecentRun[];
}

const PROMOTED: SkillTelemetry = {
  skill_name: "approval.route",
  current_grade: "published_verified",
  current_grade_awarded_at: "2026-04-12T08:30:00.000Z",
  total_successful_runs: 23,
  distinct_workers: 4,
  trust_history: [
    {
      grade: "tenant",
      awarded_at: "2026-03-22T11:05:00.000Z",
      sample_size: 6,
      verdict_summary: "Workspace promotion — 6 supervised approvals routed correctly.",
    },
    {
      grade: "published_verified",
      awarded_at: "2026-04-12T08:30:00.000Z",
      sample_size: 23,
      verdict_summary: "Marketplace promotion — 23 runs across 4 workers, 0 escalations.",
    },
  ],
  adoption_trend: [
    0, 0, 1, 1, 0, 2, 1, 3, 2, 1, 2, 3, 2, 4, 3, 5, 4, 6, 5, 4, 6, 7, 5, 6, 8, 7,
    9, 8, 7, 9,
  ],
  recent_runs: [
    {
      worker_id: "wkr-mariah",
      worker_name: "Mariah",
      verdict: "approved",
      latency_ms: 1280,
      ran_at: "2026-05-27T22:14:00.000Z",
    },
    {
      worker_id: "wkr-priya",
      worker_name: "Priya",
      verdict: "approved",
      latency_ms: 940,
      ran_at: "2026-05-27T18:02:00.000Z",
    },
    {
      worker_id: "wkr-mariah",
      worker_name: "Mariah",
      verdict: "auto",
      latency_ms: 612,
      ran_at: "2026-05-27T09:48:00.000Z",
    },
  ],
};

const HIGH_ADOPTION: SkillTelemetry = {
  skill_name: "pricing.competitor_scrape",
  current_grade: "published_verified",
  current_grade_awarded_at: "2026-02-08T10:00:00.000Z",
  total_successful_runs: 184,
  distinct_workers: 11,
  trust_history: [
    {
      grade: "tenant",
      awarded_at: "2026-01-19T09:00:00.000Z",
      sample_size: 12,
      verdict_summary: "Workspace promotion — 12 scrape runs against 3 vendors.",
    },
    {
      grade: "published_verified",
      awarded_at: "2026-02-08T10:00:00.000Z",
      sample_size: 47,
      verdict_summary: "Marketplace promotion — 47 runs across 9 workers, 0 false rows.",
    },
  ],
  adoption_trend: [
    3, 4, 5, 4, 6, 7, 6, 8, 9, 7, 10, 11, 9, 12, 13, 11, 14, 15, 12, 16, 17, 14,
    18, 19, 16, 20, 21, 18, 22, 23,
  ],
  recent_runs: [
    {
      worker_id: "wkr-priya",
      worker_name: "Priya",
      verdict: "approved",
      latency_ms: 2104,
      ran_at: "2026-05-28T02:10:00.000Z",
    },
    {
      worker_id: "wkr-mariah",
      worker_name: "Mariah",
      verdict: "approved",
      latency_ms: 1880,
      ran_at: "2026-05-27T23:45:00.000Z",
    },
    {
      worker_id: "wkr-jensen",
      worker_name: "Jensen",
      verdict: "approved",
      latency_ms: 2210,
      ran_at: "2026-05-27T20:12:00.000Z",
    },
  ],
};

const SAMPLES: Record<string, SkillTelemetry> = {
  "approval.route": PROMOTED,
  "seed-approval-route": PROMOTED,
  "pricing.competitor_scrape": HIGH_ADOPTION,
  "seed-pricing-competitor-scrape": HIGH_ADOPTION,
};

/**
 * Look up sample telemetry for a skill id or name. Returns `null` when
 * the skill has no recorded runs — the card switches to its "earns A
 * through real workers" empty state in that case so no history gets
 * fabricated.
 */
export function sampleSkillTelemetry(idOrName: string): SkillTelemetry | null {
  return SAMPLES[idOrName] ?? null;
}
