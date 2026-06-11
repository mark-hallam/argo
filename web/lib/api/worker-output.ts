/**
 * Worker run-output API client (brief `uf-spa-teammate-output-surface`).
 *
 * Surfaces a single teammate run's REAL work product — the assistant's
 * reasoning/plan (`assistant_output.md`), the parsed `result.json`, and
 * the proposed `actions.json` summary — read from the on-disk artifact
 * bundle by the gateway route
 * `GET /api/v1/workers/<id>/runs/<job_id>/output`.
 *
 * NO THEATER: there is deliberately no mock/sample fallback. A run's
 * output is either real (served by the gateway for the owning tenant) or
 * absent. An anonymous/demo visitor, a missing run, or a cross-tenant
 * `job_id` (which the gateway 404s) all degrade to `null` — the caller
 * renders an honest empty state, never fabricated output.
 */

import { apiGet } from "@/lib/api/client";
import { hasRealSession, isDemoLive } from "@/lib/skycot/demo-mode";
import { isMissingOrUnreachable } from "./unreachable";

/** Summary of the proposed actions block in the run's `actions.json`. */
export interface RunActionCounts {
  total?: number;
  pendingApproval?: number;
  pendingAutonomous?: number;
  blocked?: number;
}

/** One teammate run's output, as rendered by the SPA "Output" surface. */
export interface WorkerRunOutput {
  jobId: string;
  workerId: string;
  workerName: string;
  sessionId: string | null;
  completedAt: string | null;
  stopReason: string | null;
  stopDetail: string | null;
  stepsCompleted: number | null;
  actionsEmitted: number | null;
  /** Full assistant reasoning/plan markdown (may be truncated by the
   *  gateway for very large runs). `null` if the bundle file is absent. */
  assistantOutput: string | null;
  /** Parsed `result.json` — pointers + section presence + evidence. */
  result: Record<string, unknown> | null;
  /** Parsed `actions.json` — the proposed actions + counts summary. */
  actions: Record<string, unknown> | null;
  /** Convenience: the counts block lifted out of `actions`, if present. */
  actionCounts: RunActionCounts | null;
}

/** Raw gateway shape (snake_case) — see `routes_worker_outputs.rs`. */
interface RawRunOutput {
  job_id?: string;
  worker_id?: string;
  worker_name?: string;
  session_id?: string | null;
  completed_at?: string | null;
  stop_reason?: string | null;
  stop_detail?: string | null;
  steps_completed?: number | null;
  actions_emitted?: number | null;
  assistant_output?: string | null;
  result?: Record<string, unknown> | null;
  actions?: Record<string, unknown> | null;
}

function numOrNull(v: unknown): number | null {
  return typeof v === "number" && Number.isFinite(v) ? v : null;
}

function liftActionCounts(
  actions: Record<string, unknown> | null,
): RunActionCounts | null {
  const counts = actions?.["counts"];
  if (!counts || typeof counts !== "object") return null;
  const c = counts as Record<string, unknown>;
  return {
    total: numOrNull(c["total"]) ?? undefined,
    pendingApproval: numOrNull(c["pending_approval"]) ?? undefined,
    pendingAutonomous: numOrNull(c["pending_autonomous"]) ?? undefined,
    blocked: numOrNull(c["blocked"]) ?? undefined,
  };
}

/** Map the raw gateway payload into the camelCase SPA shape. */
export function normalizeRunOutput(
  raw: RawRunOutput,
  workerId: string,
  jobId: string,
): WorkerRunOutput {
  const actions = raw.actions ?? null;
  return {
    jobId: raw.job_id ?? jobId,
    workerId: raw.worker_id ?? workerId,
    workerName: raw.worker_name ?? "",
    sessionId: raw.session_id ?? null,
    completedAt: raw.completed_at ?? null,
    stopReason: raw.stop_reason ?? null,
    stopDetail: raw.stop_detail ?? null,
    stepsCompleted: numOrNull(raw.steps_completed),
    actionsEmitted: numOrNull(raw.actions_emitted),
    assistantOutput: raw.assistant_output ?? null,
    result: raw.result ?? null,
    actions,
    actionCounts: liftActionCounts(actions),
  };
}

/**
 * Fetch one run's output bundle. Returns `null` (honest empty state)
 * when there is no real session, when the gateway is unreachable, or
 * when the run is missing / not the caller's (gateway 404). Any other
 * error propagates so the page can render an error state.
 */
export async function fetchWorkerRunOutput(
  workerId: string,
  jobId: string,
): Promise<WorkerRunOutput | null> {
  // Real output only — never fabricate for anonymous/demo visitors.
  if (!hasRealSession() && !isDemoLive()) {
    return null;
  }
  const path = `/api/v1/workers/${encodeURIComponent(
    workerId,
  )}/runs/${encodeURIComponent(jobId)}/output`;
  try {
    const raw = await apiGet<RawRunOutput>(path);
    return normalizeRunOutput(raw, workerId, jobId);
  } catch (err) {
    if (isMissingOrUnreachable(err)) {
      return null;
    }
    throw err;
  }
}
