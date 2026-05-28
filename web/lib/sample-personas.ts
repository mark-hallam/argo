/**
 * Single source of truth for the six sample worker personas
 * referenced across /team, /audit, /workers/<id>/overview, and the
 * cohorts surface.
 *
 * Wave-4 data-integrity audit (2026-05-20-wave-4-data-integrity.md)
 * found Sam, Mariah, and Nori had split personas: /team showed one
 * role, /audit showed a different one. This file is the canonical
 * home. The persona spine (name + role + vertical + headline +
 * traits + level + personality) is anchored here; runtime-ish
 * fields (status, lastActiveAt) stay on the consuming sample file.
 *
 * The roles below match the operator-locked v1 demo runbook
 * (docs/skycot/operator/demo-runbook.md) and the personality chips
 * in WorkerPersonalityCard.tsx. Worker IDs are stable per the
 * brief's must-not-touch.
 *
 * `level` + `personality` were folded onto the spine via
 * fix-peta-personality-trait-level-lookup-coverage to kill the
 * sibling-table sprawl that left Peta missing from three parallel
 * name-keyed maps after #2367 added her to the roster.
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

/** Behavior copy rendered by WorkerPersonalityCard. Traits live on
 * the persona top-level so /team chips and /workers chips stay
 * sourced from one place. */
export interface SamplePersonality {
  greatAt: string[];
  escalateWhen: string[];
  signatureBehaviors: string[];
}

export interface SamplePersona {
  id: string;
  name: string;
  /** Two-letter avatar fallback. */
  initials: string;
  /**
   * URL-safe alias used in operator-visible deep links (e.g.
   * `/audit?worker=mariah`) so a 36-char UUID isn't leaked on a
   * screen-share. UUID remains the canonical key for storage +
   * gateway filtering; slug is SPA-only and resolved at parse time.
   */
  slug: string;
  role: SamplePersonaRole;
  headline?: SamplePersonaHeadline;
  /** Short adjectives — sourced by personality cards + audit colour. */
  traits: string[];
  /** Curated gamification level shown on the holographic teammate
   * card. Hand-tuned per persona; absent personas fall back to a
   * stable hash in the consumer. */
  level: number;
  /** Optional behaviour copy for WorkerPersonalityCard. Personas
   * without an explicit entry fall through to DEFAULT_PERSONALITY in
   * the card. */
  personality?: SamplePersonality;
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
  slug: "priya",
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
  level: 9,
  personality: {
    greatAt: [
      "First-pass skill-match scoring against the JD",
      "Spotting tenure red flags + cultural mismatch",
      "Writing rejection emails that don't burn bridges",
      "Booking interview slots that respect both calendars",
    ],
    escalateWhen: [
      "Score is in the 50-65 band (neither yes nor no)",
      "Candidate has worked at a direct competitor",
      "JD has been edited in the last 24h",
      "Hiring manager hasn't replied to the last 3 candidates",
    ],
    signatureBehaviors: [
      "Always sends the rejection within 48h of decline",
      "Books a buffer week before a senior interview",
      "Asks for clarification before discounting a strong yes",
    ],
  },
};

const MARIAH: SamplePersona = {
  id: "018f67d0-0000-7000-8000-000000000001",
  name: "Mariah",
  initials: "MA",
  slug: "mariah",
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
  level: 5,
  personality: {
    greatAt: [
      "Spotting price gaps against the live market",
      "Cropping + ordering photos for best buyer scroll",
      "Writing tight 60-word listing copy with real specifics",
      "Knowing when a listing needs a category change",
    ],
    escalateWhen: [
      "Price drop below the seller's floor",
      "Photos look heavily edited or stock",
      "Buyer asks a question outside the listing scope",
      "Multiple competing offers on the same item",
    ],
    signatureBehaviors: [
      "Always confirms the seller before posting publicly",
      "Asks for a single source photo if you give multiples",
      "Refuses to use stock imagery — always your photos",
    ],
  },
};

const JAMAL: SamplePersona = {
  id: "018f67d0-0000-7000-8000-000000000003",
  name: "Jamal",
  initials: "JA",
  slug: "jamal",
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
  level: 12,
  personality: {
    greatAt: [
      "Classifying tickets by severity in <2 minutes",
      "Pulling the right KB article for self-deflection",
      "Writing follow-ups that land in your customer's voice",
      "Spotting the same issue across 5+ tickets",
    ],
    escalateWhen: [
      "Customer mentions churn or 'cancel' in any reply",
      "Same customer opens 3+ tickets in 24h",
      "Issue involves billing or a refund over $200",
      "Customer is on a VIP or enterprise tier",
    ],
    signatureBehaviors: [
      "Always names the human owner when escalating",
      "Sends a 'still investigating' note at the 4h mark",
      "Never closes a ticket without a one-line outcome",
    ],
  },
};

const NORI: SamplePersona = {
  id: "018f67d0-0000-7000-8000-000000000004",
  name: "Nori",
  initials: "NO",
  slug: "nori",
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
  level: 15,
};

const PETA: SamplePersona = {
  id: "018f67d0-0000-7000-8000-000000000007",
  name: "Peta",
  initials: "PE",
  slug: "peta",
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
  level: 8,
  personality: {
    greatAt: [
      "Driving the standup digest at the same time every day",
      "Tracking action-item ownership across multiple owners",
      "Calling out scope creep on the live sprint board",
      "Drafting status updates in the operator's voice",
    ],
    escalateWhen: [
      "An owner has missed the same action item three times",
      "Sprint scope shifts by more than 20% mid-week",
      "A blocker has been open for more than 72 hours",
      "Two stakeholders publicly disagree on the same call",
    ],
    signatureBehaviors: [
      "Always closes the standup with a one-line digest",
      "Names the owner before writing the action item",
      "Never moves a deadline without naming the trade-off",
    ],
  },
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

const SLUG_BY_ID: Record<string, string> = Object.fromEntries(
  Object.values(SAMPLE_PERSONAS).map((p) => [p.id, p.slug]),
);

const ID_BY_SLUG: Record<string, string> = Object.fromEntries(
  Object.values(SAMPLE_PERSONAS).map((p) => [p.slug, p.id]),
);

/**
 * Return the URL-safe slug for a known persona id, or `undefined`
 * when the id isn't in the sample registry. Callers should fall back
 * to the id itself so the link still works for non-sample workers.
 */
export function lookupWorkerSlug(id: string): string | undefined {
  return SLUG_BY_ID[id];
}

/**
 * Inverse of `lookupWorkerSlug`. Used by `/audit` to translate a
 * shareable `?worker=mariah` URL back into the UUID the filter
 * machinery + gateway recognise.
 */
export function lookupWorkerIdFromSlug(slug: string): string | undefined {
  return ID_BY_SLUG[slug];
}
