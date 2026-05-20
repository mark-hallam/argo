"use client";

import { useQuery } from "@tanstack/react-query";
import { fetchSkills, type FetchSkillsResult } from "@/lib/api/skills";

const SKILLS_KEY = ["skills"] as const;

/**
 * Marketplace skill catalog. Pulls the layered skill registry via the
 * gateway and revalidates on focus. Empty/offline gateway falls through
 * to the seed catalog in `lib/api/skills.ts` so the /skills landing
 * still renders during dev.
 */
export function useSkills() {
  return useQuery<FetchSkillsResult>({
    queryKey: SKILLS_KEY,
    queryFn: fetchSkills,
  });
}
