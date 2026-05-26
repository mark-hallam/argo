/**
 * Single source of truth for the six sample worker personas
 * referenced across /team, /audit, /workers/<id>/overview, and the
 * cohorts surface.
 *
 * Wave-4 data-integrity audit (2026-05-20-wave-4-data-integrity.md)
 * found Sam, Mariah, and Nori had split personas: /team showed one
 * role, /audit showed a different one. This file is the canonical
 * home. The persona spine (name + role + vertical + headline +
 * traits) is anchored here; runtime-ish fields (status,
 * lastActiveAt) stay on the consuming sample file.
 *
 * The roles below match the operator-locked v1 demo runbook
 * (docs/skycot/operator/demo-runbook.md) and the personality chips
 * in WorkerPersonalityCard.tsx. Worker IDs are stable per the
 * brief's must-not-touch.
 */

export interface SamplePersonaRole {
  /** Public-facing role tagline ("books showings"). */
  tagline: string;
  /** Vertical key — receptionist, sales, support, marketplace, etc. */
  vertical: string;
  /** Compact role title ("Recruiter", "Marketplace Lister"). */
  title: string;
  /** Team / department label used in worker-overview charters. */
  team: string;
}

export interface SamplePersonaHeadline {
  bullets: string[];
  tomorrow?: string;
}

export interface SamplePersona {
  id: string;
  name: string;
  /** Two-letter avatar fallback. */
  initials: string;
  role: SamplePersonaRole;
  headline?: SamplePersonaHeadline;
  /** Short adjectives — sourced by personality cards + audit colour. */
  traits: string[];
}

export interface DemoSeedPersona {
  id: string;
  name: string;
  role: string;
  headline: SamplePersonaHeadline;
  vertical_id: string;
  traits: string[];
}

const PRIYA: SamplePersona = {
  id: "018f67d0-0000-7000-8000-000000000002",
  name: "Priya",
  initials: "PR",
  role: {
    tagline: "screens candidates and books interviews",
    vertical: "recruiting",
    title: "Recruiter",
    team: "People Ops",
  },
  headline: {
    bullets: ["8 candidates screened today", "1 interview pending your slot"],
    tomorrow: "next-day shortlist review",
  },
  traits: ["Thorough", "Respectful", "Fast", "Calibrated"],
};

const MARIAH: SamplePersona = {
  id: "018f67d0-0000-7000-8000-000000000001",
  name: "Mariah",
  initials: "MA",
  role: {
    tagline: "lists and re-prices items for sale",
    vertical: "marketplace",
    title: "Marketplace Lister",
    team: "Procurement",
  },
  headline: {
    bullets: ["3 listings live", "2 buyers waiting on you"],
    tomorrow: "morning re-price sweep at 06:30",
  },
  traits: ["Patient", "Detail-oriented", "Picture-perfect listings", "Polite"],
};

const JAMAL: SamplePersona = {
  id: "018f67d0-0000-7000-8000-000000000003",
  name: "Jamal",
  initials: "JA",
  role: {
    tagline: "handles customer support tickets",
    vertical: "support",
    title: "Tier-1 Support",
    team: "Customer Experience",
  },
  headline: {
    bullets: ["12 tickets resolved", "median reply 4m"],
  },
  traits: ["Calm under pressure", "Empathic", "Quick", "Honest"],
};

const NORI: SamplePersona = {
  id: "018f67d0-0000-7000-8000-000000000004",
  name: "Nori",
  initials: "NO",
  role: {
    tagline: "drafts product comms and announcements",
    vertical: "marketing",
    title: "Brand Comms",
    team: "Brand",
  },
  headline: {
    bullets: ["last campaign: 12,400 reached"],
    tomorrow: "Q2 product update draft at 09:00",
  },
  traits: ["On-voice", "Considered", "Schedule-respecting", "Reach-tracking"],
};

const PETA: SamplePersona = {
  id: "018f67d0-0000-7000-8000-000000000007",
  name: "Peta",
  initials: "PE",
  role: {
    tagline: "keeps standups, action items, and status moving",
    vertical: "project-ops",
    title: "Project Coordinator L2",
    team: "Project Management",
  },
  headline: {
    bullets: ["3 action items chased today", "2 standup digests drafted"],
    tomorrow: "morning standup digest at 09:00",
  },
  traits: ["Cadence-true", "Follow-through", "Calm escalator", "Context keeper"],
};

export const SAMPLE_PERSONAS: Record<string, SamplePersona> = {
  [PRIYA.id]: PRIYA,
  [MARIAH.id]: MARIAH,
  [JAMAL.id]: JAMAL,
  [NORI.id]: NORI,
  [PETA.id]: PETA,
};

/** Canonical roster — Scene-1 demo lockup. Keep in /team display order. */
export const SAMPLE_PERSONA_IDS: readonly string[] = [
  MARIAH.id,
  PETA.id,
  PRIYA.id,
  JAMAL.id,
  NORI.id,
];

function toDemoSeedPersona(persona: SamplePersona): DemoSeedPersona {
  return {
    id: persona.id,
    name: persona.name,
    role: persona.role.title,
    headline: persona.headline ?? { bullets: [] },
    vertical_id: persona.role.vertical,
    traits: persona.traits,
  };
}

/** Canonical v1 demo seed roster. Scene 1 narrates exactly these five. */
export const DEMO_SEED_PERSONAS: readonly DemoSeedPersona[] = [
  toDemoSeedPersona(MARIAH),
  toDemoSeedPersona(PETA),
  toDemoSeedPersona(PRIYA),
  toDemoSeedPersona(JAMAL),
  toDemoSeedPersona(NORI),
];

/** Lookup with a fall-through that throws on unknown id — seeds */
/* are static and a typo should fail loudly at module-load time. */
export function getSamplePersona(id: string): SamplePersona {
  const persona = SAMPLE_PERSONAS[id];
  if (!persona) {
    throw new Error(`Unknown sample persona id: ${id}`);
  }
  return persona;
}

/** Soft lookup for callers that may receive a non-sample id. */
export function findSamplePersona(id: string): SamplePersona | undefined {
  return SAMPLE_PERSONAS[id];
}

/** Minimal {id, name} pair used by audit rows + activity rows. */
export interface PersonaRef {
  id: string;
  name: string;
}

export function personaRef(id: string): PersonaRef {
  const persona = getSamplePersona(id);
  return { id: persona.id, name: persona.name };
}
