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

/**
 * Fleet-wide cohort summary list. Server-cached.
 *
 * `staleTime` + `refetchOnWindowFocus: false` keep the query from
 * re-firing on every tab focus / re-mount during the cold-paint window;
 * each refetch invalidates the memoised `groups` array in
 * `web/app/(app)/cohorts/page.tsx`, which re-renders the dynamic-imported
 * magazine grid and compounds the hydration cost. The fleet roster does
 * not change minute-to-minute, so 60s of cache freshness is well within
 * tolerance.
 */
export function useCohorts() {
  return useQuery<FetchCohortsResult>({
    queryKey: COHORTS_KEY,
    queryFn: fetchCohorts,
    staleTime: 60_000,
    refetchOnWindowFocus: false,
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
