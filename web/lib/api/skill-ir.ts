/**
 * Skill-IR API client.
 *
 * Backs the L4 read-only canvas viewer. Mirrors the mock-fallback
 * contract used by `worker-skills.ts` / `worker-overview.ts`:
 *
 *   - `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=1` → mock immediately.
 *   - 404 / network error in dev → bundled sample IR.
 *   - Strict production (`NODE_ENV === 'production'` AND
 *     `NEXT_PUBLIC_ENV === 'production'`) propagates all errors.
 *
 * Gateway endpoints arrive via L4-gw:
 *   - `GET /api/v1/skills/<skillId>/ir?version=<v>`
 *   - `GET /api/v1/skills/<skillId>/ir/versions`
 *
 * Until L4-gw merges, both calls satisfy from `sample-skill-ir.ts`.
 * Missing skill (no sample + no gateway row) → `SkillIrNotFoundError`,
 * which the canvas page renders as the "teach this skill first"
 * empty state.
 */

import { apiGet } from "@/lib/api/client";
import { ApiError } from "@/lib/api/types";
import { findSampleSkillIr } from "@/lib/sample-skill-ir";
import type { SkillGraph } from "@/lib/types/skill-ir";

export type SkillIrSource = "gateway" | "mock-offline" | "mock-bypass";

export interface SkillIrVersionRef {
  version: string;
  irHash: string;
  createdAt?: string;
}

export interface FetchSkillIrResult {
  graph: SkillGraph;
  version: string;
  irHash?: string;
  source: SkillIrSource;
}

export class SkillIrNotFoundError extends Error {
  readonly skillId: string;
  readonly version?: string;
  constructor(skillId: string, version?: string) {
    super(
      version
        ? `Skill IR for '${skillId}' version '${version}' not found`
        : `Skill IR for '${skillId}' not found`,
    );
    this.name = "SkillIrNotFoundError";
    this.skillId = skillId;
    this.version = version;
  }
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
    process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS === "1"
  );
}

function isMissingOrUnreachable(err: unknown): boolean {
  if (err instanceof ApiError) return err.status === 404;
  return err instanceof TypeError;
}

function mockResult(
  skillId: string,
  version: string | undefined,
  source: SkillIrSource,
): FetchSkillIrResult {
  const sample = findSampleSkillIr(skillId, version);
  if (!sample) throw new SkillIrNotFoundError(skillId, version);
  return { ...sample, source };
}

/**
 * Gateway-shape envelope for `GET /skills/<id>/ir` (L4-gw). Until that
 * brief merges, the shape is pinned here so the client compiles
 * against the agreed contract; L4-gw must mirror it field-for-field.
 */
interface GatewaySkillIrPayload {
  graph: SkillGraph;
  version: string;
  irHash?: string;
}

export async function fetchSkillIr(
  skillId: string,
  version?: string,
): Promise<FetchSkillIrResult> {
  if (isAuthBypassed()) return mockResult(skillId, version, "mock-bypass");

  const path =
    `/api/v1/skills/${encodeURIComponent(skillId)}/ir` +
    (version ? `?version=${encodeURIComponent(version)}` : "");

  try {
    const payload = await apiGet<GatewaySkillIrPayload>(path);
    return {
      graph: payload.graph,
      version: payload.version,
      irHash: payload.irHash,
      source: "gateway",
    };
  } catch (err) {
    if (!isStrictProduction() && isMissingOrUnreachable(err)) {
      return mockResult(skillId, version, "mock-offline");
    }
    if (err instanceof ApiError && err.status === 404) {
      throw new SkillIrNotFoundError(skillId, version);
    }
    throw err;
  }
}
