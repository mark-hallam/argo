import type { CohortSummary } from "@/lib/api/cohorts";

/**
 * Shared tile-rendering helpers for the D3 magazine cohort layout. The
 * cohort summary payload is sparse (vertical + trust band + counts), so
 * each tile derives its display strings (initials, kicker, description,
 * gradient title) from the same handful of fields.
 */

export const TRUST_BAND_COPY: Record<string, string> = {
  unknown: "Pre-trust",
  developing: "Developing",
  established: "Established",
  trusted: "Trusted",
  principal: "Principal",
};

export interface CohortPersonaTileMeta {
  name: string;
  level: string;
  role: string;
  stack: string;
  hero: string;
  initials: string;
  greatAt: readonly string[];
  escalatesWhen: readonly string[];
}

const PERSONA_TILE_META: Record<string, CohortPersonaTileMeta> = {
  "project-ops::project-coordinator": {
    name: "Peta",
    level: "L2",
    role: "Project Coordinator",
    stack: "Linear · GitHub · Slack",
    hero: "Coordinates work, action items, and standups across teams.",
    initials: "PE",
    greatAt: ["Action-item chase", "Standup digests"],
    escalatesWhen: ["Slipping milestones", "Outbound sends"],
  },
  "project-ops::status-reporter": {
    name: "Patrick",
    level: "L3",
    role: "Status Reporter",
    stack: "Linear · GitHub · Slack",
    hero: "Reports work, status, and risks weekly across teams.",
    initials: "PA",
    greatAt: ["Weekly exec summaries", "RAID log upkeep"],
    escalatesWhen: ["Stakeholder outbound", "Board updates"],
  },
  "sales-investor-relations::pitch-agent": {
    name: "Pitch",
    level: "L4",
    role: "Pitch Agent",
    stack: "Canonical Q&A · Slide control · Voice",
    hero: "Represents the founder in investor, customer, and press conversations.",
    initials: "PI",
    greatAt: ["Investor objections", "Live demos", "Mode switching"],
    escalatesWhen: ["Financials", "Unannounced roadmap", "Non-public customers"],
  },
};

export function cohortPersonaTileMeta(
  cohort: CohortSummary,
): CohortPersonaTileMeta | null {
  return PERSONA_TILE_META[cohort.cohort_id] ?? null;
}

export function trustBandLabel(band: string): string {
  return TRUST_BAND_COPY[band] ?? band;
}

export function cohortAvatarInitials(cohort: CohortSummary): string {
  const persona = cohortPersonaTileMeta(cohort);
  if (persona) return persona.initials;
  const vertical = (cohort.vertical || "?").trim();
  const band = (cohort.trust_grade_band || "?").trim();
  const v = vertical.charAt(0).toUpperCase() || "?";
  const b = band.charAt(0).toUpperCase() || "?";
  return `${v}${b}`;
}

export function cohortKickerLabel(cohort: CohortSummary): string {
  const persona = cohortPersonaTileMeta(cohort);
  if (persona) return `${persona.level} · ${persona.role}`;
  const band = trustBandLabel(cohort.trust_grade_band).toUpperCase();
  return `★ ${band} COHORT`;
}

/**
 * Hero copy for the lead tile. Splits into a plain prefix and a
 * gradient-accented suffix so the lead tile can render the accent via
 * `background-clip: text` for the locked brand-loud register.
 */
export function cohortDisplayTitle(cohort: CohortSummary): {
  plain: string;
  accent: string;
} {
  const persona = cohortPersonaTileMeta(cohort);
  if (persona) return { plain: `${persona.name} `, accent: persona.role };
  const band = trustBandLabel(cohort.trust_grade_band);
  const grade =
    cohort.trust_grade_max > cohort.trust_grade_min
      ? `L${cohort.trust_grade_min}–L${cohort.trust_grade_max}`
      : `L${cohort.trust_grade_min}`;
  const vertical = capitalise(cohort.vertical);
  return { plain: `${grade} ${band} `, accent: vertical };
}

/**
 * One-line, copy-driven description that varies by trust band so the
 * tile reads as an editorial blurb rather than a stats card.
 */
export function cohortDescription(cohort: CohortSummary): string {
  const persona = cohortPersonaTileMeta(cohort);
  if (persona) return persona.hero;
  const band = cohort.trust_grade_band;
  const vertical = cohort.vertical;
  const members = cohort.member_count;
  const skills = cohort.skill_count;
  const memberWord = members === 1 ? "teammate" : "teammates";
  const skillWord = skills === 1 ? "skill" : "skills";
  switch (band) {
    case "principal":
      return `${members} principal ${vertical} ${memberWord} with full delegation. Cohort owns its ${skills} ${skillWord} end-to-end.`;
    case "trusted":
      return `${members} trusted ${vertical} ${memberWord} on always-approval. ${skills} ${skillWord} shipped repeatedly.`;
    case "established":
      return `${members} established ${vertical} ${memberWord} on standard-approval. Covers ${skills} core ${skillWord}.`;
    case "developing":
      return `${members} developing ${vertical} ${memberWord} earning trust. ${skills} ${skillWord} in active rotation.`;
    case "unknown":
      return `${members} new ${vertical} ${memberWord} pre-trust. Pinned to ${skills} starter ${skillWord}.`;
    default:
      return `${members} ${vertical} ${memberWord} across ${skills} ${skillWord}.`;
  }
}

function capitalise(s: string): string {
  if (!s) return s;
  return s.charAt(0).toUpperCase() + s.slice(1);
}
