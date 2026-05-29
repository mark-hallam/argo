"use client";

import { useQuery } from "@tanstack/react-query";
import { fetchTeam, type FetchTeamResult } from "@/lib/api/team";

const TEAM_KEY = ["team"] as const;

/**
 * The team roster. Server-cached + revalidated on window focus.
 *
 * `staleTime: 0` + `refetchOnMount: "always"` defeat a SSR/client
 * hydration race observed in dev: when the server-rendered HTML
 * hydrated against a stale pathname, React Query would retain its
 * initial pending state across the boundary without re-running
 * `queryFn`, leaving /team stuck on a perpetual skeleton. Forcing
 * an always-refetch on mount makes the query authoritative on the
 * client and the loading chip flips within 2s of paint.
 *
 * `retry: 1` caps the transient-failure spin: `fetchTeam` already
 * has rich fallback paths (demo SAMPLE_TEAM, unreachable empty),
 * so one quick retry is enough to recover from a flapping gateway
 * without burning seconds on exponential backoff.
 */
export function useTeam() {
  return useQuery<FetchTeamResult>({
    queryKey: TEAM_KEY,
    queryFn: fetchTeam,
    staleTime: 0,
    refetchOnMount: "always",
    retry: 1,
  });
}
