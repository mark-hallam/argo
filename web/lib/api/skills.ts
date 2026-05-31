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

import { apiGet, apiPost } from "@/lib/api/client";
import { ApiError } from "@/lib/api/types";
import { isDemoLive } from "@/lib/skycot/demo-mode";
import { fetchTeam } from "@/lib/api/team";
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

/**
 * One REAL eval re-run for the learning-loop card, from
 * `skill_reeval_history`. NB: these are scheduled eval re-runs of the
 * skill (the same rows that drive `/evals`), NOT trust-grade promotion
 * events — there is no promotion-event table, so the card never
 * fabricates a promotion arc.
 */
export interface SkillEvalRun {
  /** Skill version the eval ran against (e.g. `v3`). */
  version: string;
  /** 0..1 pass rate, or `null` when the eval itself errored. */
  pass_rate: number | null;
  /** Prior run's pass rate, or `null` for the first (baseline) run. */
  previous_pass_rate?: number | null;
  /** `stable` / `regressed` / `no_baseline` — the real eval decision. */
  decision: "stable" | "regressed" | "no_baseline";
  /** ISO-8601 timestamp of the run. */
  ran_at: string;
}

/**
 * REAL telemetry bundle for one skill, from the gateway
 * `GET /api/v1/skills/<id-or-name>/telemetry` endpoint. Every field is
 * sourced from a live table (`skill_reeval_history`, `worker_skill`,
 * the skill's provenance); fields the retired fixture invented
 * (per-run `latency_ms` / `worker_name`) are deliberately absent.
 */
export interface SkillTelemetry {
  skill_id: string;
  skill_name: string;
  /** Canonical `argo_skill::TrustGrade` discriminant. */
  current_grade: string;
  /** REAL count of scheduled eval re-runs for this skill. */
  run_count: number;
  /** REAL distinct workers across all tenants (aggregate only). */
  distinct_workers: number;
  /** Eval re-run history, oldest → newest. */
  eval_history: SkillEvalRun[];
  /**
   * Cumulative adoption series — `distinct workers who had learned this
   * skill by day D`, oldest → newest (30 points). Bucketed from
   * `worker_skill.created_at`.
   */
  adoption_trend: number[];
  /** Newest 3 eval re-runs, newest-first. */
  recent_runs: SkillEvalRun[];
}

export interface FetchSkillTelemetryResult {
  /** `null` → the card renders its honest empty state. */
  telemetry: SkillTelemetry | null;
  source: SkillUsageSource;
}

/**
 * GET `/api/v1/skills/<id-or-name>/telemetry` — REAL learning-loop
 * telemetry. Accepts the skill name or the catalog UUID (the endpoint
 * resolves either form). Used by the skill drill-in's "Learning loop"
 * card; this retired the hardcoded `sample-skill-telemetry.ts` fixture.
 *
 * Demo-live contract (NO THEATER): in `auth-bypass + demo-live` we hit
 * the real gateway; if it is unreachable / 404s we return
 * `{ telemetry: null }` so the card shows its honest EMPTY state — never
 * a fabricated fixture. In plain auth-bypass dev (no demo-live) we also
 * return `{ telemetry: null }`: there is no offline fixture any more, so
 * the card honestly reports "no runs yet" rather than inventing trend
 * data. Strict prod propagates the ApiError.
 */
export async function fetchSkillTelemetry(
  skillNameOrId: string,
): Promise<FetchSkillTelemetryResult> {
  if (isAuthBypassed() && !isDemoLive()) {
    return { telemetry: null, source: "mock-bypass" };
  }
  try {
    const telemetry = await apiGet<SkillTelemetry>(
      `/api/v1/skills/${encodeURIComponent(skillNameOrId)}/telemetry`,
    );
    return { telemetry, source: "gateway" };
  } catch (err) {
    if (!isStrictProduction() && isMissingOrUnreachable(err)) {
      // Honest empty state — NOT the (deleted) fixture.
      return { telemetry: null, source: "mock-offline" };
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

/**
 * Result of an Install action: how the request was satisfied plus the
 * number of workers the skill was made available to.
 *
 * `source` follows the same three-value contract as the read fetchers:
 * `gateway` (real `POST /api/v1/workers/<id>/skills` round-trip),
 * `mock-bypass` (auth-bypass dev short-circuit, no network), or
 * `mock-offline` (gateway unreachable / 404 in dev).
 */
export interface InstallSkillResult {
  skillId: string;
  /** Worker ids the skill was assigned to (added to their active overlay). */
  workerIds: string[];
  source: SkillDataSource;
}

/**
 * Mock-path install roster. Mirrors the Scene-1 canon hero workers so
 * the auth-bypass demo path reports a plausible "available to N workers"
 * count without a gateway round-trip.
 */
const SEED_INSTALL_TARGET_WORKER_IDS: readonly string[] = [
  "wkr-mariah",
  "wkr-priya",
  "wkr-jamal",
];

/**
 * Install a catalog skill by assigning it to the workspace's workers.
 *
 * "Install → available to workers" is, in the runtime, a `worker_skill`
 * overlay row per worker. The real gateway endpoint is
 * `POST /api/v1/workers/<id>/skills` with `{ skill_id: "<uuid>" }`
 * (`crates/argo-cli/src/handlers/gateway/routes_worker_skills.rs`
 * `add_response` → 201, audited as `worker.skills.added`). There is no
 * single tenant-wide "install" verb on the gateway; making a skill
 * available to the team means adding it to each teammate's active set,
 * which is exactly what this does — the same endpoint the per-worker
 * Skills tab already calls via `worker-skills.ts::enableSkill`.
 *
 * `skillId` must be the catalog skill's UUID (the gateway's
 * `SkillRow.id` / `SkillUsage.skill_id`); the endpoint rejects
 * name-style ids. On the mock paths we never reach the gateway, so the
 * non-UUID seed id is irrelevant.
 *
 * Mock-fallback contract mirrors the read fetchers:
 * - `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=1` (and not demo-live) →
 *   resolve a mock success against the seed roster, no network call.
 * - 404 / gateway-unreachable in dev → mock success (mock-offline).
 * - Strict prod → ApiError propagates.
 *
 * When `workerIds` is omitted the target roster is resolved from
 * `fetchTeam()` so the marketplace can install to the whole team in one
 * click (the Scene-4 "Available to workers" beat).
 */
export async function installSkillToWorkers(
  skillId: string,
  workerIds?: string[],
): Promise<InstallSkillResult> {
  if (isAuthBypassed() && !isDemoLive()) {
    const ids = workerIds ?? [...SEED_INSTALL_TARGET_WORKER_IDS];
    return { skillId, workerIds: ids, source: "mock-bypass" };
  }

  let targets = workerIds;
  if (!targets || targets.length === 0) {
    const team = await fetchTeam();
    targets = team.workers.map((w) => w.id);
  }

  if (targets.length === 0) {
    // No workers to assign to — surface as a real error so the UI can
    // tell the operator to hire a teammate first rather than silently
    // claiming success.
    throw new ApiError(
      409,
      "No workers to install this skill to — hire a teammate first.",
      "no_workers",
    );
  }

  try {
    await Promise.all(
      targets.map((workerId) =>
        apiPost<unknown>(
          `/api/v1/workers/${encodeURIComponent(workerId)}/skills`,
          { skill_id: skillId },
        ),
      ),
    );
    return { skillId, workerIds: targets, source: "gateway" };
  } catch (err) {
    if (!isStrictProduction() && isMissingOrUnreachable(err)) {
      return { skillId, workerIds: targets, source: "mock-offline" };
    }
    throw err;
  }
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
