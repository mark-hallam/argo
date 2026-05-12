/**
 * Worker-skills API client.
 *
 * Backs the three-section Skills tab — Active / Library / Learned —
 * with a mock-fallback contract that mirrors `worker-activity.ts` and
 * `worker-overview.ts`:
 *
 *   - `NEXT_PUBLIC_GATEWAY_AUTH_BYPASS=1` → mock immediately.
 *   - 404 / network error in dev → bundled mock.
 *   - Strict production (`NODE_ENV === 'production'` AND
 *     `NEXT_PUBLIC_ENV === 'production'`) propagates all errors.
 *
 * Gateway endpoints arrive via the sibling Track-F briefs
 * `mission-skill-store-available-for-worker` (Library) and
 * `mission-skill-publish-verified-ingest` (Promote). Until those land,
 * every call is satisfied by `lib/sample-worker-skills.ts` and
 * mutations are no-ops on the server but produce a client-side
 * `move-to-active` / `remove-from-learned` update returned to the
 * hook so the UI can optimistically reflect the change.
 */

import { apiDelete, apiGet, apiPost } from "@/lib/api/client";
import { ApiError } from "@/lib/api/types";
import { findSampleWorkerSkills } from "@/lib/sample-worker-skills";

export type SkillSource = "workspace" | "shared" | "marketplace";

export const SKILL_SOURCE_LABELS: Record<SkillSource, string> = {
  workspace: "Workspace",
  shared: "Shared",
  marketplace: "Marketplace",
};

export interface ActiveSkill {
  id: string;
  name: string;
  version: string;
  /** Trust score 0..100 from `mission-skill-quality-from-audit`. */
  trustScore: number;
  /** 7-day usage count for the trailing-window pill. */
  usageCount7d: number;
}

export interface LibrarySkill {
  id: string;
  name: string;
  version: string;
  trustScore: number;
  source: SkillSource;
}

export interface LearnedPattern {
  id: string;
  /** Short kebab-style label, e.g. `auto-decline-incomplete-applications`. */
  label: string;
  successCount: number;
  failCount: number;
  /** Three example outputs surfaced in the Promote dialog. */
  sampleOutputs: string[];
}

export interface WorkerSkills {
  active: ActiveSkill[];
  library: LibrarySkill[];
  learned: LearnedPattern[];
}

export type WorkerSkillsSource = "gateway" | "mock-offline" | "mock-bypass";

export interface FetchWorkerSkillsResult {
  skills: WorkerSkills;
  source: WorkerSkillsSource;
}

export class WorkerSkillsNotFoundError extends Error {
  readonly workerId: string;
  constructor(workerId: string) {
    super(`Worker skills for '${workerId}' not found`);
    this.name = "WorkerSkillsNotFoundError";
    this.workerId = workerId;
  }
}

/** Trust-score bucket → tone used by the badge. */
export type TrustTone = "success" | "warning" | "destructive";

export function trustTone(trustScore: number): TrustTone {
  if (trustScore >= 90) return "success";
  if (trustScore >= 70) return "warning";
  return "destructive";
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

function mockResult(workerId: string, source: WorkerSkillsSource): FetchWorkerSkillsResult {
  const skills = findSampleWorkerSkills(workerId);
  if (!skills) throw new WorkerSkillsNotFoundError(workerId);
  return { skills, source };
}

export async function fetchSkills(workerId: string): Promise<FetchWorkerSkillsResult> {
  if (isAuthBypassed()) return mockResult(workerId, "mock-bypass");

  try {
    const skills = await apiGet<WorkerSkills>(
      `/api/v1/workers/${encodeURIComponent(workerId)}/skills`,
    );
    return { skills, source: "gateway" };
  } catch (err) {
    if (!isStrictProduction() && isMissingOrUnreachable(err)) {
      return mockResult(workerId, "mock-offline");
    }
    if (err instanceof ApiError && err.status === 404) {
      throw new WorkerSkillsNotFoundError(workerId);
    }
    throw err;
  }
}

/** Mutation result returned to the hook for optimistic cache updates. */
export interface SkillMutationResult {
  /** Skill / pattern id the mutation acted on. */
  id: string;
  /** Was the request handled by the gateway or the mock fallback? */
  source: WorkerSkillsSource;
}

async function mutate(
  workerId: string,
  path: string,
  body: unknown,
  method: "post" | "delete",
): Promise<WorkerSkillsSource> {
  if (isAuthBypassed()) return "mock-bypass";
  try {
    if (method === "post") {
      await apiPost<unknown>(`/api/v1/workers/${encodeURIComponent(workerId)}${path}`, body);
    } else {
      await apiDelete<unknown>(`/api/v1/workers/${encodeURIComponent(workerId)}${path}`);
    }
    return "gateway";
  } catch (err) {
    if (!isStrictProduction() && isMissingOrUnreachable(err)) {
      return "mock-offline";
    }
    throw err;
  }
}

export async function disableSkill(
  workerId: string,
  skillId: string,
): Promise<SkillMutationResult> {
  const source = await mutate(workerId, `/skills/${encodeURIComponent(skillId)}`, undefined, "delete");
  return { id: skillId, source };
}

export async function enableSkill(
  workerId: string,
  skillId: string,
): Promise<SkillMutationResult> {
  const source = await mutate(workerId, `/skills`, { skillId }, "post");
  return { id: skillId, source };
}

/** Library "Try it" action — alias of `enableSkill` for readability. */
export function tryLibrarySkill(workerId: string, skillId: string) {
  return enableSkill(workerId, skillId);
}

export async function promoteLearned(
  workerId: string,
  patternId: string,
): Promise<SkillMutationResult> {
  const source = await mutate(
    workerId,
    `/skills/learned/${encodeURIComponent(patternId)}/promote`,
    {},
    "post",
  );
  return { id: patternId, source };
}

export async function discardLearned(
  workerId: string,
  patternId: string,
): Promise<SkillMutationResult> {
  const source = await mutate(
    workerId,
    `/skills/learned/${encodeURIComponent(patternId)}`,
    undefined,
    "delete",
  );
  return { id: patternId, source };
}
