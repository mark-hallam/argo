/**
 * Per-worker "last Run-now response" summary, persisted to
 * `sessionStorage` so the worker-overview surfaces survive the
 * post-run remount.
 *
 * The problem this fixes (observed live, 2026-06-12 DEC-044 pass): a
 * real Run-now completes, but the response (`job_id` / `held_tools` /
 * `probation`) lives only in `WorkerOverviewContent` state. The page
 * remounts right after the run, so:
 *   - `RunOutputCard` (a sibling rendered by the route page, NOT a
 *     child of `WorkerOverviewContent`) never receives the `jobId` and
 *     shows "No runs yet" immediately after a successful run;
 *   - the `ProbationApprovalCard` never re-renders even though the
 *     toast told the operator to "approve the held tools below".
 *
 * Both cards live in different React subtrees, so a shared parent
 * `useState` can't bridge them. A per-worker `sessionStorage` entry
 * does — `WorkerOverviewContent` writes it on every run response, and
 * each card rehydrates from it on mount. No backend change: the
 * `job_id` is already on the Run-now response envelope, and the
 * authenticated output route already serves the bundle for it.
 *
 * NO THEATER: a worker that has never been run has no entry, so the
 * honest "No runs yet" / no-approval-card empty states stay.
 */

/** The minimal slice of a Run-now response the overview cards rehydrate from. */
export interface LastRunSummary {
  /**
   * The pending-job id the run claimed — the key for
   * `GET /api/v1/workers/<id>/runs/<job_id>/output`. `null` on a run
   * that completed no job (the gateway returns an empty string there).
   */
  jobId: string | null;
  /** Tools the probation window held — empty when the run converged. */
  heldTools: string[];
  /** Whether the worker was inside its 24h probation window. */
  probation: boolean;
  /** Epoch milliseconds when the response came back (freshness signal). */
  at: number;
}

const KEY_PREFIX = "skycot:last-run:";

function storageKey(workerId: string): string {
  return `${KEY_PREFIX}${workerId}`;
}

/**
 * Persist the last-run summary for `workerId`. Storage failures
 * (private-mode quota, disabled storage) are swallowed — the live
 * in-memory render still works; only the post-remount rehydration is
 * lost, which degrades gracefully to today's behavior.
 */
export function saveLastRunSummary(
  workerId: string,
  summary: LastRunSummary,
): void {
  try {
    sessionStorage.setItem(storageKey(workerId), JSON.stringify(summary));
  } catch {
    /* storage unavailable — non-fatal */
  }
}

/**
 * Load + validate the last-run summary for `workerId`. Returns `null`
 * when nothing is stored or the stored value is malformed, so callers
 * fall through to the honest empty state.
 */
export function loadLastRunSummary(workerId: string): LastRunSummary | null {
  let raw: string | null = null;
  try {
    raw = sessionStorage.getItem(storageKey(workerId));
  } catch {
    return null;
  }
  if (!raw) return null;
  try {
    const parsed = JSON.parse(raw) as unknown;
    if (typeof parsed !== "object" || parsed === null) return null;
    const p = parsed as Record<string, unknown>;
    const jobId = typeof p.jobId === "string" && p.jobId ? p.jobId : null;
    const heldTools = Array.isArray(p.heldTools)
      ? p.heldTools.filter((t): t is string => typeof t === "string")
      : [];
    const probation = p.probation === true;
    const at = typeof p.at === "number" && Number.isFinite(p.at) ? p.at : 0;
    return { jobId, heldTools, probation, at };
  } catch {
    return null;
  }
}

/** Drop the stored summary for `workerId` (e.g. an explicit reset). */
export function clearLastRunSummary(workerId: string): void {
  try {
    sessionStorage.removeItem(storageKey(workerId));
  } catch {
    /* storage unavailable — non-fatal */
  }
}
