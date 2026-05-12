"use client";

import { useQuery } from "@tanstack/react-query";
import { fetchTeam, type FetchTeamResult } from "@/lib/api/team";

const TEAM_KEY = ["team"] as const;

/** The team roster. Server-cached + revalidated on window focus. */
export function useTeam() {
  return useQuery<FetchTeamResult>({
    queryKey: TEAM_KEY,
    queryFn: fetchTeam,
  });
}
