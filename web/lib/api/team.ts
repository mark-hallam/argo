/**
 * Team API client.
 *
 * Routes through the typed `lib/api/client.ts` wrapper. The gateway's
 * `/api/v1/team` endpoint is being authored under brief
 * `mission-gateway-team-list-endpoint`; until that lands, every call
 * here will see a 404 and fall back to the bundled `SAMPLE_TEAM`
 * dataset so the team-page demo keeps working in dev without a
 * running gateway.
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

export async function fetchTeam(): Promise<FetchTeamResult> {
  if (isAuthBypassed()) {
    return { workers: SAMPLE_TEAM, source: "mock-bypass" };
  }

  try {
    // Gateway returns { workers: Worker[], total_count: number } envelope.
    // Fall back to treating a bare-array response as Worker[] for any older
    // gateway build that hasn't been redeployed. Guard against a malformed
    // envelope (missing `workers` field) so a server bug doesn't silently
    // collapse the roster to empty.
    const response = await apiGet<{ workers: Worker[]; total_count?: number } | Worker[]>(
      "/api/v1/team",
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
