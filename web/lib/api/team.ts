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
 * - `NEXT_PUBLIC_GATEWAY_AUTH_BYPASS=1` → mock immediately, no
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
import type { Worker } from "@/lib/types/worker";

export type TeamDataSource = "gateway" | "mock-offline" | "mock-bypass";

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
    process.env.NEXT_PUBLIC_GATEWAY_AUTH_BYPASS === "1"
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
    const workers = await apiGet<Worker[]>("/api/v1/team");
    return { workers, source: "gateway" };
  } catch (err) {
    if (!isStrictProduction() && isMissingOrUnreachable(err)) {
      return { workers: SAMPLE_TEAM, source: "mock-offline" };
    }
    throw err;
  }
}
