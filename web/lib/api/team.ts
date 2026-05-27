/**
 * Team API client.
 *
 * Routes through the typed `lib/api/client.ts` wrapper. The gateway's
 * `/api/v1/workers` is the canonical collection endpoint. The legacy
 * `/api/v1/team` gateway path remains as a deprecated back-compat
 * alias, but the SPA should use the Workers noun that matches the
 * product navigation and singleton `/api/v1/workers/<id>` routes.
 *
 * Acceptance contract (mirrors `lib/api/approvals.ts`):
 * - `fetchTeam` returns `Worker[]`.
 * - 404 from the gateway → mock fallback (`dataSource = "mock-offline"`).
 * - `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=1` → mock immediately, no
 *   network call.
 * - Production builds (`NODE_ENV === 'production'` AND
 *   `NEXT_PUBLIC_ENV === 'production'`): no mock fallback. 404 and
 *   any other ApiError propagate so they surface as errors.
 * - NetworkError (gateway unreachable) → mock fallback in dev,
 *   propagates in prod.
 */

import { apiGet } from "@/lib/api/client";
import { ApiError } from "@/lib/api/types";
import { listDemoWorkers } from "@/lib/api/demo-roster-store";
import { SAMPLE_TEAM } from "@/lib/sample-team";
import { isDemoMode } from "@/lib/skycot/demo-mode";
import type { Worker } from "@/lib/types/worker";

export type TeamDataSource =
  | "gateway"
  | "mock-offline"
  | "mock-bypass"
  | "unreachable";

export interface FetchTeamResult {
  workers: Worker[];
  source: TeamDataSource;
}

function isStrictProduction(): boolean {
  return (
    process.env.NODE_ENV === "production" &&
    process.env.NEXT_PUBLIC_ENV === "production"
  );
}

function isAuthBypassed(): boolean {
  return (
    process.env.NODE_ENV !== "production" &&
    process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS === "1"
  );
}

function isMissingOrUnreachable(err: unknown): boolean {
  if (err instanceof ApiError) return err.status === 404;
  return err instanceof TypeError;
}

/**
 * Defence-in-depth fallback for `verifyWorkerReadable` in
 * `web/lib/onboarding/use-first-hire.ts`. If the freshly returned
 * `worker_id` 404s on `/api/v1/workers/{id}/overview` (e.g. a future
 * gateway regression of the "ghost worker" bug), refetching the team
 * roster and matching by name lets the wizard recover instead of
 * surfacing `FIRST_HIRE_VERIFY_FAILED_MESSAGE` to the operator.
 *
 * Match is case- and whitespace-insensitive. Returns `null` when the
 * roster fetch fails OR no worker matches — the caller treats `null` as
 * "fallback unavailable, surface the original error".
 */
export async function findWorkerByName(name: string): Promise<Worker | null> {
  const target = name.trim().toLowerCase();
  if (target.length === 0) {
    return null;
  }
  try {
    const result = await fetchTeam();
    const hit = result.workers.find((w) => w.name.trim().toLowerCase() === target);
    return hit ?? null;
  } catch {
    return null;
  }
}

export async function fetchTeam(): Promise<FetchTeamResult> {
  if (isAuthBypassed()) {
    // Splice in any session-local hires from the demo roster so the
    // /team deck grows when the Hire wizard commits in bypass mode.
    // When the roster is empty we return the bundled array verbatim
    // (referential identity) so consumers holding `=== SAMPLE_TEAM`
    // checks stay stable.
    const hires = listDemoWorkers();
    if (hires.length === 0) {
      return { workers: SAMPLE_TEAM, source: "mock-bypass" };
    }
    return { workers: [...SAMPLE_TEAM, ...hires], source: "mock-bypass" };
  }

  try {
    // Gateway returns { workers: Worker[], total_count: number } envelope.
    // Fall back to treating a bare-array response as Worker[] for any older
    // gateway build that hasn't been redeployed. Guard against a malformed
    // envelope (missing `workers` field) so a server bug doesn't silently
    // collapse the roster to empty.
    const response = await apiGet<{ workers: Worker[]; total_count?: number } | Worker[]>(
      "/api/v1/workers",
    );
    if (Array.isArray(response)) {
      return { workers: response, source: "gateway" };
    }
    if (response && Array.isArray(response.workers)) {
      return { workers: response.workers, source: "gateway" };
    }
    throw new ApiError(
      200,
      "Malformed team envelope: missing workers array",
      "malformed_envelope",
      response,
    );
  } catch (err) {
    if (!isStrictProduction() && isMissingOrUnreachable(err)) {
      // Real signed-in users get an empty roster + the "hire your first
      // teammate" empty state when the gateway endpoint isn't live yet.
      // The mock fallback is reserved for the v1.0 recorded demo.
      if (isDemoMode()) {
        return { workers: SAMPLE_TEAM, source: "mock-offline" };
      }
      // Source `unreachable` (not `gateway`) so the diagnostic chip on
      // /team and any future operator surface can tell "gateway said
      // zero" apart from "gateway never answered".
      return { workers: [], source: "unreachable" };
    }
    throw err;
  }
}
