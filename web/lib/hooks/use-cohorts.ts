"use client";

import { useQuery } from "@tanstack/react-query";
import {
  fetchCohortDetail,
  fetchCohorts,
  type FetchCohortDetailResult,
  type FetchCohortsResult,
} from "@/lib/api/cohorts";

const COHORTS_KEY = ["cohorts"] as const;
const COHORT_DETAIL_KEY = ["cohort"] as const;

/** Fleet-wide cohort summary list. Server-cached + revalidated on focus. */
export function useCohorts() {
  return useQuery<FetchCohortsResult>({
    queryKey: COHORTS_KEY,
    queryFn: fetchCohorts,
  });
}

/**
 * One cohort's full breakdown. Disabled while `cohortId` is null so the
 * page can hold the drawer closed without firing a query.
 */
export function useCohortDetail(cohortId: string | null) {
  return useQuery<FetchCohortDetailResult>({
    queryKey: [...COHORT_DETAIL_KEY, cohortId] as const,
    queryFn: () => {
      if (cohortId === null) {
        throw new Error("cohort id required");
      }
      return fetchCohortDetail(cohortId);
    },
    enabled: cohortId !== null,
  });
}
