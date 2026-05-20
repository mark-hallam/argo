/**
 * Customer-story seed — feeds `/customers` index + `/customers/[slug]` detail.
 *
 * V1 ships 6 placeholder stories so the page reads believably until real
 * design-partner stories land. Every story carries `dataPending: true` so a
 * future bulk-replace pass can trivially distinguish seeded from real.
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
  /** Customer company name. */
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
  /** ID of the workflow template this customer runs — must exist in WORKFLOW_TEMPLATES. */
  workflowId: string;
  /** Short headline KPI for the index card, ≤ 64 chars. */
  cardKpi: string;
  /** Longer headline KPI for the detail-page hero strip. */
  headlineKpi: string;
  /** 1-2 sentence pull-quote shown on the index card. */
  cardQuote: string;
  /** Attribution for `cardQuote`, e.g. "Jane Doe, COO at Acme Co". */
  cardQuoteAuthor: string;
  /** Large editorial pull-quote on the detail page (longer than cardQuote). */
  heroPullQuote: string;
  /** Attribution for `heroPullQuote`. */
  heroQuoteAuthor: string;
  /** 3-4 paragraph "what changed" story, each paragraph a string. */
  setupParagraphs: string[];
  /** Exactly 3 KPI tiles for the metrics block on the detail page. */
  metrics: [CustomerMetric, CustomerMetric, CustomerMetric];
  /** Shorter quote from a different stakeholder (engineering / ops / CEO). */
  secondQuote: string;
  /** Attribution for `secondQuote`. */
  secondQuoteAuthor: string;
  /** Always `true` on seeded stories — flipped to false when real content lands. */
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
    logoGradient: "linear-gradient(135deg, #8b5cf6, #ec4899)",
    vertical: "Customer Support",
    workflowId: "support-triage-ticket",
    cardKpi: "73% of inbound tickets resolved without a human",
    headlineKpi: "73% of inbound tickets resolved without a human",
    cardQuote:
      "Mariah replied to 12,400 buyers our human team didn't have to. She knows when to escalate and when to ship the answer.",
    cardQuoteAuthor: "Priya Subramaniam, VP Support at Northwind",
    heroPullQuote:
      "We were drowning in post-purchase questions — order status, refund eligibility, shipping ETAs — the same six tickets, ten thousand times a month. Mariah handles 73% of them now, and the audit trail means we know exactly when she got it right and exactly when she escalated. The trust ratchet did what nothing else could: we finally trust the AI replies our customers see.",
    heroQuoteAuthor: "Priya Subramaniam, VP Support at Northwind Commerce",
    setupParagraphs: [
      "Northwind moved 4.2 million orders in 2025. Their human support team — 11 agents covering 16 hours a day — was buried under repetitive tickets. \"Where's my order?\" alone accounted for 38% of inbound volume. Macro replies helped, but customers could tell. Customer-satisfaction scores started slipping at the exact moment ticket volume was peaking.",
      "They evaluated four AI support platforms before choosing SKYCOT. The deal-closer wasn't accuracy — every vendor demoed accuracy. It was the approval-first posture: every escalation point is configurable, every action is audit-logged, and the trust ratchet means Mariah's autonomy grows only as her track record earns it.",
      "Onboarding took eleven days. The Northwind team plugged in their Zendesk webhook, mapped their order-management API as a tool, and loaded their knowledge base. Mariah started on a tight leash — every reply pre-approved by an agent. After two weeks of clean replies, the team flipped her to autonomous on six low-risk intents. After two months, that expanded to seventeen intents covering 73% of inbound.",
      "The unblock wasn't \"AI did the work.\" It was the audit chain — every reply traceable to a specific KB article, every decision logged against a specific intent classification, every escalation tagged with the reason. That is what made the support manager comfortable letting Mariah loose.",
    ],
    metrics: [
      { value: "12,400", label: "Tickets / month" },
      { value: "640 hrs", label: "Team time saved / mo" },
      { value: "73%", label: "Auto-resolved" },
    ],
    secondQuote:
      "I review the weekly Mariah-flagged escalations every Monday. It's the only thing my support manager and I agree on — those reviews stay short.",
    secondQuoteAuthor: "Dan Petrov, CEO at Northwind Commerce",
    dataPending: true,
  },
  {
    slug: "beacon-health-ops",
    name: "Beacon Health Ops",
    industry: "Healthcare",
    size: "1,200 employees",
    logoUrl: "/customers/beacon-health-ops.svg",
    logoInitials: "BH",
    logoGradient: "linear-gradient(135deg, #ec4899, #14b8a6)",
    vertical: "Patient Support",
    workflowId: "support-escalate-vip",
    cardKpi: "16,200 patient-portal tickets handled — every one audit-logged",
    headlineKpi: "16,200 patient-portal tickets handled — every action audit-logged",
    cardQuote:
      "HIPAA-compliant out of the box. Jamal handles patient-portal tickets and our compliance team finally relaxed.",
    cardQuoteAuthor: "Dr Mei Lin, Chief Compliance Officer at Beacon",
    heroPullQuote:
      "We are a regulated healthcare organization. The phrase \"AI handles patient communication\" should terrify anyone in our position — and it did, until we saw what audit-first actually means in practice. Every action Jamal takes is gated, logged, and traceable to a specific policy. Our compliance team didn't approve the deployment; they helped configure it.",
    heroQuoteAuthor: "Dr Mei Lin, Chief Compliance Officer at Beacon Health Ops",
    setupParagraphs: [
      "Beacon's patient portal generates ~22,000 messages a month: appointment changes, prescription refills, results questions, insurance queries. Their human team of 18 patient-services representatives could keep up — but only by triaging aggressively and letting non-urgent messages sit for 36+ hours. Patient experience scores reflected that.",
      "Healthcare AI deployments had been a non-starter for them three times before. The blocker was always the same: vendor demos showed accuracy, but couldn't produce the per-action audit chain HIPAA requires. \"Where's the log of who authorized this reply, on what evidence, with what policy version?\" — silence, or a vague pointer to a transcript log.",
      "SKYCOT was the first platform that handed Beacon's compliance team a sample audit row before the demo. Every Jamal action ships with: the policy version, the tenant, the actor (Jamal vs human override), the inputs, the redaction set applied, and a content hash. Beacon spent four weeks running Jamal in shadow mode — generating replies that humans graded — before flipping a single message live.",
      "Eight months in, Jamal handles 16,200 patient-portal tickets a month. Compliance audits are now a query against the Postgres audit-log table, not a fire drill.",
    ],
    metrics: [
      { value: "16,200", label: "Tickets / month" },
      { value: "890 hrs", label: "Team time saved / mo" },
      { value: "100%", label: "Audit-logged" },
    ],
    secondQuote:
      "Our SOC 2 auditor asked for a sample of AI-handled tickets with full evidence. We pulled it in three minutes. Last year that question took us three weeks.",
    secondQuoteAuthor: "Marcus Yates, Director of Engineering at Beacon Health Ops",
    dataPending: true,
  },
  {
    slug: "maple-finance",
    name: "Maple Finance",
    industry: "Financial Services",
    size: "85 employees · AFSL holder",
    logoUrl: "/customers/maple-finance.svg",
    logoInitials: "MF",
    logoGradient: "linear-gradient(135deg, #ff6b6b, #8b5cf6)",
    vertical: "Compliance Ops",
    workflowId: "compliance-evidence-collect",
    cardKpi: "AFSL audit prepared in 3 days — used to take 6 weeks",
    headlineKpi: "AFSL evidence pack assembled in 3 days. Previous cycle: 6 weeks.",
    cardQuote:
      "AFSL compliance was the deal-breaker. SKYCOT was the only platform that shipped with the audit chain regulators actually want.",
    cardQuoteAuthor: "Sandra Okafor, Head of Compliance at Maple Finance",
    heroPullQuote:
      "I am the person ASIC will speak to if something goes wrong. Every other AI vendor we evaluated had me defending decisions an opaque model made on data I couldn't reproduce. Tobias is different — he collects evidence on a schedule, every collection ties to a policy version, and the trust ratchet means he only acts inside the boundaries my compliance program defines.",
    heroQuoteAuthor: "Sandra Okafor, Head of Compliance at Maple Finance",
    setupParagraphs: [
      "Maple Finance holds an Australian Financial Services Licence and operates under continuous ASIC oversight. Their compliance program runs against a 142-item evidence checklist — controls testing, policy attestations, access reviews, transaction sampling — that previously required two full-time compliance analysts six weeks per cycle.",
      "Off-the-shelf AI tools failed every internal review. Generative outputs without provenance, no per-action audit chain, no concept of policy versioning — all the things a regulator cares about, none of the things AI vendors think about.",
      "SKYCOT's compliance evidence-collect template was the entry point. Tobias runs the 142-item evidence collection autonomously, attached to specific policy versions and source systems. Outputs go into a structured pack with content hashes. The human compliance analysts now spend their time on judgment calls — disputed attestations, exception triage — instead of running queries and pasting into spreadsheets.",
      "The first full cycle under Tobias landed the audit pack in 3 working days. The audit-trail completeness test passed on the first run.",
    ],
    metrics: [
      { value: "3 days", label: "Audit pack assembly" },
      { value: "260 hrs", label: "Compliance time saved / mo" },
      { value: "100%", label: "Evidence reproducibility" },
    ],
    secondQuote:
      "We brought a redacted Tobias audit row to ASIC as part of a routine licence review. They had follow-up questions. They were all easy.",
    secondQuoteAuthor: "Daniel Riggs, Managing Director at Maple Finance",
    dataPending: true,
  },
  {
    slug: "apex-recruiting",
    name: "Apex Recruiting",
    industry: "Talent / HR Tech",
    size: "32 employees",
    logoUrl: "/customers/apex-recruiting.svg",
    logoInitials: "AR",
    logoGradient: "linear-gradient(135deg, #8b5cf6, #14b8a6)",
    vertical: "Recruiting",
    workflowId: "people-screen-candidate",
    cardKpi: "200 candidates / week screened — 8 surface to a human",
    headlineKpi: "200 candidates screened weekly. 8 escalate. Recruiter time spent on the 8.",
    cardQuote:
      "Priya screens 200 candidates a week and flags the 8 we should actually talk to. The trust ratchet means she only escalates close calls.",
    cardQuoteAuthor: "Liam Cheng, Founder at Apex Recruiting",
    heroPullQuote:
      "Recruiting is a screening problem disguised as a talent problem. We were paying senior recruiters to read 192 resumes that didn't need reading, so they could find the 8 that did. Priya inverted that — she handles the 192, and our humans get to do the work humans are good at: judgment calls on the close ones.",
    heroQuoteAuthor: "Liam Cheng, Founder at Apex Recruiting",
    setupParagraphs: [
      "Apex Recruiting fills senior technical roles for series-B and -C startups. Their screening pipeline historically ran ~200 candidates a week per recruiter — high volume, narrow funnel, low conversion to phone-screen. Recruiters reported that 60% of their week was triage they could describe as \"obvious yes or obvious no.\"",
      "They tried two AI resume-screeners and abandoned both inside 30 days. One had no concept of bias mitigation. The other had no audit trail. Both shipped \"decisions\" the recruiters couldn't defend to candidates or clients.",
      "Priya runs the people-screen-candidate workflow. She scores against the role-specific rubric the recruiter authored, flags every candidate she rejects with a structured reason code, and asks for human review on any score within the close-call band. The bias-mitigation skill is a workflow step the recruiter can audit — not a black box.",
      "Eight months in, Priya handles 192-194 of the weekly 200, and the recruiters spend their time on the 6-8 close calls. Phone-screen-to-offer rate doubled.",
    ],
    metrics: [
      { value: "200", label: "Candidates screened / wk" },
      { value: "410 hrs", label: "Recruiter time saved / mo" },
      { value: "2×", label: "Phone-to-offer rate" },
    ],
    secondQuote:
      "Every screening decision Priya makes has a structured reason code. When a candidate asks for feedback, we send the structured reason. Nobody has ever called us black-box.",
    secondQuoteAuthor: "Jana Pillai, Head of Recruiting Operations at Apex",
    dataPending: true,
  },
  {
    slug: "skyline-engineering",
    name: "Skyline Engineering",
    industry: "AEC / Construction",
    size: "240 employees · 18 active sites",
    logoUrl: "/customers/skyline-engineering.svg",
    logoInitials: "SE",
    logoGradient: "linear-gradient(135deg, #0891b2, #14b8a6)",
    vertical: "Status Reporting",
    workflowId: "marketing-draft-status-report",
    cardKpi: "Weekly client updates drafted across 18 sites — 1 afternoon back",
    headlineKpi: "18 sites, 18 weekly client updates, drafted before Monday morning",
    cardQuote:
      "Nori drafts our weekly client updates. We review, she sends. Saves the senior engineers a full afternoon each week.",
    cardQuoteAuthor: "Hamish Bell, Director of Engineering at Skyline",
    heroPullQuote:
      "Senior engineers are the most expensive minute in our business and we were spending half a day a week of theirs writing status updates clients largely skim. Nori reads the same source-of-truth we did — daily logs, RFI tracker, schedule milestones — and writes the same report in a tenth of the time. Engineers review and edit. The client doesn't notice the difference, but our utilisation rate did.",
    heroQuoteAuthor: "Hamish Bell, Director of Engineering at Skyline Engineering",
    setupParagraphs: [
      "Skyline runs 18 active construction sites at any given time, each with a senior engineer responsible for a weekly client update. The update reads from daily logs, the RFI (request-for-information) tracker, schedule milestones, and safety incident counts. Writing one well takes a senior engineer about 90 minutes. 18 sites × 90 minutes × every week = 27 hours of senior time per week on the same shaped task.",
      "Skyline tried generic LLM tools twice — once with ChatGPT pasted into a template, once with a custom prompt-chain. Both produced reports that read like brochures: plausible, generic, occasionally wrong. The engineers spent more time correcting than writing.",
      "Nori is different in two ways. First, she's tool-connected — she reads the actual daily-log database and the RFI tracker, not engineer summaries of them. Second, every report is reviewed and signed off by the responsible engineer before send. The senior engineer's role moved from writer to editor.",
      "Six months in: 18 weekly reports drafted automatically every Monday by 7am. Engineers spend ~10 minutes each on review. Net: ~25 hours of senior engineering time returned per week.",
    ],
    metrics: [
      { value: "18", label: "Weekly reports drafted" },
      { value: "180 hrs", label: "Senior time saved / mo" },
      { value: "10 min", label: "Per-report review time" },
    ],
    secondQuote:
      "I used to dread Monday morning. Now I scan Nori's drafts on the commute, mark anything I want changed, and they go out before standup.",
    secondQuoteAuthor: "Priya Anand, Senior Engineer at Skyline",
    dataPending: true,
  },
  {
    slug: "polaris-marketing",
    name: "Polaris Marketing",
    industry: "Agency / Marketing",
    size: "60 employees · 40 client engagements",
    logoUrl: "/customers/polaris-marketing.svg",
    logoInitials: "PM",
    logoGradient: "linear-gradient(135deg, #14b8a6, #8b5cf6)",
    vertical: "Project Coordination",
    workflowId: "marketing-campaign-perf-digest",
    cardKpi: "40 client engagements coordinated — nothing slips through",
    headlineKpi: "40 active client engagements coordinated. Zero follow-ups missed last quarter.",
    cardQuote:
      "Peta coordinates 14 client engagements at once. She remembers what every project needs and chases the right person — without ever forgetting.",
    cardQuoteAuthor: "Sam Tukoroirangi, MD at Polaris Marketing",
    heroPullQuote:
      "Agency work is 80% remembering. Who owes what, when it's due, who hasn't replied to the third follow-up, which campaign is waiting on which sign-off. We hired four humans for this in the last two years and lost three of them to burnout. Peta does the remembering. The humans get to do the work that pays.",
    heroQuoteAuthor: "Sam Tukoroirangi, Managing Director at Polaris Marketing",
    setupParagraphs: [
      "Polaris runs 40 concurrent client engagements across paid media, content, and brand strategy. Each engagement is a graph of tasks, approvals, deliverables, and review cycles. They had a project coordinator team of three. Two left in 18 months. The work — relentless, low-glamour, but business-critical — burnt them out.",
      "They evaluated three AI project-management tools before SKYCOT. All three were schedulers. None of them did the actual coordinator work — chasing replies, surfacing blockers, prepping status updates, escalating risks. The blocker on those tools wasn't intelligence; it was integration. They couldn't see the email threads, the Slack channels, the campaign performance dashboards. They could only see the calendar.",
      "Peta runs across the agency's stack — email, Slack, Asana, the campaign-perf-digest template fed by ad-platform connectors. She maintains a working memory of every engagement, knows what each client is waiting on, drafts the follow-ups, and surfaces what's blocked. The humans coordinate the things that need judgment.",
      "Net result: 40 engagements coordinated by one human PM + Peta, where it used to require three humans. The remaining human coordinator is the one of the three originals who stayed — and she now describes the job as \"actually doable.\"",
    ],
    metrics: [
      { value: "40", label: "Client engagements" },
      { value: "520 hrs", label: "Coordinator time saved / mo" },
      { value: "0", label: "Follow-ups missed last qtr" },
    ],
    secondQuote:
      "She doesn't forget. That's the whole job description. She doesn't forget.",
    secondQuoteAuthor: "Carlos Méndez, Senior PM at Polaris Marketing",
    dataPending: true,
  },
];

/** Look up a story by slug. Returns undefined if no match. */
export function getCustomerStory(slug: string): CustomerStory | undefined {
  return SEED_CUSTOMERS.find((c) => c.slug === slug);
}
