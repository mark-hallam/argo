/**
 * Customer-scenario seed — feeds `/customers` index + `/customers/[slug]` detail.
 *
 * These are REPRESENTATIVE DEPLOYMENT SCENARIOS, not real named customers.
 * They describe, in third person, what a SKYCOT deployment looks like for a
 * given vertical — which teammate, which workflow, which approval posture —
 * using current workflow-template data. Honesty rules (shakedown F3, brought
 * to the `/security` standard):
 *
 *   - NO fabricated person-quotes. There are no named people endorsing
 *     SKYCOT here, and nothing on the page is styled as a testimonial with a
 *     "— Name, Title at Company" attribution.
 *   - NO compliance claim voiced by a fictional person. Any compliance
 *     statement lives in `compliancePosture`, written in SKYCOT's OWN voice
 *     with `/security`-grade honesty (no aspirational claims, no certification
 *     asserted on a customer's behalf).
 *   - Metrics are illustrative MODELED OUTCOMES, labelled as such — not
 *     measured results from a real account.
 *
 * Every story carries `dataPending: true` so a future bulk-replace pass can
 * trivially distinguish seeded scenarios from real, cleared case studies.
 *
 * The `workflowId` on each story points at a real entry in
 * `web/lib/workflow-templates.ts`, so the detail-page workflow showcase
 * renders a real step list from existing template data — no parallel
 * workflow source-of-truth.
 */

export interface CustomerMetric {
  /** large number / phrase shown on the metric tile, e.g. "12,400". */
  value: string;
  /** SHORT all-caps label below the number, e.g. "Actions/month". */
  label: string;
}

export interface CustomerStory {
  /** kebab-case stable identifier — also the route segment. */
  slug: string;
  /** Scenario company name (illustrative, not a real account). */
  name: string;
  /** Industry tag for the index card, e.g. "E-commerce". */
  industry: string;
  /** Size descriptor, e.g. "200 employees" or "45 stores · 380 staff". */
  size: string;
  /** Public path to the logo asset under `web/public/customers/`. */
  logoUrl: string;
  /** 2-letter fallback initials used in the gradient tile if no SVG present. */
  logoInitials: string;
  /** CSS background for the logo tile (gradient string). */
  logoGradient: string;
  /** Vertical chip on the card, e.g. "Customer Support". */
  vertical: string;
  /** ID of the workflow template this scenario runs — must exist in WORKFLOW_TEMPLATES. */
  workflowId: string;
  /** Short modeled-outcome headline for the index card, ≤ 64 chars. */
  cardKpi: string;
  /** Longer modeled-outcome headline for the detail-page hero strip. */
  headlineKpi: string;
  /**
   * Third-person SCENARIO line shown on the index card. Describes what the
   * deployment looks like. NEVER a person-quote and NEVER an attribution.
   * (Field name retained for the `/landing` import; it is not a quote.)
   */
  cardQuote: string;
  /** Longer third-person scenario description on the detail page. */
  heroPullQuote: string;
  /** 3-4 paragraph "what the deployment looks like" narrative, third person. */
  setupParagraphs: string[];
  /** Exactly 3 KPI tiles — illustrative modeled outcomes, not measured results. */
  metrics: [CustomerMetric, CustomerMetric, CustomerMetric];
  /** Third-person "what this profile demonstrates" note (no attribution). */
  secondQuote: string;
  /**
   * Optional compliance statement in SKYCOT's OWN voice, `/security`-grade:
   * no aspirational claims, no certification asserted on a customer's behalf.
   * Used for regulated verticals (healthcare, financial services). When a line
   * names a framework such as HIPAA it carries explicit posture framing.
   */
  compliancePosture?: string;
  /** Always `true` on seeded scenarios — flipped to false when real content lands. */
  dataPending: true;
}

export const SEED_CUSTOMERS: CustomerStory[] = [
  {
    slug: "northwind-commerce",
    name: "Northwind Commerce",
    industry: "E-commerce",
    size: "120 employees",
    logoUrl: "/customers/northwind-commerce.svg",
    logoInitials: "NC",
    logoGradient: "linear-gradient(135deg, var(--brand-primary), var(--brand-pink))",
    vertical: "Customer Support",
    workflowId: "support-triage-ticket",
    cardKpi: "Modeled: most repetitive tickets resolved without a human",
    headlineKpi: "Modeled outcome: the repetitive majority of inbound tickets resolved without a human",
    cardQuote:
      "In this profile, Mariah handles post-purchase support — order status, refunds, shipping ETAs — resolving the repetitive majority autonomously and escalating the judgment calls to the human team.",
    heroPullQuote:
      "This profile models a high-volume e-commerce support desk where the same handful of questions arrive thousands of times a month. Mariah is configured to answer the repetitive intents under an approval-first posture, escalate anything outside her trusted set, and write every reply to the audit chain — so the team can see exactly when she resolved a ticket and exactly when she handed it off.",
    setupParagraphs: [
      "Picture a mid-size online retailer moving millions of orders a year. A human support team of around a dozen agents, covering extended hours, is buried under repetitive tickets. \"Where's my order?\" alone can account for a third of inbound volume. Macro replies help, but customers can tell, and satisfaction scores tend to slip exactly when ticket volume peaks.",
      "What distinguishes a SKYCOT deployment here is not raw accuracy — every vendor demos accuracy. It is the approval-first posture: every escalation point is configurable, every action is audit-logged, and the trust ratchet means Mariah's autonomy grows only as her track record earns it.",
      "A typical rollout starts on a tight leash: every reply pre-approved by an agent. After a stretch of clean replies, the team flips Mariah to autonomous on a few low-risk intents, then expands intent-by-intent as the audit trail earns trust — rather than switching full autonomy on all at once.",
      "The unblock in this scenario is not \"AI did the work.\" It is the audit chain — every reply traceable to a specific knowledge-base article, every decision logged against a specific intent classification, every escalation tagged with its reason. That is what makes a support manager comfortable widening the teammate's remit.",
    ],
    metrics: [
      { value: "12,400", label: "Tickets / month (modeled)" },
      { value: "640 hrs", label: "Team time saved / mo (modeled)" },
      { value: "73%", label: "Auto-resolved (modeled)" },
    ],
    secondQuote:
      "What this profile shows: an approval-first deployment where autonomy expands intent-by-intent as the audit trail earns trust, rather than being switched on all at once.",
    dataPending: true,
  },
  {
    slug: "beacon-health-ops",
    name: "Beacon Health Ops",
    industry: "Healthcare",
    size: "1,200 employees",
    logoUrl: "/customers/beacon-health-ops.svg",
    logoInitials: "BH",
    logoGradient: "linear-gradient(135deg, var(--brand-pink), var(--brand-coral))",
    vertical: "Patient Support",
    workflowId: "support-escalate-vip",
    cardKpi: "Modeled: patient-portal tickets handled — every action audit-logged",
    headlineKpi: "Modeled outcome: high patient-portal volume handled — every action audit-logged",
    cardQuote:
      "In this profile, Jamal handles patient-portal messages — appointment changes, refill requests, results questions — with every action gated, logged, and traceable to a specific policy version.",
    heroPullQuote:
      "This profile models a regulated healthcare support deployment. Jamal is configured to run patient-portal triage under tight approval gates, with every action carrying its policy version, tenant, actor, inputs, applied redactions, and a content hash — so a compliance review is a query against the audit log rather than a reconstruction after the fact.",
    setupParagraphs: [
      "Picture a healthcare provider whose patient portal generates tens of thousands of messages a month: appointment changes, prescription refills, results questions, insurance queries. A human team can keep up only by triaging aggressively and letting non-urgent messages sit for a day or more — and patient-experience scores reflect that.",
      "Healthcare AI deployments stall when a vendor can show accuracy but cannot produce the per-action audit chain a regulated deployment requires. \"Where is the log of who authorized this reply, on what evidence, with what policy version?\" is the question that ends most evaluations.",
      "A SKYCOT deployment answers that question before the demo: every action Jamal takes ships with its policy version, the tenant, the actor (Jamal vs a human override), the inputs, the redaction set applied, and a content hash. A common pattern is to run the teammate in shadow mode first — generating replies humans grade — before a single message goes live.",
      "In this profile, Jamal handles on the order of 16,000 patient-portal tickets a month, and a compliance audit becomes a query against the audit-log table rather than a fire drill.",
    ],
    metrics: [
      { value: "16,200", label: "Tickets / month (modeled)" },
      { value: "890 hrs", label: "Team time saved / mo (modeled)" },
      { value: "100%", label: "Audit-logged (by design)" },
    ],
    secondQuote:
      "What this profile shows: an audit-first deployment where the evidence for every AI-handled message is reproducible on demand.",
    compliancePosture:
      "SKYCOT's posture on regulated workloads (for example HIPAA): we ship a per-action, hash-chained audit trail and configurable approval gates designed to support a customer's own compliance program. SKYCOT does not assert that any deployment is automatically compliant — compliance depends on how the customer configures controls, access, and data handling. See /security for the claims we do and do not make.",
    dataPending: true,
  },
  {
    slug: "maple-finance",
    name: "Maple Finance",
    industry: "Financial Services",
    size: "85 employees · AFSL holder",
    logoUrl: "/customers/maple-finance.svg",
    logoInitials: "MF",
    logoGradient: "linear-gradient(135deg, var(--brand-coral), var(--brand-primary))",
    vertical: "Compliance Ops",
    workflowId: "compliance-evidence-collect",
    cardKpi: "Modeled: evidence pack assembled in days, not weeks",
    headlineKpi: "Modeled outcome: an evidence pack assembled in days where a manual cycle took weeks",
    cardQuote:
      "In this profile, Tobias runs a recurring compliance evidence-collection workflow against a large control checklist, attaching every collected item to a specific policy version and source system.",
    heroPullQuote:
      "This profile models a financial-services compliance team that previously spent weeks per cycle assembling an evidence pack by hand. Tobias is configured to collect evidence on a schedule, tie each collection to a policy version, and act only inside the boundaries the compliance program defines — leaving the human analysts to handle the judgment calls.",
    setupParagraphs: [
      "Picture a licensed financial-services firm operating under continuous regulatory oversight. Its compliance program runs against a long evidence checklist — controls testing, policy attestations, access reviews, transaction sampling — that can require analysts weeks per cycle.",
      "Off-the-shelf AI tools tend to fail this internal review: generative outputs without provenance, no per-action audit chain, no concept of policy versioning — the things a regulator cares about, and the things general-purpose AI tools rarely address.",
      "In a SKYCOT deployment, the compliance evidence-collect template is the entry point. Tobias runs the evidence collection on a schedule, attached to specific policy versions and source systems, and writes outputs into a structured pack with content hashes. The human analysts spend their time on judgment calls — disputed attestations, exception triage — instead of running queries and pasting into spreadsheets.",
      "In this profile, a full cycle under Tobias lands the audit pack in a few working days, with an audit-trail-completeness check that passes on the first run.",
    ],
    metrics: [
      { value: "3 days", label: "Audit pack assembly (modeled)" },
      { value: "260 hrs", label: "Compliance time saved / mo (modeled)" },
      { value: "100%", label: "Evidence reproducibility (by design)" },
    ],
    secondQuote:
      "What this profile shows: a deployment where evidence is reproducible and every collection step is traceable to its source and its policy version.",
    compliancePosture:
      "SKYCOT's posture for regulated financial services such as AFSL holders: we provide reproducible, hash-chained evidence and policy-versioned actions intended to support a licensee's own compliance obligations. SKYCOT makes no standalone regulatory-compliance claim on a customer's behalf. See /security.",
    dataPending: true,
  },
  {
    slug: "apex-recruiting",
    name: "Apex Recruiting",
    industry: "Talent / HR Tech",
    size: "32 employees",
    logoUrl: "/customers/apex-recruiting.svg",
    logoInitials: "AR",
    logoGradient: "linear-gradient(135deg, var(--brand-primary), var(--brand-pink))",
    vertical: "Recruiting",
    workflowId: "people-screen-candidate",
    cardKpi: "Modeled: high weekly screening volume, a handful surfaced to a human",
    headlineKpi: "Modeled outcome: most weekly candidates screened automatically; recruiters spend their time on the close calls",
    cardQuote:
      "In this profile, Priya screens the weekly inbound resume volume against a recruiter-authored rubric, flags every rejection with a structured reason code, and routes the close calls to a human.",
    heroPullQuote:
      "This profile models a high-volume technical recruiting funnel where most of a recruiter's week goes to obvious-yes / obvious-no triage. Priya is configured to score against the role rubric, attach an auditable reason code to every decision, and ask for human review on any score inside the close-call band.",
    setupParagraphs: [
      "Picture a recruiting firm filling senior technical roles. Its screening pipeline runs a high volume of candidates per recruiter each week — high volume, narrow funnel, low conversion to phone-screen — and recruiters report that much of their week is triage they would describe as \"obvious yes or obvious no.\"",
      "Generic AI resume-screeners tend to get abandoned quickly: one with no concept of bias mitigation, another with no audit trail. Both ship \"decisions\" recruiters cannot defend to candidates or clients.",
      "In a SKYCOT deployment, Priya runs the people-screen-candidate workflow. She scores against the role-specific rubric the recruiter authored, flags every candidate she rejects with a structured reason code, and asks for human review on any score within the close-call band. The bias-mitigation skill is a workflow step the recruiter can audit — not a black box.",
      "In this profile, Priya handles the large majority of the weekly volume, and recruiters spend their time on the close calls — the work humans are good at.",
    ],
    metrics: [
      { value: "200", label: "Candidates screened / wk (modeled)" },
      { value: "410 hrs", label: "Recruiter time saved / mo (modeled)" },
      { value: "2×", label: "Phone-to-offer rate (modeled)" },
    ],
    secondQuote:
      "What this profile shows: a screening deployment where every decision carries a structured, defensible reason code rather than a black-box verdict.",
    dataPending: true,
  },
  {
    slug: "skyline-engineering",
    name: "Skyline Engineering",
    industry: "AEC / Construction",
    size: "240 employees · 18 active sites",
    logoUrl: "/customers/skyline-engineering.svg",
    logoInitials: "SE",
    logoGradient: "linear-gradient(135deg, var(--brand-pink), var(--brand-coral))",
    vertical: "Status Reporting",
    workflowId: "marketing-draft-status-report",
    cardKpi: "Modeled: weekly client updates drafted across every site",
    headlineKpi: "Modeled outcome: a weekly client update drafted for every active site before Monday morning",
    cardQuote:
      "In this profile, Nori drafts the weekly client update for each active site by reading the underlying daily logs, RFI tracker, and schedule milestones — the responsible engineer reviews and sends.",
    heroPullQuote:
      "This profile models an engineering firm whose senior staff spend hours each week writing status updates clients largely skim. Nori is configured to read the same source systems the engineers do — daily logs, the RFI tracker, schedule milestones — and draft each report, leaving the engineer as reviewer-and-approver rather than author.",
    setupParagraphs: [
      "Picture an engineering firm running many active construction sites at once, each with a senior engineer responsible for a weekly client update. The update reads from daily logs, the request-for-information (RFI) tracker, schedule milestones, and safety-incident counts. Writing one well takes a senior engineer well over an hour — multiplied across every site, every week.",
      "Generic LLM tools — a chatbot pasted into a template, or a custom prompt-chain — tend to produce reports that read like brochures: plausible, generic, occasionally wrong. Engineers spend more time correcting than they would writing.",
      "A SKYCOT deployment differs in two ways. First, Nori is tool-connected — she reads the actual daily-log database and the RFI tracker, not engineer summaries of them. Second, every report is reviewed and signed off by the responsible engineer before send. The senior engineer's role moves from writer to editor.",
      "In this profile, every site's weekly report is drafted automatically by early Monday, engineers spend a few minutes each on review, and a meaningful block of senior engineering time is returned each week.",
    ],
    metrics: [
      { value: "18", label: "Weekly reports drafted (modeled)" },
      { value: "180 hrs", label: "Senior time saved / mo (modeled)" },
      { value: "10 min", label: "Per-report review time (modeled)" },
    ],
    secondQuote:
      "What this profile shows: a tool-connected drafting deployment where the human stays the sign-off and the expensive writing step is automated against real source data.",
    dataPending: true,
  },
  {
    slug: "polaris-marketing",
    name: "Polaris Marketing",
    industry: "Agency / Marketing",
    size: "60 employees · 40 client engagements",
    logoUrl: "/customers/polaris-marketing.svg",
    logoInitials: "PM",
    logoGradient: "linear-gradient(135deg, var(--brand-coral), var(--brand-primary))",
    vertical: "Project Coordination",
    workflowId: "marketing-campaign-perf-digest",
    cardKpi: "Modeled: dozens of client engagements coordinated, nothing slipping",
    headlineKpi: "Modeled outcome: many active client engagements coordinated; follow-ups tracked rather than dropped",
    cardQuote:
      "In this profile, Peta coordinates dozens of concurrent client engagements — tracking what each is waiting on, drafting follow-ups, and surfacing blockers for the human PM to resolve.",
    heroPullQuote:
      "This profile models an agency where most coordination work is remembering: who owes what, what's overdue, which sign-off is blocking which campaign. Peta is configured to run across the agency's email, chat, and project tooling, hold a working memory of every engagement, and surface the judgment calls to a human coordinator.",
    setupParagraphs: [
      "Picture a marketing agency running dozens of concurrent client engagements across paid media, content, and brand strategy. Each engagement is a graph of tasks, approvals, deliverables, and review cycles. The coordinator role is relentless and low-glamour but business-critical — and a common place for burnout.",
      "Generic AI project-management tools tend to be schedulers. They don't do the actual coordinator work — chasing replies, surfacing blockers, prepping status updates, escalating risks — because they can only see the calendar, not the email threads, chat channels, and performance dashboards where the work actually lives.",
      "In a SKYCOT deployment, Peta runs across the agency's stack — email, chat, project tooling, and the campaign-perf-digest template fed by ad-platform connectors. She maintains a working memory of every engagement, knows what each client is waiting on, drafts the follow-ups, and surfaces what's blocked. The humans coordinate the things that need judgment.",
      "In this profile, the engagements are coordinated by one human PM plus Peta where it would previously have taken several people — with the repetitive chase-and-remind work automated.",
    ],
    metrics: [
      { value: "40", label: "Client engagements (modeled)" },
      { value: "520 hrs", label: "Coordinator time saved / mo (modeled)" },
      { value: "0", label: "Follow-ups missed (modeled)" },
    ],
    secondQuote:
      "What this profile shows: a coordination deployment where the repetitive chase-and-remind work is automated and the human owns the judgment calls.",
    dataPending: true,
  },
];

/** Look up a story by slug. Returns undefined if no match. */
export function getCustomerStory(slug: string): CustomerStory | undefined {
  return SEED_CUSTOMERS.find((c) => c.slug === slug);
}
