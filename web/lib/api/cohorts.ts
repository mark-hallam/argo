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

import { apiGet } from "@/lib/api/client";
import { ApiError } from "@/lib/api/types";
import { SAMPLE_COHORT_DETAILS, SAMPLE_COHORT_SUMMARIES } from "@/lib/sample-cohorts";

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

export interface CohortDetail extends CohortSummary {
  members: CohortMember[];
  skills: SkillCoverage[];
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
    process.env.NEXT_PUBLIC_GATEWAY_AUTH_BYPASS === "1"
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
      return { cohorts: SAMPLE_COHORT_SUMMARIES, source: "mock-offline" };
    }
    throw err;
  }
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
      return {
        detail: SAMPLE_COHORT_DETAILS[cohortId] ?? null,
        source: "mock-offline",
      };
    }
    throw err;
  }
}
