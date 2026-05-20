/**
 * Per-vertical config for the `/for/<vertical>` landing pages.
 *
 * Each vertical inherits the locked `/landing` brand register (Audiowide
 * hero, Inter body, purple→pink gradient) and overlays vertical-tailored
 * copy + a subtle accent tint on the hero.
 */

export type VerticalSlug =
  | "sales"
  | "recruiting"
  | "support"
  | "finance"
  | "marketing"
  | "ops";

export type WorkflowCard = {
  title: string;
  outcome: string;
};

export type VerticalConfig = {
  slug: VerticalSlug;
  /** Page <title> */
  title: string;
  /** Page <meta description> */
  description: string;
  /** OG image path (file is a v1.x placeholder) */
  ogImage: string;
  /** Eyebrow above the hero headline */
  eyebrow: string;
  /** Hero headline. The {accent} marker is replaced by the gradient-painted phrase. */
  headline: { lead: string; accent: string; tail?: string };
  /** Hero subhead (1 sentence) */
  subhead: string;
  /** Hex accent color used for the hero tint + secondary accents */
  accent: string;
  /** Section heading for the 3-card workflow region */
  workflowHeading: { lead: string; accent: string };
  workflows: [WorkflowCard, WorkflowCard, WorkflowCard];
  /** Signature workflow showcased mid-page */
  signatureWorkflow: {
    name: string;
    blurb: string;
    /** Steps shown in the inline mockup */
    steps: { actor: string; action: string }[];
  };
  /** Stub customer quote — replace when real ones land */
  customerQuote: {
    text: string;
    name: string;
    role: string;
    company: string;
  };
  /** CTA strip copy */
  ctaStrip: { headline: string; sub: string };
};

const SALES: VerticalConfig = {
  slug: "sales",
  title: "AI sales teammates that never sleep — SKYCOT",
  description:
    "Qualify leads, draft outreach, and book demos with AI sales workers that ask before they act. Audit-first, single-binary, enterprise-grade.",
  ogImage: "/og/og-sales.png",
  eyebrow: "AI for sales teams",
  headline: { lead: "Your AI sales team that", accent: "never sleeps." },
  subhead:
    "Hire teammates that qualify every inbound lead, draft outreach in your voice, and book demos while you're asleep. Approval-first, fully audit-logged, never off-brand.",
  accent: "#8b5cf6",
  workflowHeading: { lead: "Built for", accent: "sales teams" },
  workflows: [
    {
      title: "Qualify inbound leads",
      outcome: "Every form fill scored, enriched, and routed in under 60 seconds.",
    },
    {
      title: "Draft outreach sequences",
      outcome: "Personalised first touches drafted in your voice, ready to send.",
    },
    {
      title: "Book demos automatically",
      outcome: "Calendar-aware booking with the right rep, no back-and-forth.",
    },
  ],
  signatureWorkflow: {
    name: "Inbound lead → qualified demo",
    blurb: "Watch how a single form fill becomes a booked demo, hands-free.",
    steps: [
      { actor: "Trigger", action: "New form fill on /pricing" },
      { actor: "Skylar", action: "Enriches lead from Clearbit + LinkedIn" },
      { actor: "Skylar", action: "Scores fit · proposes routing to AE" },
      { actor: "Approval", action: "Rep one-taps 'Send' on draft outreach" },
      { actor: "Skylar", action: "Books demo on rep's calendar" },
    ],
  },
  customerQuote: {
    text: "Skylar qualified 2,800 inbound leads our reps didn't have to touch. Demo conversions are up 38%.",
    name: "Daniela Reyes",
    role: "VP of Sales",
    company: "Northwind Commerce",
  },
  ctaStrip: {
    headline: "Hire your first sales teammate",
    sub: "Live in Slack in under four minutes. No credit card.",
  },
};

const RECRUITING: VerticalConfig = {
  slug: "recruiting",
  title: "AI recruiting teammates — Hire faster, screen better — SKYCOT",
  description:
    "Screen applications, schedule interviews, and chase references with AI recruiting workers. Approval-first, audit-logged, never lose a candidate.",
  ogImage: "/og/og-recruiting.png",
  eyebrow: "AI for recruiting teams",
  headline: {
    lead: "Hire faster. Screen better.",
    accent: "Never lose a candidate.",
  },
  subhead:
    "AI teammates that read every application, schedule first-rounds against your calendar, and chase references so your recruiters can spend time with humans.",
  accent: "#0891b2",
  workflowHeading: { lead: "Built for", accent: "recruiting teams" },
  workflows: [
    {
      title: "Screen incoming applications",
      outcome: "Every candidate scored against your rubric in minutes, not days.",
    },
    {
      title: "Schedule first-round interviews",
      outcome: "Calendar-aware scheduling across panels, time zones, and reschedules.",
    },
    {
      title: "Reference-check candidates",
      outcome: "Reference emails sent, replies summarised, red flags surfaced.",
    },
  ],
  signatureWorkflow: {
    name: "Application → interview booked",
    blurb: "From Greenhouse webhook to a booked first-round, hands-free.",
    steps: [
      { actor: "Trigger", action: "New application in Greenhouse" },
      { actor: "Priya", action: "Scores against role rubric + JD" },
      { actor: "Priya", action: "Drafts 'next step' email" },
      { actor: "Approval", action: "Recruiter one-taps 'Send' on screen-pass" },
      { actor: "Priya", action: "Books first-round on panel calendars" },
    ],
  },
  customerQuote: {
    text: "Priya screens 400 applications a week and never drops a candidate. Time-to-first-round is down from 12 days to 3.",
    name: "Marcus Lin",
    role: "Head of Talent",
    company: "Apex Recruiting",
  },
  ctaStrip: {
    headline: "Hire your first recruiting teammate",
    sub: "Live in Slack in under four minutes. No credit card.",
  },
};

const SUPPORT: VerticalConfig = {
  slug: "support",
  title: "AI customer support teammates — SKYCOT",
  description:
    "Triage tickets, draft replies, and escalate the hard ones. AI support workers that get it right the first time, audit-logged on every action.",
  ogImage: "/og/og-support.png",
  eyebrow: "AI for support teams",
  headline: {
    lead: "Customer support that gets it right",
    accent: "the first time.",
  },
  subhead:
    "AI teammates that triage every ticket by urgency, draft replies in your tone, and escalate when they're stuck. Approval-first, audit-first, on-brand.",
  accent: "#f59e0b",
  workflowHeading: { lead: "Built for", accent: "support teams" },
  workflows: [
    {
      title: "Triage tickets by urgency",
      outcome: "Every ticket scored on impact + sentiment, queued for the right human.",
    },
    {
      title: "Draft replies in your tone",
      outcome: "First-draft answers grounded in your knowledge base, ready to send.",
    },
    {
      title: "Escalate when stuck",
      outcome: "Hand-off to humans with a summary, history, and proposed next step.",
    },
  ],
  signatureWorkflow: {
    name: "Ticket → resolved (or escalated)",
    blurb: "How Jamal handles a new Zendesk ticket end-to-end.",
    steps: [
      { actor: "Trigger", action: "New Zendesk ticket arrives" },
      { actor: "Jamal", action: "Reads thread + customer history" },
      { actor: "Jamal", action: "Drafts reply from KB + tone guide" },
      { actor: "Approval", action: "Agent one-taps 'Send' or 'Escalate'" },
      { actor: "Jamal", action: "Closes ticket · tags trust score" },
    ],
  },
  customerQuote: {
    text: "Jamal handles 12,000 tickets a month. Median reply time is 4 minutes. CSAT is up 11 points.",
    name: "Aisha Patel",
    role: "Director of Customer Experience",
    company: "Zenith Support",
  },
  ctaStrip: {
    headline: "Hire your first support teammate",
    sub: "Live in Slack in under four minutes. No credit card.",
  },
};

const FINANCE: VerticalConfig = {
  slug: "finance",
  title: "AI finance teammates — Close the books faster — SKYCOT",
  description:
    "Extract invoices, reconcile vendor payments, and flag expense anomalies. AI finance workers that close the books faster and catch the variance.",
  ogImage: "/og/og-finance.png",
  eyebrow: "AI for finance teams",
  headline: {
    lead: "Close the books. Catch the variance.",
    accent: "Sleep at night.",
  },
  subhead:
    "AI teammates that extract every invoice, reconcile vendor payments line-by-line, and flag the anomaly your spreadsheet would have missed. Fully audit-logged.",
  accent: "#10b981",
  workflowHeading: { lead: "Built for", accent: "finance teams" },
  workflows: [
    {
      title: "Extract invoices",
      outcome: "PDFs and emails parsed into your ledger with line-item accuracy.",
    },
    {
      title: "Reconcile vendor payments",
      outcome: "Bank lines matched to invoices, exceptions queued for your review.",
    },
    {
      title: "Flag expense anomalies",
      outcome: "Outliers surfaced against your team's historical pattern.",
    },
  ],
  signatureWorkflow: {
    name: "Invoice → ledger entry",
    blurb: "From inbox PDF to a posted ledger entry, fully audit-trailed.",
    steps: [
      { actor: "Trigger", action: "New invoice in finance inbox" },
      { actor: "Fen", action: "Extracts vendor · line items · totals" },
      { actor: "Fen", action: "Matches PO + flags mismatch (if any)" },
      { actor: "Approval", action: "AP clerk one-taps 'Post' on clean lines" },
      { actor: "Fen", action: "Posts to Xero · attaches PDF · logs hash" },
    ],
  },
  customerQuote: {
    text: "Fen extracts 1,200 invoices a month and closes our books five days faster. The audit chain alone paid for itself.",
    name: "Harper Chen",
    role: "Controller",
    company: "Maple Finance",
  },
  ctaStrip: {
    headline: "Hire your first finance teammate",
    sub: "Live in Slack in under four minutes. No credit card.",
  },
};

const MARKETING: VerticalConfig = {
  slug: "marketing",
  title: "AI marketing teammates — Always-on, never off-brand — SKYCOT",
  description:
    "Brand-voice QA on outgoing copy, lead enrichment from form fills, and weekly campaign digests. AI marketing workers that are always-on and never off-brand.",
  ogImage: "/og/og-marketing.png",
  eyebrow: "AI for marketing teams",
  headline: {
    lead: "Always-on campaigns,",
    accent: "never off-brand.",
  },
  subhead:
    "AI teammates that QA every outgoing piece against your brand voice, enrich the leads coming in, and pull a campaign digest before your Monday standup.",
  accent: "#ec4899",
  workflowHeading: { lead: "Built for", accent: "marketing teams" },
  workflows: [
    {
      title: "Brand-voice QA on outgoing copy",
      outcome: "Every draft scored against your tone guide before it ships.",
    },
    {
      title: "Lead enrichment from form fills",
      outcome: "New leads enriched, segmented, and synced to your CRM in seconds.",
    },
    {
      title: "Campaign performance digest",
      outcome: "Weekly digest that says what worked, what didn't, and why.",
    },
  ],
  signatureWorkflow: {
    name: "Draft → on-brand published copy",
    blurb: "How Mira QAs a marketing email before it goes out.",
    steps: [
      { actor: "Trigger", action: "Draft email queued in HubSpot" },
      { actor: "Mira", action: "Scores against tone guide + style rules" },
      { actor: "Mira", action: "Proposes edits inline · flags off-brand lines" },
      { actor: "Approval", action: "Marketer one-taps 'Accept' or 'Revise'" },
      { actor: "Mira", action: "Logs final version · sends to campaign" },
    ],
  },
  customerQuote: {
    text: "Mira reads every piece of copy we ship and tells us when it's off-brand. We caught a tone drift mid-campaign last week.",
    name: "Sofia Markov",
    role: "VP of Marketing",
    company: "Polaris Marketing",
  },
  ctaStrip: {
    headline: "Hire your first marketing teammate",
    sub: "Live in Slack in under four minutes. No credit card.",
  },
};

const OPS: VerticalConfig = {
  slug: "ops",
  title: "AI ops teammates — Run the back office on autopilot — SKYCOT",
  description:
    "Weekly status reports, cross-system data sync, and skill catalog audits. AI ops workers that run the back office on autopilot, audit-first.",
  ogImage: "/og/og-ops.png",
  eyebrow: "AI for ops teams",
  headline: { lead: "Run the back office on", accent: "autopilot." },
  subhead:
    "AI teammates that pull weekly status across every project, sync data between your CRM and ERP, and audit the skills your humans are spending time on.",
  accent: "#64748b",
  workflowHeading: { lead: "Built for", accent: "ops teams" },
  workflows: [
    {
      title: "Run weekly status reports",
      outcome: "Status pulled across Linear, Notion, and Slack into one digest.",
    },
    {
      title: "Sync data across CRM/ERP",
      outcome: "Records reconciled across Salesforce, NetSuite, and your warehouse.",
    },
    {
      title: "Audit your skill catalog",
      outcome: "Surface which skills are used, stale, or quietly broken.",
    },
  ],
  signatureWorkflow: {
    name: "Monday morning status digest",
    blurb: "How Patrick assembles your weekly ops digest by 8am Monday.",
    steps: [
      { actor: "Trigger", action: "Cron: Monday 06:30 local time" },
      { actor: "Patrick", action: "Pulls Linear · Notion · Slack updates" },
      { actor: "Patrick", action: "Diffs against last week · flags red rows" },
      { actor: "Approval", action: "Ops lead one-taps 'Publish' on digest" },
      { actor: "Patrick", action: "Posts to #weekly-status · logs trail" },
    ],
  },
  customerQuote: {
    text: "Patrick chases status across 14 engagements. He remembers what every project needs and never lets a red row hide.",
    name: "Mira Olsen",
    role: "Operations Director",
    company: "Meridian Logistics",
  },
  ctaStrip: {
    headline: "Hire your first ops teammate",
    sub: "Live in Slack in under four minutes. No credit card.",
  },
};

export const VERTICALS: Record<VerticalSlug, VerticalConfig> = {
  sales: SALES,
  recruiting: RECRUITING,
  support: SUPPORT,
  finance: FINANCE,
  marketing: MARKETING,
  ops: OPS,
};

export const VERTICAL_NAV: { slug: VerticalSlug; label: string; tagline: string }[] = [
  { slug: "sales", label: "Sales", tagline: "Qualify leads, book demos." },
  { slug: "recruiting", label: "Recruiting", tagline: "Screen, schedule, reference." },
  { slug: "support", label: "Support", tagline: "Triage tickets, draft replies." },
  { slug: "finance", label: "Finance", tagline: "Extract, reconcile, flag." },
  { slug: "marketing", label: "Marketing", tagline: "Brand QA, enrich, digest." },
  { slug: "ops", label: "Ops", tagline: "Status, sync, audit." },
];
