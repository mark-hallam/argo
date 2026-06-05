"use client";

import { useQuery } from "@tanstack/react-query";
import {
  fetchWorkerScorecard,
  fetchTeamScorecard,
  type WorkerScorecard,
  type TeamScorecard,
} from "@/lib/api/scorecard";

export const workerScorecardQueryKey = (workerId: string) =>
  ["worker", workerId, "scorecard"] as const;

export const teamScorecardQueryKey = () => ["team", "scorecard"] as const;

/**
 * Fetch one worker's goal-vs-actual scorecard. No mock fallback — the
 * endpoint is live, and a missing endpoint should surface as an error
 * rather than fabricated goals (NO THEATER).
 */
export function useWorkerScorecard(workerId: string) {
  return useQuery<WorkerScorecard, Error>({
    queryKey: workerScorecardQueryKey(workerId),
    queryFn: () => fetchWorkerScorecard(workerId),
  });
}

/** Fetch the team "goals hit this week" rollup. */
export function useTeamScorecard() {
  return useQuery<TeamScorecard, Error>({
    queryKey: teamScorecardQueryKey(),
    queryFn: () => fetchTeamScorecard(),
  });
}
