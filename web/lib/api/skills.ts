/**
 * Skills marketplace API client.
 *
 * The gateway exposes the layered skill registry under `/api/v1/skills`
 * (`crates/argo-cli/src/handlers/gateway/routes_skills.rs`). Response
 * envelope: `{ risk, risk_defaulted, count, skills: LayeredSkillRow[] }`.
 *
 * Acceptance contract (mirrors `lib/api/team.ts` and `lib/api/cohorts.ts`):
 * - 404 from the gateway → fall back to a seed catalog in dev so the
 *   marketplace landing still renders.
 * - `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=1` → mock immediately, no network call.
 * - Strict prod (`NODE_ENV === 'production'` AND `NEXT_PUBLIC_ENV ===
 *   'production'`): no mock fallback; ApiError propagates.
 * - NetworkError (gateway unreachable) → mock in dev, propagate in prod.
 */

import { apiGet } from "@/lib/api/client";
import { ApiError } from "@/lib/api/types";
import { isDemoLive } from "@/lib/skycot/demo-mode";
import {
  SEED_SKILLS,
  MOCK_LEARNED_SKILL_IDS,
  SEED_AUTHOR_LABEL_BY_NAME,
} from "./skills-seed";

// Re-exported for the page components + tests that import the seed
// catalogue from this module. The data now lives in `./skills-seed`.
export { SEED_SKILLS, MOCK_LEARNED_SKILL_IDS };

export type SkillDataSource = "gateway" | "mock-offline" | "mock-bypass";

export interface SkillRow {
  /** Stable skill id (UUID string from `SkillStore`). */
  id: string;
  /** Slug-style name used in URL paths (`approval.route`). */
  name: string;
  version: string;
  description: string;
  /** `workspace` or `library` per the layered-lookup. */
  layer: string;
  /** Provenance source string (`learned`, `published:verified:team`, …). */
  provenance_source: string;
  /** Derived trust grade (`learned`, `published_verified`, …). */
  trust_grade: string;
  /** Mock/offline provenance for skills workers learned inside this tenant. */
  learned_by?: SkillLearnedBy;
}

export interface SkillLearnedBy {
  worker_id: string;
  worker_name: string;
  learned_at: string;
}

export interface FetchSkillsResult {
  skills: SkillRow[];
  source: SkillDataSource;
}

interface SkillsEnvelope {
  risk?: string;
  risk_defaulted?: boolean;
  count?: number;
  skills?: SkillRow[];
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

export async function fetchSkills(): Promise<FetchSkillsResult> {
  if (isAuthBypassed() && !isDemoLive()) {
    return { skills: SEED_SKILLS, source: "mock-bypass" };
  }
  try {
    const envelope = await apiGet<SkillsEnvelope>("/api/v1/skills");
    const skills = envelope.skills ?? [];
    if (skills.length === 0 && !isStrictProduction()) {
      return { skills: SEED_SKILLS, source: "mock-offline" };
    }
    return { skills, source: "gateway" };
  } catch (err) {
    if (!isStrictProduction() && isMissingOrUnreachable(err)) {
      return { skills: SEED_SKILLS, source: "mock-offline" };
    }
    throw err;
  }
}

/**
 * Worker (in the current workspace tenant) using a skill, paired with
 * the timestamp she learned it. Backs the "Workers using this skill"
 * section on the skill-detail page and the learning-loop chip.
 */
export interface SkillUsageWorker {
  worker_id: string;
  name: string;
  learned_at: string;
}

export interface SkillUsage {
  skill_id: string;
  skill_name: string;
  /** Canonical `argo_skill::trust::TrustGrade` discriminant. */
  trust_grade: string;
  /** Anonymized "Authored by SKYCOT" / "Authored by your org". */
  author_label: string;
  workers_using: SkillUsageWorker[];
  workers_using_count: number;
  /** Distinct workers across all tenants — aggregate only, no identities. */
  adoption_count: number;
}

export type SkillUsageSource = "gateway" | "mock-offline" | "mock-bypass";

export interface FetchSkillUsageResult {
  usage: SkillUsage;
  source: SkillUsageSource;
}

/**
 * Shared demo roster used to populate the "Workers using this skill"
 * card when the gateway is offline. Mirrors the cross-surface worker
 * names (Mariah / Priya / Jamal / Nori from the Scene-1 canon, plus
 * Peta / Patrick from Project & Ops and Devon / Alex as fill) so the
 * skill-detail page never renders an empty usage card on the mock path.
 */
const SKILL_USAGE_ROSTER: ReadonlyArray<{ worker_id: string; name: string }> = [
  { worker_id: "wkr-mariah", name: "Mariah" },
  { worker_id: "wkr-priya", name: "Priya" },
  { worker_id: "wkr-jamal", name: "Jamal" },
  { worker_id: "wkr-nori", name: "Nori" },
  { worker_id: "wkr-peta", name: "Peta" },
  { worker_id: "wkr-patrick", name: "Patrick" },
  { worker_id: "wkr-devon", name: "Devon" },
  { worker_id: "wkr-alex", name: "Alex" },
];

/**
 * Fixed pool of plausible "learned N ago" timestamps. Indexed
 * deterministically off the skill hash so the same skill always renders
 * the same dates — no `Date.now()`, so SSR and client agree and the
 * density vitest is stable.
 */
const SKILL_USAGE_LEARNED_AT: readonly string[] = [
  "2026-05-27T10:15:00.000Z",
  "2026-05-24T16:40:00.000Z",
  "2026-05-21T08:05:00.000Z",
  "2026-05-16T13:30:00.000Z",
  "2026-05-09T11:20:00.000Z",
  "2026-04-30T15:55:00.000Z",
  "2026-04-18T09:45:00.000Z",
  "2026-04-04T12:10:00.000Z",
];

/**
 * djb2 string hash → unsigned 32-bit int. Stable across runs and
 * platforms, so every derived value (worker count, roster slice,
 * adoption count) is deterministic per skill name.
 */
function djb2(input: string): number {
  let hash = 5381;
  for (let i = 0; i < input.length; i++) {
    hash = ((hash * 33) ^ input.charCodeAt(i)) >>> 0;
  }
  return hash;
}

/** Deterministically pick 2–4 distinct roster workers for a skill. */
function deterministicWorkers(name: string, hash: number): SkillUsageWorker[] {
  const count = 2 + (hash % 3); // 2, 3, or 4
  const start = hash % SKILL_USAGE_ROSTER.length;
  const workers: SkillUsageWorker[] = [];
  for (let i = 0; i < count; i++) {
    const worker = SKILL_USAGE_ROSTER[(start + i) % SKILL_USAGE_ROSTER.length];
    const learnedAt =
      SKILL_USAGE_LEARNED_AT[(hash + i * 7) % SKILL_USAGE_LEARNED_AT.length];
    workers.push({
      worker_id: worker.worker_id,
      name: worker.name,
      learned_at: learnedAt,
    });
  }
  return workers;
}

export function seedUsage(name: string): SkillUsage {
  const seed = SEED_SKILLS.find((s) => s.name === name || s.id === name);
  const learnedBy = seed?.learned_by;
  const hash = djb2(seed?.name ?? name);
  // Skills a worker learned inside this tenant carry their real
  // provenance worker, single-entry (preserves the learned-skill
  // contract). Every other seed gets a deterministic 2–4 worker set so
  // the usage card is never empty on the mock-offline path.
  const workersUsing = learnedBy
    ? [
        {
          worker_id: learnedBy.worker_id,
          name: learnedBy.worker_name,
          learned_at: learnedBy.learned_at,
        },
      ]
    : deterministicWorkers(seed?.name ?? name, hash);
  return {
    skill_id: name,
    skill_name: seed?.name ?? name,
    trust_grade: seed?.trust_grade ?? "published_verified",
    author_label: learnedBy
      ? `Learned by ${learnedBy.worker_name} · ${learnedBy.learned_at.slice(0, 10)}`
      : SEED_AUTHOR_LABEL_BY_NAME[seed?.name ?? name] ?? "Authored by SKYCOT",
    workers_using: workersUsing,
    workers_using_count: workersUsing.length,
    // Plausible cross-tenant adoption: deterministic 40–300 range.
    adoption_count: 40 + (hash % 261),
  };
}

/**
 * Server-side metadata resolver for `<title>`/OG cards on the skill
 * detail route. Used by `app/(app)/skills/[skillId]/layout.tsx` so
 * SSR stamps the skill-specific title + description before the client
 * component hydrates. Reads from the seed catalogue — no network hop
 * in `generateMetadata`, so cold loads on the demo path don't widen
 * the skeleton-flash window. Falls back to the skill id when the
 * catalogue has no entry, so the title still reads cleanly.
 */
export function resolveSkillDisplayMetadata(
  skillId: string,
): { title: string; description: string } {
  const seed = SEED_SKILLS.find((s) => s.name === skillId || s.id === skillId);
  if (seed) {
    return { title: seed.name, description: seed.description };
  }
  return {
    title: skillId,
    description: `Skill ${skillId} — where it came from, which workers use it, and how widely it's adopted.`,
  };
}

/** GET `/api/v1/skills/<name>/usage` — provenance + workers-using + adoption. */
export async function fetchSkillUsage(
  skillName: string,
): Promise<FetchSkillUsageResult> {
  if (isAuthBypassed() && !isDemoLive()) {
    return { usage: seedUsage(skillName), source: "mock-bypass" };
  }
  try {
    const usage = await apiGet<SkillUsage>(
      `/api/v1/skills/${encodeURIComponent(skillName)}/usage`,
    );
    return { usage, source: "gateway" };
  } catch (err) {
    if (!isStrictProduction() && isMissingOrUnreachable(err)) {
      return { usage: seedUsage(skillName), source: "mock-offline" };
    }
    throw err;
  }
}

/** Response envelope for `/api/v1/skills/learned-this-month`. */
export interface LearnedThisMonth {
  /** RFC3339 cutoff the server picked (first-of-month UTC). */
  since: string;
  count: number;
  skill_ids: string[];
}

export interface FetchLearnedThisMonthResult {
  learned: LearnedThisMonth;
  source: SkillUsageSource;
}

/** GET `/api/v1/skills/learned-this-month` — drives the marketplace filter chip. */
export async function fetchLearnedThisMonth(): Promise<FetchLearnedThisMonthResult> {
  const learnedFallback: LearnedThisMonth = {
    since: new Date(
      Date.UTC(new Date().getUTCFullYear(), new Date().getUTCMonth(), 1),
    ).toISOString(),
    count: MOCK_LEARNED_SKILL_IDS.length,
    skill_ids: MOCK_LEARNED_SKILL_IDS,
  };
  if (isAuthBypassed() && !isDemoLive()) {
    return { learned: learnedFallback, source: "mock-bypass" };
  }
  try {
    const learned = await apiGet<LearnedThisMonth>(
      "/api/v1/skills/learned-this-month",
    );
    return { learned, source: "gateway" };
  } catch (err) {
    if (!isStrictProduction() && isMissingOrUnreachable(err)) {
      return { learned: learnedFallback, source: "mock-offline" };
    }
    throw err;
  }
}
