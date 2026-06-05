/**
 * Goal-vs-actual scorecard fetchers.
 *
 * Reads the live gateway endpoints authored under
 * `uf-scorecard-goal-attainment-surface`:
 *   - `GET /api/v1/workers/<id>/scorecard` — one worker's goals, each
 *     joining its declarative KPI target with the actual measured from
 *     the tenant-scoped audit trail.
 *   - `GET /api/v1/team/scorecard` — the "goals hit this week" rollup
 *     across every worker in the tenant.
 *
 * NO THEATER: a goal whose actual cannot yet be measured from a
 * countable audit source comes back with `measured: false` and a
 * `note` ("target set — measurement not wired"); it carries NO
 * `actual` / `attainment_pct`. The UI must render the note, never a
 * fabricated number. This file mirrors that contract exactly — it
 * does not invent fields the endpoint omits.
 */

import { apiGet } from "@/lib/api/client";

export type GoalStatus = "met" | "at_risk";
export type GoalComparator = "gte" | "lte" | "eq";

/** The configured pass/fail target for a goal. */
export interface ScorecardTarget {
  value: number;
  comparator: GoalComparator;
  /** Pre-rendered, e.g. "≤ 60000ms". */
  display: string;
}

/** One goal row: a KPI target joined with its measured actual. */
export interface ScorecardGoal {
  name: string;
  label: string;
  /** KPI kind tag, e.g. "duration_ms" | "percentage" | "count". */
  kind: string;
  /** Gate severity, e.g. "release" | "warn" | "inform". */
  gate: string;
  target: ScorecardTarget;
  /** What the actual was measured from, e.g. "response_latency_ms". */
  source: string;
  measured: boolean;
  /** Present only when `measured`. */
  actual?: { value: number; display: string };
  /** Present only when `measured`. May exceed 100 (target beaten). */
  attainment_pct?: number;
  /** Present only when `measured`. */
  status?: GoalStatus;
  /** Present only when NOT measured — the honest no-source note. */
  note?: string;
}

export interface WorkerScorecard {
  worker_id: string;
  worker_name: string;
  vertical_id: string;
  window_days: number;
  goals: ScorecardGoal[];
  summary: {
    goals_total: number;
    goals_measured: number;
    goals_met: number;
  };
}

export interface TeamScorecardWorker {
  worker_id: string;
  worker_name: string;
  vertical_id: string;
  goals_met: number;
  goals_measured: number;
}

export interface TeamScorecard {
  window_days: number;
  summary: { workers: number; goals_met: number; goals_measured: number };
  workers: TeamScorecardWorker[];
}

/** Fetch one worker's goal-attainment scorecard from the live endpoint. */
export function fetchWorkerScorecard(workerId: string): Promise<WorkerScorecard> {
  return apiGet<WorkerScorecard>(
    `/api/v1/workers/${encodeURIComponent(workerId)}/scorecard`,
  );
}

/** Fetch the team "goals hit this week" rollup. */
export function fetchTeamScorecard(): Promise<TeamScorecard> {
  return apiGet<TeamScorecard>("/api/v1/team/scorecard");
}

/**
 * Tone for a goal badge:
 *   - "muted"   — unmeasured (no source wired); render the note.
 *   - "success" — measured and met.
 *   - "warning" — measured and at risk.
 */
export function goalTone(goal: ScorecardGoal): "success" | "warning" | "muted" {
  if (!goal.measured) return "muted";
  return goal.status === "met" ? "success" : "warning";
}
