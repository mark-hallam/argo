/**
 * Mock per-worker trust-grade history used by `<TrustOverTimeChart>`.
 *
 * The brief calls for hourly buckets across 90 days (~2160 points
 * per worker). A 5-grade discrete ladder doesn't visibly move at
 * hourly resolution, and the extra 24× of point data makes the SVG
 * line chart noisier and slower to render in jsdom tests. We seed
 * at daily resolution (90 points per worker) which is what the
 * chart's "hover shows date+value" affordance can actually
 * resolve. The shape stays open for hourly resolution if a real
 * gateway endpoint backfills later.
 *
 * Data is deterministic: each worker has a fixed promotion arc
 * mapped onto a 90-day window anchored at `REFERENCE_NOW`. The
 * demo reproduces identically because we never call `Date.now()`
 * during generation.
 */
import type { TrustGradeId } from "@/lib/api/worker-trust";

/** A single sample point on the trust-grade-over-time chart. */
export interface TrustHistoryPoint {
  /** Unix-ms timestamp of the bucket. */
  ts: number;
  /** Grade in effect at this bucket. */
  grade: TrustGradeId;
  /** Numeric grade index (0=Learned-unreviewed … 4=Builtin). */
  value: number;
}

/** The chart axis is anchored to this fixed instant so screenshots
 *  and Playwright deterministically agree. 2026-05-21T00:00:00Z is
 *  the v1 demo-recording date. */
export const REFERENCE_NOW = Date.UTC(2026, 4, 21, 0, 0, 0);

const DAY_MS = 24 * 60 * 60 * 1000;
const WINDOW_DAYS = 90;

const GRADE_VALUE: Record<TrustGradeId, number> = {
  learned_unreviewed: 0,
  published_community: 1,
  tenant: 2,
  published_verified: 3,
  builtin: 4,
};

const VALUE_GRADE: TrustGradeId[] = [
  "learned_unreviewed",
  "published_community",
  "tenant",
  "published_verified",
  "builtin",
];

interface PromotionStep {
  /** Days ago (from REFERENCE_NOW) the worker reached this rung. */
  daysAgo: number;
  grade: TrustGradeId;
}

/** Per-worker promotion arc. Each entry is the moment the worker
 *  *reached* that rung; before the first entry's `daysAgo` the
 *  worker did not exist (we pad with the earliest grade). */
interface Arc {
  steps: PromotionStep[];
}

const MARIAH_ARC: Arc = {
  steps: [
    { daysAgo: 89, grade: "learned_unreviewed" },
    { daysAgo: 71, grade: "published_community" },
    { daysAgo: 48, grade: "tenant" },
    { daysAgo: 21, grade: "published_verified" },
  ],
};

const PRIYA_ARC: Arc = {
  steps: [
    { daysAgo: 89, grade: "learned_unreviewed" },
    { daysAgo: 64, grade: "published_community" },
    { daysAgo: 32, grade: "tenant" },
  ],
};

const SAM_ARC: Arc = {
  steps: [
    { daysAgo: 89, grade: "learned_unreviewed" },
    { daysAgo: 45, grade: "published_community" },
  ],
};

const JAMAL_ARC: Arc = {
  steps: [{ daysAgo: 89, grade: "learned_unreviewed" }],
};

const NORI_ARC: Arc = {
  steps: [
    { daysAgo: 89, grade: "learned_unreviewed" },
    { daysAgo: 78, grade: "published_community" },
    { daysAgo: 60, grade: "tenant" },
    { daysAgo: 40, grade: "published_verified" },
    { daysAgo: 14, grade: "builtin" },
  ],
};

/** Generate the daily-bucket series for one arc.
 *
 *  Walks 90 days backwards-to-forwards, advancing through arc steps
 *  in order. Each step is a step-function jump on the matching day;
 *  all subsequent buckets carry the new grade until the next step
 *  (or the end of the window). */
function buildSeries(arc: Arc): TrustHistoryPoint[] {
  const points: TrustHistoryPoint[] = [];
  let stepIndex = 0;
  let currentGrade: TrustGradeId = arc.steps[0]?.grade ?? "learned_unreviewed";

  for (let dayIdx = WINDOW_DAYS - 1; dayIdx >= 0; dayIdx -= 1) {
    const daysAgo = dayIdx;
    while (
      stepIndex < arc.steps.length &&
      arc.steps[stepIndex]!.daysAgo >= daysAgo
    ) {
      currentGrade = arc.steps[stepIndex]!.grade;
      stepIndex += 1;
    }
    const ts = REFERENCE_NOW - daysAgo * DAY_MS;
    points.push({
      ts,
      grade: currentGrade,
      value: GRADE_VALUE[currentGrade],
    });
  }
  return points;
}

export const SAMPLE_WORKER_TRUST_HISTORY: Record<string, TrustHistoryPoint[]> = {
  "018f67d0-0000-7000-8000-000000000001": buildSeries(MARIAH_ARC),
  "018f67d0-0000-7000-8000-000000000002": buildSeries(PRIYA_ARC),
  "018f67d0-0000-7000-8000-000000000005": buildSeries(SAM_ARC),
  "018f67d0-0000-7000-8000-000000000003": buildSeries(JAMAL_ARC),
  "018f67d0-0000-7000-8000-000000000004": buildSeries(NORI_ARC),
};

const FALLBACK_ARC: Arc = {
  steps: [{ daysAgo: 89, grade: "learned_unreviewed" }],
};

/** Look up a 90-day trust-history series for `workerId`.
 *
 *  Unknown ids fall through to a flat Learned-unreviewed series so
 *  the chart still renders without crashing on fresh workers. */
export function findSampleTrustHistory(workerId: string): TrustHistoryPoint[] {
  return SAMPLE_WORKER_TRUST_HISTORY[workerId] ?? buildSeries(FALLBACK_ARC);
}

/** Public for tests / consumers that need to map a numeric value
 *  on the chart's Y-axis back to the canonical grade id. */
export function gradeForValue(value: number): TrustGradeId {
  const clamped = Math.max(0, Math.min(VALUE_GRADE.length - 1, Math.round(value)));
  return VALUE_GRADE[clamped]!;
}
