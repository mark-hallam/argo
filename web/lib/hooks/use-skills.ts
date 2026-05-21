"use client";

import { useQuery } from "@tanstack/react-query";
import {
  fetchLearnedThisMonth,
  fetchSkills,
  fetchSkillUsage,
  type FetchLearnedThisMonthResult,
  type FetchSkillsResult,
  type FetchSkillUsageResult,
} from "@/lib/api/skills";

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

/**
 * Workers-using-this-skill bundle for the skill-detail page. Returns
 * the canonical trust grade, anonymized author label, current-tenant
 * workers (with `learned_at`), and the cross-tenant adoption count.
 */
export function useSkillUsage(skillName: string | undefined) {
  return useQuery<FetchSkillUsageResult>({
    queryKey: ["skill-usage", skillName ?? ""],
    queryFn: () => fetchSkillUsage(skillName as string),
    enabled: typeof skillName === "string" && skillName.length > 0,
  });
}

/**
 * Skill ids the workers in the current workspace tenant have learned
 * since the first-of-month UTC cutoff. Drives the marketplace's
 * "Skills my workers learned this month" filter chip.
 */
export function useSkillsLearnedThisMonth() {
  return useQuery<FetchLearnedThisMonthResult>({
    queryKey: ["skills-learned-this-month"],
    queryFn: fetchLearnedThisMonth,
  });
}
