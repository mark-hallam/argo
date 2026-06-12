/**
 * Worker eval-REPLAY scorecard API client.
 *
 * Reads the live gateway endpoint authored under
 * `routes_worker_evals_replay.rs`:
 *   - `GET /api/v1/workers/<id>/evals/replay` — the teammate's 6-axis
 *     eval-replay scorecard, merging hand-authored SYNTHETIC golden
 *     cases with the worker's real PRODUCTION interactions replayed off
 *     the tamper-evident audit chain.
 *
 * NO THEATER: the route grades the teammate only on rows that genuinely
 * exist. A worker with no replay-shaped audit rows comes back with
 * `production.scored === 0` — the UI must render that as an honest
 * "no eval runs yet" state, never a fabricated score. This module
 * mirrors that contract exactly; it never invents axes the endpoint
 * omits and carries no mock fallback (a missing endpoint surfaces as an
 * error, not a synthesised scorecard).
 */

import { apiGet } from "@/lib/api/client";

/**
 * The six eval axes, in `argo_eval::teammate_scorecard::AXIS_NAMES`
 * order — the SAME order the route's `axisMeans` array is emitted in,
 * so a positional zip labels each mean correctly.
 */
export const EVAL_AXIS_NAMES = [
  "correctness",
  "escalation",
  "boundary",
  "skill_usage",
  "honesty",
  "budget",
] as const;

export type EvalAxis = (typeof EVAL_AXIS_NAMES)[number];

/** Operator-facing label for each axis. */
export const EVAL_AXIS_LABELS: Record<EvalAxis, string> = {
  correctness: "Correctness",
  escalation: "Escalation",
  boundary: "Boundary",
  skill_usage: "Skill usage",
  honesty: "Honesty",
  budget: "Budget",
};

/** One rendered axis row: the axis, its mean, and whether it passed. */
export interface EvalAxisRow {
  axis: EvalAxis;
  label: string;
  /** Mean score 0..1 across the scored scenarios. */
  mean: number;
  /** True when `mean >= threshold`. */
  passed: boolean;
}

/** The camelCase scorecard shape the route's `scorecard_json` emits. */
export interface EvalScorecardData {
  teammate: string;
  /** Scenarios that passed every axis. */
  passed: number;
  /** Total scenarios scored. */
  total: number;
  /** Headline pass-rate, a fraction 0..1. */
  passRate: number;
  /** Per-axis means, in `EVAL_AXIS_NAMES` order. */
  axisMeans: number[];
  /** Per-axis pass threshold (DEC-017 default 0.7). */
  threshold: number;
  /** Lowest-scoring axis — "points the next improvement"; null if empty. */
  weakestAxis: { axis: string; mean: number } | null;
}

/** Production half of the replay: live-interaction coverage + scorecard. */
export interface EvalReplayProduction {
  /** Production interactions that paired to a synthetic scenario. */
  scored: number;
  /** Production interactions with no matching scenario. */
  unmatched: number;
  /** Synthetic scenarios with no production interaction yet. */
  uncovered: number;
  scorecard: EvalScorecardData;
}

/** Full `GET /workers/<id>/evals/replay` body. */
export interface WorkerEvalReplay {
  workerId: string;
  teammate: string;
  threshold: number;
  limit: number;
  /** Hand-authored golden expectation set. */
  synthetic: EvalScorecardData;
  /** Real past interactions replayed off the audit chain. */
  production: EvalReplayProduction;
  /** Synthetic + production combined — the headline scorecard. */
  merged: EvalScorecardData;
}

/** Fetch one worker's eval-replay scorecard from the live endpoint. */
export function fetchWorkerEvalReplay(workerId: string): Promise<WorkerEvalReplay> {
  return apiGet<WorkerEvalReplay>(
    `/api/v1/workers/${encodeURIComponent(workerId)}/evals/replay`,
  );
}

/**
 * Expand a scorecard's flat `axisMeans` array into labelled rows,
 * computing each axis's pass against the card threshold. Extra means
 * beyond the six known axes are ignored; missing means are skipped.
 */
export function toAxisRows(card: EvalScorecardData): EvalAxisRow[] {
  return EVAL_AXIS_NAMES.flatMap((axis, i) => {
    const mean = card.axisMeans[i];
    if (typeof mean !== "number") return [];
    return [
      {
        axis,
        label: EVAL_AXIS_LABELS[axis],
        mean,
        passed: mean >= card.threshold,
      },
    ];
  });
}

/**
 * Tone for an axis bar:
 *   - "success" — at or above the threshold.
 *   - "warning" — below the threshold (the weak axis to improve).
 */
export function axisTone(mean: number, threshold: number): "success" | "warning" {
  return mean >= threshold ? "success" : "warning";
}
