"use client";

import { useCallback } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  disableSkill as apiDisableSkill,
  discardLearned as apiDiscardLearned,
  enableSkill as apiEnableSkill,
  fetchSkills,
  promoteLearned as apiPromoteLearned,
  WorkerSkillsNotFoundError,
  type FetchWorkerSkillsResult,
  type LibrarySkill,
  type SkillMutationResult,
  type WorkerSkills,
} from "@/lib/api/worker-skills";

/**
 * Skills-tab data + mutation hooks.
 *
 * `useWorkerSkills(workerId)` returns the three-section payload with
 * the same query-cache contract as `useWorkerOverview` (single key,
 * not retried on 404). Sibling mutation hooks call the gateway (or
 * the mock fallback) and apply an optimistic `setQueryData` so the
 * UI reflects the change before the network round-trip completes —
 * with rollback on error.
 *
 * The optimistic transforms are local to the hook so the UI cards
 * don't have to know about cache shape.
 */
export const workerSkillsQueryKey = (workerId: string) =>
  ["worker", workerId, "skills"] as const;

export function useWorkerSkills(workerId: string) {
  return useQuery<FetchWorkerSkillsResult, Error>({
    queryKey: workerSkillsQueryKey(workerId),
    queryFn: () => fetchSkills(workerId),
    retry: (failureCount, err) => {
      if (err instanceof WorkerSkillsNotFoundError) return false;
      return failureCount < 2;
    },
  });
}

type Updater = (skills: WorkerSkills) => WorkerSkills;

function applyOptimistic(
  client: ReturnType<typeof useQueryClient>,
  workerId: string,
  updater: Updater,
): FetchWorkerSkillsResult | undefined {
  const key = workerSkillsQueryKey(workerId);
  const previous = client.getQueryData<FetchWorkerSkillsResult>(key);
  if (!previous) return undefined;
  client.setQueryData<FetchWorkerSkillsResult>(key, {
    ...previous,
    skills: updater(previous.skills),
  });
  return previous;
}

function rollback(
  client: ReturnType<typeof useQueryClient>,
  workerId: string,
  previous: FetchWorkerSkillsResult | undefined,
) {
  if (!previous) return;
  client.setQueryData(workerSkillsQueryKey(workerId), previous);
}

export function useDisableSkill(workerId: string) {
  const client = useQueryClient();
  return useMutation<SkillMutationResult, Error, string, FetchWorkerSkillsResult | undefined>({
    mutationFn: (skillId) => apiDisableSkill(workerId, skillId),
    onMutate: (skillId) =>
      applyOptimistic(client, workerId, (skills) => ({
        ...skills,
        active: skills.active.filter((s) => s.id !== skillId),
      })),
    onError: (_err, _skillId, previous) => rollback(client, workerId, previous),
  });
}

export function useTryLibrarySkill(workerId: string) {
  const client = useQueryClient();
  return useMutation<SkillMutationResult, Error, LibrarySkill, FetchWorkerSkillsResult | undefined>({
    mutationFn: (skill) => apiEnableSkill(workerId, skill.id),
    onMutate: (skill) =>
      applyOptimistic(client, workerId, (skills) => ({
        ...skills,
        library: skills.library.filter((s) => s.id !== skill.id),
        active: [
          ...skills.active,
          {
            id: skill.id,
            name: skill.name,
            version: skill.version,
            trustScore: skill.trustScore,
            usageCount7d: 0,
          },
        ],
      })),
    onError: (_err, _skill, previous) => rollback(client, workerId, previous),
  });
}

export function usePromoteLearned(workerId: string) {
  const client = useQueryClient();
  return useMutation<SkillMutationResult, Error, string, FetchWorkerSkillsResult | undefined>({
    mutationFn: (patternId) => apiPromoteLearned(workerId, patternId),
    onMutate: (patternId) =>
      applyOptimistic(client, workerId, (skills) => {
        const learned = skills.learned.find((p) => p.id === patternId);
        if (!learned) return skills;
        return {
          ...skills,
          learned: skills.learned.filter((p) => p.id !== patternId),
          library: [
            ...skills.library,
            {
              id: learned.id,
              name: learned.label,
              version: "v0.1.0",
              trustScore: 80,
              source: "workspace",
            },
          ],
        };
      }),
    onError: (_err, _patternId, previous) => rollback(client, workerId, previous),
  });
}

export function useDiscardLearned(workerId: string) {
  const client = useQueryClient();
  return useMutation<SkillMutationResult, Error, string, FetchWorkerSkillsResult | undefined>({
    mutationFn: (patternId) => apiDiscardLearned(workerId, patternId),
    onMutate: (patternId) =>
      applyOptimistic(client, workerId, (skills) => ({
        ...skills,
        learned: skills.learned.filter((p) => p.id !== patternId),
      })),
    onError: (_err, _patternId, previous) => rollback(client, workerId, previous),
  });
}

/** Convenience bundle for the page-level component. */
export function useSkillsActions(workerId: string) {
  const disable = useDisableSkill(workerId);
  const tryLibrary = useTryLibrarySkill(workerId);
  const promote = usePromoteLearned(workerId);
  const discard = useDiscardLearned(workerId);

  const onDisable = useCallback((id: string) => disable.mutate(id), [disable]);
  const onTry = useCallback((skill: LibrarySkill) => tryLibrary.mutate(skill), [tryLibrary]);
  const onPromote = useCallback((id: string) => promote.mutate(id), [promote]);
  const onDiscard = useCallback((id: string) => discard.mutate(id), [discard]);

  return { onDisable, onTry, onPromote, onDiscard };
}
