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

/**
 * Seed catalog used when the gateway has no skills installed (a fresh
 * workspace) or is unreachable in dev. Mirrors the marketing-friendly
 * names the operator showed in the locked /skills design — keep this
 * list in sync with the design brief, not with the real registry.
 */
export const SEED_SKILLS: SkillRow[] = [
  {
    id: "seed-approval-route",
    name: "approval.route",
    version: "0.1.0",
    description:
      "Route approvals to the right human based on risk, dollar, and seniority rules.",
    layer: "library",
    provenance_source: "seed",
    trust_grade: "seed",
  },
  {
    id: "seed-invoice-extract",
    name: "invoice.extract",
    version: "0.1.0",
    description:
      "Pull structured line items from PDF or scanned invoices with vendor reconciliation.",
    layer: "library",
    provenance_source: "seed",
    trust_grade: "seed",
  },
  {
    id: "seed-candidate-screen",
    name: "candidate.screen",
    version: "0.1.0",
    description:
      "First-pass candidate screen against a JD with skill-match scoring and red-flag flags.",
    layer: "library",
    provenance_source: "seed",
    trust_grade: "seed",
  },
  {
    id: "seed-deal-qualify",
    name: "deal.qualify",
    version: "0.1.0",
    description:
      "BANT-style outbound qualification, calendar-link insertion, CRM update on accept.",
    layer: "library",
    provenance_source: "seed",
    trust_grade: "seed",
  },
  {
    id: "seed-support-triage",
    name: "support.triage",
    version: "0.1.0",
    description:
      "Classify tickets, deflect with KB articles, escalate the rest with full context bundle.",
    layer: "library",
    provenance_source: "seed",
    trust_grade: "seed",
  },
  {
    id: "seed-lead-enrich",
    name: "lead.enrich",
    version: "0.1.0",
    description:
      "Append firmographic + technographic context to inbound leads from public sources.",
    layer: "library",
    provenance_source: "seed",
    trust_grade: "seed",
  },
];

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
  if (isAuthBypassed()) {
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

function seedUsage(name: string): SkillUsage {
  return {
    skill_id: name,
    skill_name: name,
    trust_grade: "published_verified",
    author_label: "Authored by SKYCOT",
    workers_using: [],
    workers_using_count: 0,
    adoption_count: 0,
  };
}

/** GET `/api/v1/skills/<name>/usage` — provenance + workers-using + adoption. */
export async function fetchSkillUsage(
  skillName: string,
): Promise<FetchSkillUsageResult> {
  if (isAuthBypassed()) {
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
  const empty: LearnedThisMonth = {
    since: new Date(Date.UTC(new Date().getUTCFullYear(), new Date().getUTCMonth(), 1)).toISOString(),
    count: 0,
    skill_ids: [],
  };
  if (isAuthBypassed()) {
    return { learned: empty, source: "mock-bypass" };
  }
  try {
    const learned = await apiGet<LearnedThisMonth>(
      "/api/v1/skills/learned-this-month",
    );
    return { learned, source: "gateway" };
  } catch (err) {
    if (!isStrictProduction() && isMissingOrUnreachable(err)) {
      return { learned: empty, source: "mock-offline" };
    }
    throw err;
  }
}
