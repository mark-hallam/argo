/**
 * Cohorts API client (Stream `mission-skill-cohort-ui`).
 *
 * Mirrors the `lib/api/team.ts` contract: gateway-first, mock fallback
 * on 404 in dev, strict-prod propagates errors. Two endpoints:
 *
 * - `GET /api/v1/cohorts` → `{ cohorts: CohortSummary[], total_count }`
 * - `GET /api/v1/cohorts/<id>` → flattened `CohortDetail`
 *
 * The gateway returns the list inside a `{ cohorts, total_count }`
 * envelope (matches `routes_team_list` convention); this client
 * normalizes to `CohortSummary[]` so callers don't need to peek at
 * the envelope.
 */

import { apiGet, apiPost } from "@/lib/api/client";
import { ApiError } from "@/lib/api/types";
import { SAMPLE_COHORT_DETAILS, SAMPLE_COHORT_SUMMARIES } from "@/lib/sample-cohorts";
import { isDemoMode } from "@/lib/skycot/demo-mode";

export interface CohortSummary {
  cohort_id: string;
  vertical: string;
  trust_grade_band: string;
  trust_grade_min: number;
  trust_grade_max: number;
  member_count: number;
  skill_count: number;
}

export interface CohortMember {
  worker_id: string;
  trust_grade: number;
  skill_ids: string[];
}

export interface SkillCoverage {
  skill_name: string;
  version: string | null;
  member_coverage_pct: number;
  member_hits: number;
}

/**
 * One audit-record row attributed to a cohort member, surfaced as the
 * drawer's "recent activity" feed. Shape mirrors the gateway envelope
 * emitted by `routes_cohorts::recent_cohort_activity`: a flat row with
 * the canonical kind + a short `summary` string extracted from the
 * payload (`summary` → `endpoint` → `message` → `intent`).
 */
export interface CohortActivityRow {
  seq: number;
  kind: string;
  action_type: string | null;
  worker_id: string | null;
  principal_id: string | null;
  channel: string | null;
  outcome_status: string | null;
  created_at_ms: number;
  summary: string | null;
}

export interface CohortDetail extends CohortSummary {
  members: CohortMember[];
  skills: SkillCoverage[];
  recent_activity: CohortActivityRow[];
}

export type CohortDataSource = "gateway" | "mock-offline" | "mock-bypass";

export interface FetchCohortsResult {
  cohorts: CohortSummary[];
  source: CohortDataSource;
}

export interface FetchCohortDetailResult {
  detail: CohortDetail | null;
  source: CohortDataSource;
}

interface CohortListEnvelope {
  cohorts: CohortSummary[];
  total_count?: number;
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
    process.env.ARGO_GATEWAY_AUTH_BYPASS === "1"
  );
}

function isMissingOrUnreachable(err: unknown): boolean {
  if (err instanceof ApiError) return err.status === 404;
  return err instanceof TypeError;
}

export async function fetchCohorts(): Promise<FetchCohortsResult> {
  if (isAuthBypassed()) {
    return { cohorts: SAMPLE_COHORT_SUMMARIES, source: "mock-bypass" };
  }
  try {
    const envelope = await apiGet<CohortListEnvelope>("/api/v1/cohorts");
    return { cohorts: envelope.cohorts ?? [], source: "gateway" };
  } catch (err) {
    if (!isStrictProduction() && isMissingOrUnreachable(err)) {
      // Demo mode keeps the seeded SAMPLE_COHORT_SUMMARIES so the v1.0
      // recorded walkthrough still plays back. Real signed-in users on
      // a fresh workspace fall through to the empty state.
      if (isDemoMode()) {
        return { cohorts: SAMPLE_COHORT_SUMMARIES, source: "mock-offline" };
      }
      return { cohorts: [], source: "gateway" };
    }
    throw err;
  }
}

/**
 * Result envelope for [`bulkAssignSkill`]. Mirrors the gateway shape
 * exactly so the modal can surface partial successes (idempotent
 * re-application — `already_had > 0`) without erroring.
 */
export interface BulkAssignSkillResponse {
  /** Workers that did NOT already hold the skill — the new rows. */
  assigned: number;
  /** Workers that already held the skill — counted but no-op. */
  already_had: number;
  /** Per-worker errors; v1 always empty on a 2xx response. */
  errors: Array<{ worker_id: string; message: string }>;
}

/**
 * `POST /api/v1/cohorts/<id>/bulk-assign-skill` — assign one skill to
 * every selected worker in a cohort. Wraps the gateway endpoint with
 * the standard `apiPost`. Errors propagate as `ApiError` (caller
 * handles toast / banner). The endpoint validates tenant + cohort
 * membership server-side so the client only has to forward the
 * selection.
 */
export async function bulkAssignSkill(
  cohortId: string,
  workerIds: string[],
  skillId: string,
): Promise<BulkAssignSkillResponse> {
  const path = `/api/v1/cohorts/${encodeURIComponent(cohortId)}/bulk-assign-skill`;
  return apiPost<BulkAssignSkillResponse>(path, {
    worker_ids: workerIds,
    skill_id: skillId,
  });
}

export async function fetchCohortDetail(
  cohortId: string,
): Promise<FetchCohortDetailResult> {
  if (isAuthBypassed()) {
    return {
      detail: SAMPLE_COHORT_DETAILS[cohortId] ?? null,
      source: "mock-bypass",
    };
  }
  const path = `/api/v1/cohorts/${encodeURIComponent(cohortId)}`;
  try {
    const detail = await apiGet<CohortDetail>(path);
    return { detail, source: "gateway" };
  } catch (err) {
    if (!isStrictProduction() && isMissingOrUnreachable(err)) {
      if (isDemoMode()) {
        return {
          detail: SAMPLE_COHORT_DETAILS[cohortId] ?? null,
          source: "mock-offline",
        };
      }
      return { detail: null, source: "gateway" };
    }
    throw err;
  }
}
