/**
 * Single source of truth for the seven sample worker personas
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
  /**
   * Operator-settable pronouns. Default is "they/them" so a missing
   * entry reads as inclusive rather than guessed from the name.
   * `web/lib/util/pronouns.ts#formatPronouns` translates this string
   * into the rendered forms (subject / object / possessive).
   */
  pronouns: string;
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
  id: "019e6785-928d-78b1-a6b2-136d9ed263ce",
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
  pronouns: "they/them",
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
  id: "019e6785-928a-7270-a766-f945b6cd7161",
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
  pronouns: "they/them",
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
  id: "019e6785-928e-7f20-803b-55266e526011",
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
  pronouns: "they/them",
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
  id: "019e6785-928f-7281-bf8d-e062bc277478",
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
  pronouns: "they/them",
  level: 15,
};

const PETA: SamplePersona = {
  id: "019e6785-928c-72b3-85c6-436fe5004c50",
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
  pronouns: "they/them",
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

const PORTIA: SamplePersona = {
  id: "019e6785-9290-7a41-9c2e-4b8d10f5c7a2",
  name: "Portia",
  initials: "PO",
  slug: "portia",
  role: {
    tagline: "pitches the company to investors, bounded by approved facts",
    vertical: "pitch-agent",
    title: "Pitch Agent",
    team: "Sales & Investor Relations",
  },
  headline: {
    bullets: [
      "matched 12 investor questions in under 500ms each",
      "refused 1 unsupported financials claim — escalated to founder",
    ],
    tomorrow: "Thursday 10:00 Series A rehearsal with the founder",
  },
  traits: ["Founder-caliber", "Evidence-led", "Brand-safe", "Truth-bounded"],
  pronouns: "they/them",
  level: 11,
  personality: {
    greatAt: [
      "Answering investor questions from the approved-facts pack",
      "Telling the founder story in a tight, evidence-led arc",
      "Staying on-brand across rehearsal, web embed, and live Zoom",
      "Knowing the exact boundary of what's been signed off to say",
    ],
    escalateWhen: [
      "A question needs a financial figure not in the approved facts",
      "An investor asks for forward-looking guidance or projections",
      "The ask touches legal, cap-table, or unannounced product",
      "Confidence on a claim falls below the brand-safe threshold",
    ],
    signatureBehaviors: [
      "Never invents a number — cites the approved fact or escalates",
      "Always names the founder when handing off an unsupported claim",
      "Closes a pitch with a single, specific next-step ask",
    ],
  },
};

const GRANT: SamplePersona = {
  id: "019e6785-9291-7b52-8d3f-6c9e21a6d8b3",
  name: "Grant",
  initials: "GR",
  slug: "grant",
  role: {
    tagline: "owns inbound growth — qualifies signups and drafts outreach",
    vertical: "gtm",
    title: "GTM Engineer",
    team: "Growth",
  },
  headline: {
    bullets: ["18 signups qualified today", "6 outreach drafts waiting on you"],
    tomorrow: "morning lead-scoring sweep at 08:00",
  },
  traits: ["Relationship-driven", "Persistent", "Brand-safe", "Pipeline-minded"],
  pronouns: "they/them",
  level: 7,
  personality: {
    greatAt: [
      "Scoring inbound signups against the ideal-customer profile",
      "Drafting first-touch outreach that sounds human, not templated",
      "Spotting which trials are warming up and which are going cold",
      "Keeping the pipeline tidy so nothing slips between stages",
    ],
    escalateWhen: [
      "A lead asks for custom pricing or a contract change",
      "A prospect requests a security or compliance review",
      "An outbound claim would stray outside the approved messaging",
      "A high-fit enterprise account goes quiet for over a week",
    ],
    signatureBehaviors: [
      "Always tailors the first line to something specific about the lead",
      "Never sends outreach that hasn't cleared the brand-safe check",
      "Flags a hot enterprise signup to a human within the hour",
    ],
  },
};

const MARCUS: SamplePersona = {
  id: "019e6785-9292-7c63-9e40-7d0f32b7e9c4",
  name: "Marcus",
  initials: "MC",
  slug: "marcus",
  role: {
    tagline: "drafts replies + thought-leadership in Mark's voice — draft-only, never auto-posted",
    vertical: "marketing-agent",
    title: "Marketing",
    team: "Brand & Marketing",
  },
  headline: {
    bullets: [
      "4 reply drafts waiting on you",
      "held 1 draft that touched an unverified claim — escalated to founder",
    ],
    tomorrow: "morning mention sweep at 08:30",
  },
  traits: ["On-voice", "Listening-first", "Truth-bounded", "Draft-only"],
  pronouns: "they/them",
  level: 6,
  personality: {
    greatAt: [
      "Drafting replies that add a real point of view, not a generic 'great post!'",
      "Writing thought-leadership in Mark's voice — concrete open, idea built, a take to land on",
      "Listening for SKYCOT mentions + the conversations the company should be in",
      "Knowing the exact boundary of what's approved to say — and deferring the rest",
    ],
    escalateWhen: [
      "A draft would need a valuation, revenue, funding, or customer-count number",
      "A reply would commit to pricing, a date, or an unannounced feature",
      "A conversation is contentious enough that the company's reputation is on the line",
      "Any public/transmit step — those are ALWAYS approval-gated; Marcus drafts, you post",
    ],
    signatureBehaviors: [
      "Files every draft to the approval queue — never auto-posts",
      "Varies the angle so the same hook doesn't repeat on the same channel",
      "Holds a draft that strays into a guarded topic for founder review",
    ],
  },
};

export const SAMPLE_PERSONAS: Record<string, SamplePersona> = {
  [PRIYA.id]: PRIYA,
  [MARIAH.id]: MARIAH,
  [JAMAL.id]: JAMAL,
  [NORI.id]: NORI,
  [PETA.id]: PETA,
  [PORTIA.id]: PORTIA,
  [GRANT.id]: GRANT,
  [MARCUS.id]: MARCUS,
};

// Canonical sample-worker UUIDs aligned with the `argo demo seed`
// gateway-side canon (UUID v7 generated at seed time, stable per
// tenant once seeded). Exported here so a single grep finds the
// source of truth; consumers import via re-exports in
// `sample-workers-detail.ts` for the public name.
export const SAMPLE_MARIAH_ID = MARIAH.id;
export const SAMPLE_PETA_ID = PETA.id;
export const SAMPLE_PRIYA_ID = PRIYA.id;
export const SAMPLE_JAMAL_ID = JAMAL.id;
export const SAMPLE_NORI_ID = NORI.id;
export const SAMPLE_PORTIA_ID = PORTIA.id;
export const SAMPLE_GRANT_ID = GRANT.id;
export const SAMPLE_MARCUS_ID = MARCUS.id;

/** Canonical roster — Scene-1 demo lockup. Keep in /team display order. */
export const SAMPLE_PERSONA_IDS: readonly string[] = [
  MARIAH.id,
  PETA.id,
  PRIYA.id,
  JAMAL.id,
  NORI.id,
  PORTIA.id,
  GRANT.id,
  MARCUS.id,
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

/** Canonical v1 demo seed roster. Scene 1 narrates exactly these eight
 * (DEC-028 added Marcus, the marketing teammate, as the third hero). */
export const DEMO_SEED_PERSONAS: readonly DemoSeedPersona[] = [
  toDemoSeedPersona(MARIAH),
  toDemoSeedPersona(PETA),
  toDemoSeedPersona(PRIYA),
  toDemoSeedPersona(JAMAL),
  toDemoSeedPersona(NORI),
  toDemoSeedPersona(PORTIA),
  toDemoSeedPersona(GRANT),
  toDemoSeedPersona(MARCUS),
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

const PERSONA_BY_NAME: Record<string, SamplePersona> = Object.fromEntries(
  Object.values(SAMPLE_PERSONAS).map((p) => [p.name.toLowerCase(), p]),
);

/**
 * Soft lookup by display name (case-insensitive).
 *
 * The live gateway's `argo demo seed` mints fresh UUIDs each time it
 * seeds a tenant, so the worker ids returned by the gateway no longer
 * match the hardcoded ids on the spine above — a UUID-keyed
 * `findSamplePersona(id)` misses in demo-live. The *name* is stable
 * across re-seeds, though (Marcus is always "Marcus"), so this lets a
 * consumer recover the canonical persona — including the rich
 * `personality` copy the gateway doesn't return — from the name the
 * gateway DOES return. The persona names are unique (guarded by
 * `sample-personas.test.ts`).
 */
export function findSamplePersonaByName(
  name: string | undefined | null,
): SamplePersona | undefined {
  if (!name) return undefined;
  return PERSONA_BY_NAME[name.toLowerCase()];
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
