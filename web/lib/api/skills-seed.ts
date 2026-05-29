/**
 * Skills marketplace seed catalogue.
 *
 * Static demo data backing the `/skills` marketplace and skill-detail
 * pages when the gateway has no skills installed (a fresh workspace) or
 * is unreachable in dev. Split out of `lib/api/skills.ts` to keep that
 * module's fetch/fallback logic under the 500-LOC ceiling. Keep this
 * list in sync with the locked /skills design brief, not the real
 * registry. `lib/api/skills.ts` re-exports `SEED_SKILLS` and
 * `MOCK_LEARNED_SKILL_IDS` so existing importers are unaffected.
 */

import type { SkillRow } from "./skills";

export const SEED_SKILLS: SkillRow[] = [
  {
    id: "seed-pricing-competitor-scrape",
    name: "pricing.competitor_scrape",
    version: "0.1.0",
    description:
      "Competitor pricing scrape — pull competitor product pricing across listed channels and surface deltas as memory, with confidence + freshness flags.",
    layer: "library",
    provenance_source: "published:verified:argo-seed",
    trust_grade: "published_verified",
  },
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
    provenance_source: "workspace",
    trust_grade: "tenant",
  },
  {
    id: "seed-support-triage",
    name: "support.triage",
    version: "0.1.0",
    description:
      "Classify tickets, deflect with KB articles, escalate the rest with full context bundle.",
    layer: "library",
    provenance_source: "learned",
    trust_grade: "learned_unreviewed",
    learned_by: {
      worker_id: "wkr-mariah",
      worker_name: "Mariah",
      learned_at: "2026-05-23T09:15:00.000Z",
    },
  },
  {
    id: "seed-lead-enrich",
    name: "lead.enrich",
    version: "0.1.0",
    description:
      "Append firmographic + technographic context to inbound leads from public sources.",
    layer: "library",
    provenance_source: "learned",
    trust_grade: "learned_unreviewed",
    learned_by: {
      worker_id: "wkr-priya",
      worker_name: "Priya",
      learned_at: "2026-05-18T14:40:00.000Z",
    },
  },
  {
    id: "seed-expense-reconcile",
    name: "expense.reconcile",
    version: "0.1.0",
    description:
      "Reconcile corporate-card expenses against receipts and ledger codes; flag policy breaches for review.",
    layer: "library",
    provenance_source: "published:verified:argo-seed",
    trust_grade: "published_verified",
  },
  {
    id: "seed-forecast-cashflow",
    name: "forecast.cashflow",
    version: "0.1.0",
    description:
      "Roll a 13-week cash-flow forecast from payables, receivables, and bank feeds, with scenario toggles.",
    layer: "library",
    provenance_source: "workspace",
    trust_grade: "tenant",
  },
  {
    id: "seed-po-match",
    name: "po.match",
    version: "0.1.0",
    description:
      "Three-way match purchase orders, goods receipts, and invoices before releasing payment.",
    layer: "library",
    provenance_source: "seed",
    trust_grade: "seed",
  },
  {
    id: "seed-meeting-notetaker",
    name: "meeting.notetaker",
    version: "0.1.0",
    description:
      "Capture call notes, decisions, and action items, then sync them to the right CRM opportunity.",
    layer: "library",
    provenance_source: "seed",
    trust_grade: "seed",
  },
  {
    id: "seed-proposal-draft",
    name: "proposal.draft",
    version: "0.1.0",
    description:
      "Draft a tailored proposal from a discovery-call transcript and the current pricing book.",
    layer: "library",
    provenance_source: "workspace",
    trust_grade: "tenant",
  },
  {
    id: "seed-renewal-nudge",
    name: "renewal.nudge",
    version: "0.1.0",
    description:
      "Watch renewal dates and usage signals, then draft renewal outreach with the right offer.",
    layer: "library",
    provenance_source: "seed",
    trust_grade: "seed",
  },
  {
    id: "seed-kb-author",
    name: "kb.author",
    version: "0.1.0",
    description:
      "Turn resolved tickets into draft knowledge-base articles for a human to review and publish.",
    layer: "library",
    provenance_source: "published:verified:argo-seed",
    trust_grade: "published_verified",
  },
  {
    id: "seed-csat-followup",
    name: "csat.followup",
    version: "0.1.0",
    description:
      "Spot low-satisfaction tickets and draft a personalized recovery follow-up for the owner.",
    layer: "library",
    provenance_source: "workspace",
    trust_grade: "tenant",
  },
  {
    id: "seed-standup-summary",
    name: "standup.summary",
    version: "0.1.0",
    description:
      "Summarize overnight chat and issue-tracker activity into a morning stand-up digest by squad.",
    layer: "library",
    provenance_source: "seed",
    trust_grade: "seed",
  },
  {
    id: "seed-sprint-healthcheck",
    name: "sprint.healthcheck",
    version: "0.1.0",
    description:
      "Flag at-risk sprint items by scope creep, stale status, and blocked dependencies.",
    layer: "library",
    provenance_source: "workspace",
    trust_grade: "tenant",
  },
  {
    id: "seed-content-repurpose",
    name: "content.repurpose",
    version: "0.1.0",
    description:
      "Repurpose a long-form post into channel-native variants for social, email, and the blog.",
    layer: "library",
    provenance_source: "seed",
    trust_grade: "seed",
  },
  {
    id: "seed-campaign-tracking",
    name: "campaign.tracking",
    version: "0.1.0",
    description:
      "Generate consistent tracked campaign links and a reporting sheet from a campaign brief.",
    layer: "library",
    provenance_source: "workspace",
    trust_grade: "tenant",
  },
  {
    id: "seed-onboarding-checklist",
    name: "onboarding.checklist",
    version: "0.1.0",
    description:
      "Build a role-specific onboarding checklist and schedule the new hire's first-week touchpoints.",
    layer: "library",
    provenance_source: "published:verified:argo-seed",
    trust_grade: "published_verified",
  },
  {
    id: "seed-pto-coverage",
    name: "pto.coverage",
    version: "0.1.0",
    description:
      "Check time-off requests against team coverage and surface conflicts before approval.",
    layer: "library",
    provenance_source: "workspace",
    trust_grade: "tenant",
  },
  {
    id: "seed-vendor-review",
    name: "vendor.review",
    version: "0.1.0",
    description:
      "Compile a vendor renewal review — spend, service-level adherence, and alternatives at a glance.",
    layer: "library",
    provenance_source: "seed",
    trust_grade: "seed",
  },
];

export const MOCK_LEARNED_SKILL_IDS = ["support.triage", "lead.enrich"];

export const SEED_AUTHOR_LABEL_BY_NAME: Record<string, string> = {
  "pricing.competitor_scrape": "Authored by SKYCOT",
  "approval.route": "Authored by SKYCOT",
  "invoice.extract": "Authored by SKYCOT",
  "candidate.screen": "Authored by SKYCOT",
  "deal.qualify": "Authored by your org",
  "forecast.cashflow": "Authored by your org",
  "proposal.draft": "Authored by your org",
  "csat.followup": "Authored by your org",
  "sprint.healthcheck": "Authored by your org",
  "campaign.tracking": "Authored by your org",
  "pto.coverage": "Authored by your org",
};
