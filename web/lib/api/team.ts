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
 * - Demo mode (`NEXT_PUBLIC_SKYCOT_DEMO_MODE=1`): the catch path is
 *   total — ANY thrown error or a >5s hang resolves to SAMPLE_TEAM.
 *   The /team skeleton must never be perpetual during the demo MP4.
 */

import { apiGet } from "@/lib/api/client";
import { ApiError } from "@/lib/api/types";
import { listDemoWorkers } from "@/lib/api/demo-roster-store";
import { SAMPLE_TEAM } from "@/lib/sample-team";
import {
  publishDataSource,
  type DataSource,
} from "@/lib/skycot/data-source-context";
import { isDemoLive, isDemoMode } from "@/lib/skycot/demo-mode";
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

/** Hard ceiling on demo-mode gateway wait before the SAMPLE_TEAM fallback wins the race. */
export const DEMO_GATEWAY_TIMEOUT_MS = 5000;

class GatewayTimeoutError extends Error {
  constructor(ms: number) {
    super(`gateway did not respond within ${ms}ms`);
    this.name = "GatewayTimeoutError";
  }
}

/**
 * Thrown when `/api/v1/workers` answers with anything other than the
 * canonical `{ workers: Worker[], total_count: number }` envelope —
 * including a bare `Worker[]` array (the legacy shape) or an object
 * missing the `workers` field. Tightening to envelope-only stops a
 * legacy gateway / a buggy redeploy from silently shipping a
 * non-canonical payload through the SPA. Extends `ApiError` so existing
 * `instanceof ApiError` catch paths (e.g. the demo-mode catch-all)
 * keep working unchanged.
 */
export class InvalidTeamEnvelope extends ApiError {
  constructor(body: unknown) {
    super(
      200,
      "Invalid team envelope: expected { workers: Worker[], total_count: number }",
      "invalid_team_envelope",
      body,
    );
    this.name = "InvalidTeamEnvelope";
  }
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

function toBannerSource(source: TeamDataSource): DataSource {
  switch (source) {
    case "mock-bypass":
      return "mock-bypass";
    case "mock-offline":
      return "mock-offline";
    case "gateway":
    case "unreachable":
      // `unreachable` is the post-2026-05 empty-state path — the real
      // gateway answered (or didn't) and the SPA shows an empty roster,
      // not seed fixtures. From the banner's perspective that's "live".
      return "live";
  }
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

function withTimeout<T>(p: Promise<T>, ms: number): Promise<T> {
  return new Promise((resolve, reject) => {
    const timer = setTimeout(() => reject(new GatewayTimeoutError(ms)), ms);
    p.then(
      (v) => { clearTimeout(timer); resolve(v); },
      (e) => { clearTimeout(timer); reject(e); },
    );
  });
}

export async function fetchTeam(): Promise<FetchTeamResult> {
  const result = await fetchTeamInner();
  publishDataSource(toBannerSource(result.source));
  return result;
}

async function fetchTeamInner(): Promise<FetchTeamResult> {
  // Demo-LIVE recordings (auth-bypass + DEMO_LIVE) must read the real
  // gateway roster — NOT the bundled SAMPLE_TEAM mock — so the /team
  // hero deck reflects the seeded tenant truthfully (NO THEATER). Only
  // plain auth-bypass (no demo-live) short-circuits to the bundled deck.
  // This matches the canonical gate used by audit/approvals/workers/etc.
  if (isAuthBypassed() && !isDemoLive()) {
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

  const demoMode = isDemoMode();

  try {
    // Gateway returns the canonical { workers: Worker[], total_count: number }
    // envelope. A bare-array response is NO LONGER tolerated: a legacy gateway
    // or a buggy redeploy shipping a bare `Worker[]` is a contract violation,
    // surfaced as `InvalidTeamEnvelope` rather than silently accepted. A
    // malformed object (missing `workers` array) is rejected the same way so a
    // server bug doesn't silently collapse the roster to empty.
    const gatewayCall = apiGet<{ workers: Worker[]; total_count?: number }>(
      "/api/v1/workers",
    );
    const response = demoMode
      ? await withTimeout(gatewayCall, DEMO_GATEWAY_TIMEOUT_MS)
      : await gatewayCall;
    if (response && !Array.isArray(response) && Array.isArray(response.workers)) {
      return { workers: response.workers, source: "gateway" };
    }
    throw new InvalidTeamEnvelope(response);
  } catch (err) {
    // Demo mode is the recorded-demo path; the camera must never see a
    // perpetual skeleton. Any error (404, 5xx, network TypeError,
    // GatewayTimeoutError, malformed envelope) resolves to SAMPLE_TEAM
    // so React Query settles instead of stalling on an unhandled
    // rejection. Strict production is still excluded — real buyers see
    // real errors, not seeded mocks.
    if (demoMode && !isDemoLive() && !isStrictProduction()) {
      return { workers: SAMPLE_TEAM, source: "mock-offline" };
    }
    if (!isStrictProduction() && isMissingOrUnreachable(err)) {
      // Real signed-in users get an empty roster + the "hire your first
      // teammate" empty state when the gateway endpoint isn't live yet.
      // The mock fallback is reserved for the v1.0 recorded demo.
      // Source `unreachable` (not `gateway`) so the diagnostic chip on
      // /team and any future operator surface can tell "gateway said
      // zero" apart from "gateway never answered".
      return { workers: [], source: "unreachable" };
    }
    throw err;
  }
}
