"use client";

import { useQuery } from "@tanstack/react-query";
import {
  fetchSkillIr,
  SkillIrNotFoundError,
  type FetchSkillIrResult,
} from "@/lib/api/skill-ir";

/**
 * Cache key for the per-skill IR — `["skill", id, "ir", version?]`.
 * Distinct from the worker-skills cache (`["worker", id, "skills"]`)
 * so the canvas viewer doesn't invalidate the Skills tab and vice
 * versa.
 */
export const skillIrQueryKey = (skillId: string, version?: string) =>
  ["skill", skillId, "ir", version ?? "latest"] as const;

/**
 * Fetch the Skill IR for a skill, optionally a specific version.
 * Falls back to the bundled sample dataset in dev when the gateway
 * 404s — see `fetchSkillIr`. 404 is non-retried so the canvas can
 * render its empty-state CTA promptly.
 */
export function useSkillIr(skillId: string, version?: string) {
  return useQuery<FetchSkillIrResult, Error>({
    queryKey: skillIrQueryKey(skillId, version),
    queryFn: () => fetchSkillIr(skillId, version),
    enabled: skillId.length > 0,
    retry: (failureCount, err) => {
      if (err instanceof SkillIrNotFoundError) return false;
      return failureCount < 2;
    },
  });
}
