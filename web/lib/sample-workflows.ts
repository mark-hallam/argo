/**
 * Seed workflows for the `/workflows` catalog + visual viewer.
 *
 * Used as fallback when the gateway returns no live data (offline dev,
 * 404 on `/verticals`, demo recordings against a cold store). The
 * `useWorkflows` hook marks results sourced from here with
 * `dataSource: "mock"` so the UI can render a "MOCK" pill.
 *
 * Shape matches `WorkflowSummary` / `WorkflowSpec` in
 * `lib/api/workflows.ts` so the swap from live → mock is mechanical.
 * `httpRequest` / `code` config re-uses the canonical builder types
 * so sample → builder hand-off is exact (round-trip in tests).
 */
import type {
  CodeConfig,
  HttpRequestConfig,
} from "@/app/(app)/workflows/[verticalId]/[name]/build/types";
// Type-only import; the runtime side of `api/workflows.ts` imports
// from this file, so we keep this strictly type-level to avoid a
// runtime cycle. Substrate stays canonically defined in
// `lib/api/workflows.ts` because the SubstrateChip component and 9
// other call-sites already import it from there.
import type { WorkflowSubstrate } from "@/lib/api/workflows";

export type WorkflowStepKind =
  | "trigger"
  | "skill_invoke"
  | "approval"
  | "tool_call"
  | "notify"
  | "decision"
  | "data"
  | "http_request"
  | "code";

export interface WorkflowStep {
  id: string;
  kind: WorkflowStepKind;
  name: string;
  description: string;
  approvalGate?: { reason: string; channel: string };
  inputs?: string[];
  outputs?: string[];
  httpRequest?: HttpRequestConfig;
  code?: CodeConfig;
}

export type WorkflowRisk = "approval-required" | "autonomous";

/**
 * Who last edited the workflow. Discriminant so the card can show a
 * Bot / User / Layers icon next to "by <name>" — Scene 5 of the v1
 * demo narrates "workflows can be edited by the AI teammate themself",
 * which needs a visual anchor on the card.
 */
export type WorkflowAuthorType = "worker" | "human" | "template_pack";

/**
 * Canonical worker roster used to classify `lastEditor` strings. New
 * worker names go here so fixtures + gateway-derived rows pick up the
 * `worker` discriminant automatically. `workerId` is the canonical
 * `/workers/<id>` slug; left as `null` when no real worker page exists
 * yet (Maya / Patrick aren't in `sample-personas.ts`, so the chip
 * renders without click-through for those).
 */
export const CANONICAL_WORKER_ROSTER: ReadonlyArray<{
  name: string;
  workerId: string | null;
}> = [
  { name: "Mariah", workerId: "019e6785-928a-7270-a766-f945b6cd7161" },
  { name: "Priya", workerId: "019e6785-928d-78b1-a6b2-136d9ed263ce" },
  { name: "Jamal", workerId: "019e6785-928e-7f20-803b-55266e526011" },
  { name: "Nori", workerId: "019e6785-928f-7281-bf8d-e062bc277478" },
  { name: "Peta", workerId: "019e6785-928c-72b3-85c6-436fe5004c50" },
  { name: "Maya", workerId: null },
  { name: "Patrick", workerId: null },
];

const TEMPLATE_PACK_AUTHORS = new Set(["SKYCOT", "system"]);

/**
 * Derive `authorType` + optional `authorWorkerId` from a `lastEditor`
 * name. Pure — no I/O — so callers (fixtures + gateway adapters) can
 * share one classification path.
 */
export function classifyAuthor(lastEditor: string): {
  authorType: WorkflowAuthorType;
  authorWorkerId?: string;
} {
  const trimmed = (lastEditor ?? "").trim();
  if (!trimmed) return { authorType: "human" };
  const match = CANONICAL_WORKER_ROSTER.find(
    (w) => w.name.toLowerCase() === trimmed.toLowerCase(),
  );
  if (match) {
    return match.workerId
      ? { authorType: "worker", authorWorkerId: match.workerId }
      : { authorType: "worker" };
  }
  if (TEMPLATE_PACK_AUTHORS.has(trimmed)) return { authorType: "template_pack" };
  return { authorType: "human" };
}

export interface WorkflowSummary {
  name: string;
  vertical: string;
  description: string;
  steps: number;
  versions: number;
  lastEdited: string;
  lastEditor: string;
  /** Discriminant on `lastEditor` — workers, humans, and template packs share name-space. */
  authorType: WorkflowAuthorType;
  /** UUID for `/workers/<id>` click-through. Set only when `authorType === "worker"` AND a worker page exists. */
  authorWorkerId?: string;
  risk: WorkflowRisk;
  substrate: WorkflowSubstrate;
}

export interface WorkflowSpec {
  name: string;
  vertical: string;
  description: string;
  version: string;
  steps: WorkflowStep[];
  risk: WorkflowRisk;
  substrate: WorkflowSubstrate;
}

export const SAMPLE_WORKFLOW_SUMMARIES: WorkflowSummary[] = [
  {
    name: "post_listing",
    vertical: "gtm-engineer",
    description: "Validate inventory data + post fresh listing to Facebook Marketplace.",
    steps: 6,
    versions: 3,
    lastEdited: "2 days ago",
    lastEditor: "Mariah",
    authorType: "worker",
    authorWorkerId: "019e6785-928a-7270-a766-f945b6cd7161",
    risk: "approval-required",
    substrate: "headless",
  },
  {
    name: "screen_candidate",
    vertical: "people-ops",
    description: "First-pass candidate screen against a JD with skill-match scoring.",
    steps: 8,
    versions: 5,
    lastEdited: "4 hours ago",
    lastEditor: "Priya",
    authorType: "worker",
    authorWorkerId: "019e6785-928d-78b1-a6b2-136d9ed263ce",
    risk: "autonomous",
    substrate: "headless",
  },
  {
    name: "approve_refund",
    vertical: "support",
    description: "Verify refund eligibility, check ledger, route high-value to manager.",
    steps: 5,
    versions: 2,
    lastEdited: "1 week ago",
    lastEditor: "Jamal",
    authorType: "worker",
    authorWorkerId: "019e6785-928e-7f20-803b-55266e526011",
    risk: "approval-required",
    substrate: "headless",
  },
  {
    name: "qualify_lead",
    vertical: "sales",
    description:
      "Budget / Authority / Need / Timeline qualification + calendar-link insertion + CRM update.",
    steps: 7,
    versions: 4,
    lastEdited: "3 days ago",
    lastEditor: "Mariah",
    authorType: "worker",
    authorWorkerId: "019e6785-928a-7270-a766-f945b6cd7161",
    risk: "approval-required",
    substrate: "headless",
  },
  {
    name: "triage_ticket",
    vertical: "support",
    description: "Classify tickets, deflect with KB articles, escalate the rest.",
    steps: 3,
    versions: 6,
    lastEdited: "32 minutes ago",
    lastEditor: "Jamal",
    authorType: "worker",
    authorWorkerId: "019e6785-928e-7f20-803b-55266e526011",
    risk: "autonomous",
    substrate: "headless",
  },
  {
    name: "morning_pipeline_digest",
    vertical: "sales",
    description:
      "Pull yesterday's deals from the CRM via API, summarise the headline numbers, post to Slack.",
    steps: 3,
    versions: 2,
    lastEdited: "1 hour ago",
    lastEditor: "Peta",
    authorType: "worker",
    authorWorkerId: "019e6785-928c-72b3-85c6-436fe5004c50",
    risk: "autonomous",
    substrate: "headless",
  },
  {
    name: "clean_customer_list",
    vertical: "finance",
    description:
      "Take a messy CSV of customer names, run a short JavaScript step to title-case and dedupe, write the cleaned file back.",
    steps: 3,
    versions: 1,
    lastEdited: "20 minutes ago",
    lastEditor: "Nori",
    authorType: "worker",
    authorWorkerId: "019e6785-928f-7281-bf8d-e062bc277478",
    risk: "autonomous",
    substrate: "headless",
  },
  {
    name: "weekly_standup_digest",
    vertical: "project-ops",
    description:
      "Pull last week's Linear + GitHub activity per teammate, draft a 5-bullet standup digest, post to Slack #standup.",
    steps: 4,
    versions: 2,
    lastEdited: "2 days ago",
    lastEditor: "Peta",
    authorType: "worker",
    authorWorkerId: "019e6785-928c-72b3-85c6-436fe5004c50",
    risk: "autonomous",
    substrate: "headless",
  },
  {
    name: "customer_research_followup",
    vertical: "sales",
    description:
      "After a discovery call, capture call notes into the macOS Notes app, summarise next steps, and DM the AE.",
    steps: 4,
    versions: 1,
    lastEdited: "12 minutes ago",
    lastEditor: "Mariah",
    risk: "autonomous",
    substrate: "macos",
  },
  {
    name: "competitor_pricing_scrape",
    vertical: "gtm-engineer",
    description:
      "Drive a real browser via Playwright to compare competitor pricing pages, flag deltas, post the diff to Slack.",
    steps: 5,
    versions: 2,
    lastEdited: "45 minutes ago",
    lastEditor: "Priya",
    risk: "approval-required",
    substrate: "browser",
  },
  {
    name: "pdf_invoice_extraction",
    vertical: "finance",
    description:
      "Run sandboxed Python in an E2B box to OCR a batch of PDF invoices, normalize line items, and push to the ledger.",
    steps: 4,
    versions: 1,
    lastEdited: "3 hours ago",
    lastEditor: "Nori",
    risk: "approval-required",
    substrate: "e2b",
  },
];

export const SAMPLE_WORKFLOW_SPECS: Record<string, WorkflowSpec> = {
  "support/triage_ticket": {
    name: "triage_ticket",
    vertical: "support",
    description:
      "Classify inbound tickets, deflect with KB articles, escalate VIPs to a human. Median resolution under 4 minutes.",
    version: "v6",
    risk: "autonomous",
    substrate: "headless",
    steps: [
      {
        id: "step-1",
        kind: "trigger",
        name: "Ticket arrives",
        description: "Zendesk, Intercom, Help Scout, or email — all wire to the same trigger.",
        inputs: ["customer_email", "subject", "body", "tags"],
      },
      {
        id: "step-2",
        kind: "skill_invoke",
        name: "Classify severity + intent",
        description:
          "Skill `support.classify` tags severity (P0..P3) + intent (refund / how-to / bug / billing).",
        outputs: ["severity", "intent"],
      },
      {
        id: "step-3",
        kind: "decision",
        name: "VIP / churn-signal branch",
        description:
          "If the customer is VIP, on a renewal-risk plan, or the message contains 'cancel'/'churn', skip self-deflection and hand straight to a human.",
      },
    ],
  },
  "sales/qualify_lead": {
    name: "qualify_lead",
    vertical: "sales",
    description: "BANT-style qualification + calendar-link insertion + CRM update on accept.",
    version: "v4",
    risk: "approval-required",
    substrate: "headless",
    steps: [
      {
        id: "step-1",
        kind: "trigger",
        name: "Lead from web / paid / referral",
        description:
          "HubSpot/Salesforce webhook fires when a form is submitted or a high-intent page is visited.",
      },
      {
        id: "step-2",
        kind: "skill_invoke",
        name: "Enrich + BANT score",
        description:
          "Skill `lead.enrich` pulls firmographic + technographic data; BANT-scores 0-100 across Budget, Authority, Need, Timeline.",
      },
      {
        id: "step-3",
        kind: "decision",
        name: "Branch on BANT score",
        description: "≥75 = book demo with AE. 50-74 = nurture. <50 = polite decline.",
      },
      {
        id: "step-4",
        kind: "approval",
        name: "AE approves outreach",
        description:
          "For ≥75 leads, the assigned AE gets a Slack DM with the qualification summary + one-tap approve to send the outreach email.",
        approvalGate: {
          reason: "Outreach to enterprise prospects requires the AE's eyes-on signoff.",
          channel: "Slack",
        },
      },
      {
        id: "step-5",
        kind: "tool_call",
        name: "Send personalized outreach + calendar link",
        description:
          "Gmail / Outlook sends the personalized message with the AE's Calendly slot. CRM record auto-updated.",
      },
    ],
  },
  "support/approve_refund": {
    name: "approve_refund",
    vertical: "support",
    description: "Verify refund eligibility, check ledger, route high-value to manager.",
    version: "v2",
    risk: "approval-required",
    substrate: "headless",
    steps: [
      {
        id: "step-1",
        kind: "trigger",
        name: "Refund request from support agent",
        description: "Comes in via Zendesk macro or Intercom workflow.",
      },
      {
        id: "step-2",
        kind: "skill_invoke",
        name: "Verify eligibility",
        description:
          "Skill `refund.verify` checks return window, condition, and refund-policy clauses.",
      },
      {
        id: "step-3",
        kind: "tool_call",
        name: "Check ledger balance",
        description: "Stripe / Shopify ledger pull. Cross-check against customer's lifetime value.",
      },
      {
        id: "step-4",
        kind: "decision",
        name: "Threshold check",
        description: "<$200 = auto-approve. $200-1000 = manager approval. >$1000 = director.",
      },
      {
        id: "step-5",
        kind: "approval",
        name: "Manager approves high-value refund",
        description: "Slack DM with full context, one-tap approve / reject.",
        approvalGate: {
          reason:
            "Refunds above the auto-approve threshold need manager signoff for revenue tracking.",
          channel: "Slack",
        },
      },
    ],
  },
  "sales/morning_pipeline_digest": {
    name: "morning_pipeline_digest",
    vertical: "sales",
    description:
      "Every weekday morning, pull yesterday's pipeline movement from your CRM API, summarise the headline numbers, post to Slack.",
    version: "v2",
    risk: "autonomous",
    substrate: "headless",
    steps: [
      {
        id: "step-1",
        kind: "trigger",
        name: "Weekday 8am cron",
        description: "Fires Monday–Friday at 8am client-local-time.",
      },
      {
        id: "step-2",
        kind: "http_request",
        name: "Fetch yesterday's deals from HubSpot",
        description:
          "GET the deals that moved stage in the last 24 hours. Bearer-token auth, 15 second timeout.",
        outputs: ["deals[]"],
        httpRequest: {
          method: "GET",
          url: "https://api.hubspot.com/crm/v3/objects/deals?updatedSince=24h",
          headers: [{ key: "Accept", value: "application/json" }],
          body: "",
          auth: "bearer",
          authToken: "{{secret.hubspot_token}}",
          authUsername: "",
          authPassword: "",
          authHeaderName: "",
          authHeaderValue: "",
          timeoutSeconds: 15,
        },
      },
      {
        id: "step-3",
        kind: "skill_invoke",
        name: "Summarise + post to #sales",
        description:
          "Skill `pipeline.summarise` writes 3 bullet points (won / lost / stuck) and posts to Slack.",
      },
    ],
  },
  "finance/clean_customer_list": {
    name: "clean_customer_list",
    vertical: "finance",
    description:
      "Take a messy CSV of customer names, run a short JavaScript step to title-case and dedupe, write the cleaned file back.",
    version: "v1",
    risk: "autonomous",
    substrate: "headless",
    steps: [
      {
        id: "step-1",
        kind: "trigger",
        name: "CSV dropped into the inbox",
        description: "Drag-drop a CSV into the finance inbox, or arrive via email attachment.",
        inputs: ["rows[]"],
      },
      {
        id: "step-2",
        kind: "code",
        name: "Title-case names + dedupe",
        description:
          "A few lines of JavaScript: trim whitespace, title-case each name, drop exact duplicates.",
        outputs: ["cleaned_rows[]"],
        code: {
          language: "javascript",
          code: [
            "const seen = new Set();",
            "const clean = (s) => s.trim().toLowerCase().replace(/\\b\\w/g, (c) => c.toUpperCase());",
            "return input.rows",
            "  .map((row) => ({ ...row, name: clean(row.name) }))",
            "  .filter((row) => {",
            "    if (seen.has(row.name)) return false;",
            "    seen.add(row.name);",
            "    return true;",
            "  });",
          ].join("\n"),
        },
      },
      {
        id: "step-3",
        kind: "data",
        name: "Write cleaned CSV back",
        description: "Saves to /finance/cleaned/ with the date appended to the filename.",
      },
    ],
  },
  "gtm-engineer/post_listing": {
    name: "post_listing",
    vertical: "gtm-engineer",
    description:
      "When a seller submits inventory, validate it, generate the listing copy, and post to Facebook Marketplace once the seller approves.",
    version: "v3",
    risk: "approval-required",
    substrate: "headless",
    steps: [
      {
        id: "step-1",
        kind: "trigger",
        name: "Seller submits inventory",
        description: "Slack DM, email, or web inbox carrying photos + price + condition.",
        inputs: ["photos[]", "price", "condition", "title"],
      },
      {
        id: "step-2",
        kind: "skill_invoke",
        name: "Validate inventory data",
        description:
          "Skill `inventory.validate` checks the price against market range, runs photo OCR for category hints, flags below-floor pricing.",
        inputs: ["seller input"],
        outputs: ["validation report"],
      },
      {
        id: "step-3",
        kind: "decision",
        name: "Branch on validation outcome",
        description:
          "If price is below floor or condition is unverifiable, route to manager review. Otherwise continue.",
      },
      {
        id: "step-4",
        kind: "skill_invoke",
        name: "Generate listing copy",
        description:
          "Skill `listing.compose` writes a 60-word buyer-facing description, picks 3 highlighted features, attaches up to 5 photos.",
        outputs: ["listing.title", "listing.body", "listing.photos[]"],
      },
      {
        id: "step-5",
        kind: "approval",
        name: "Seller approves the listing",
        description:
          "Send draft listing + image carousel to the seller; one-tap approve / request edits.",
        approvalGate: {
          reason: "Posting public listings requires explicit seller sign-off.",
          channel: "Slack",
        },
      },
      {
        id: "step-6",
        kind: "tool_call",
        name: "Post to Facebook Marketplace",
        description:
          "Your teammate's browser opens Facebook Marketplace, fills the form, uploads photos, submits. Returns the public listing URL.",
        outputs: ["listing.url"],
      },
    ],
  },
  "project-ops/weekly_standup_digest": {
    name: "weekly_standup_digest",
    vertical: "project-ops",
    description:
      "Every Monday morning, pull last week's Linear + GitHub activity per teammate, draft a 5-bullet standup digest, and post to Slack #standup.",
    version: "v2",
    risk: "autonomous",
    substrate: "headless",
    steps: [
      {
        id: "step-1",
        kind: "trigger",
        name: "Monday 8am cron",
        description: "Fires Monday at 8am client-local-time.",
      },
      {
        id: "step-2",
        kind: "skill_invoke",
        name: "Pull Linear + GitHub activity",
        description:
          "Skill `project.weekly_activity` collects last week's closed issues, merged PRs, and review counts per teammate.",
        outputs: ["activity[]"],
      },
      {
        id: "step-3",
        kind: "skill_invoke",
        name: "Draft 5-bullet digest",
        description:
          "Skill `standup.digest` writes 5 bullets covering shipped work, blockers, and what's next per teammate.",
        outputs: ["digest"],
      },
      {
        id: "step-4",
        kind: "notify",
        name: "Post to Slack #standup",
        description: "Posts the digest as a threaded message in #standup; pins it for the day.",
      },
    ],
  },
  "people-ops/screen_candidate": {
    name: "screen_candidate",
    vertical: "people-ops",
    description:
      "First-pass screen of inbound candidates against the JD. Surfaces strong matches, declines clear misses, books interviews for the in-between.",
    version: "v5",
    risk: "autonomous",
    substrate: "headless",
    steps: [
      {
        id: "step-1",
        kind: "trigger",
        name: "Candidate applies",
        description: "Resume + cover-letter arrive via Greenhouse webhook or email.",
      },
      {
        id: "step-2",
        kind: "skill_invoke",
        name: "Parse resume + JD",
        description: "Skill `candidate.parse` extracts skills, years, employers; loads the open JD.",
      },
      {
        id: "step-3",
        kind: "skill_invoke",
        name: "Score skill-match",
        description: "0-100 score across hard skills, years, optional must-haves; flags red flags.",
      },
      {
        id: "step-4",
        kind: "decision",
        name: "Branch on score band",
        description: "≥80 = strong yes; 50-79 = book screen; <50 = polite decline.",
      },
      {
        id: "step-5",
        kind: "skill_invoke",
        name: "Compose response",
        description: "Skill `outreach.compose` writes the candidate-facing message in your voice.",
      },
      {
        id: "step-6",
        kind: "tool_call",
        name: "Book interview / send decline",
        description:
          "Strong yes → calendly invite; book screen → 15-min slots; decline → polite no.",
      },
      {
        id: "step-7",
        kind: "notify",
        name: "Update hiring manager",
        description: "Slack DM the hiring manager with score, summary, next step taken.",
      },
      {
        id: "step-8",
        kind: "data",
        name: "Append to ATS",
        description: "Greenhouse note + tag (strong-yes / phone-screen / declined).",
      },
    ],
  },
  "sales/customer_research_followup": {
    name: "customer_research_followup",
    vertical: "sales",
    description:
      "After a Zoom discovery call, dump structured call notes into the macOS Notes app, summarise next steps, and DM the AE.",
    version: "v1",
    risk: "autonomous",
    substrate: "macos",
    steps: [
      {
        id: "step-1",
        kind: "trigger",
        name: "Discovery call ends",
        description: "Zoom webhook fires when the call recording finishes processing.",
      },
      {
        id: "step-2",
        kind: "skill_invoke",
        name: "Summarise transcript",
        description:
          "Skill `call.summarise` extracts pain points, decision criteria, and committed next steps from the transcript.",
        outputs: ["summary", "next_steps[]"],
      },
      {
        id: "step-3",
        kind: "tool_call",
        name: "Append note in macOS Notes",
        description:
          "Computer-use macro `cu.run_macro` opens the macOS Notes app on the AE's Mac and appends a new note titled after the account.",
      },
      {
        id: "step-4",
        kind: "notify",
        name: "DM the AE with the summary",
        description: "Slack DM with the 3-bullet summary + a deep-link into the Notes entry.",
      },
    ],
  },
  "gtm-engineer/competitor_pricing_scrape": {
    name: "competitor_pricing_scrape",
    vertical: "gtm-engineer",
    description:
      "Drive a real Chrome browser via Playwright to compare competitor pricing pages, flag deltas, post the diff to Slack.",
    version: "v2",
    risk: "approval-required",
    substrate: "browser",
    steps: [
      {
        id: "step-1",
        kind: "trigger",
        name: "Daily 6am cron",
        description: "Fires Monday–Friday at 6am.",
      },
      {
        id: "step-2",
        kind: "tool_call",
        name: "Open competitor pricing pages in Chrome",
        description:
          "Playwright launches a headed Chrome browser session and visits each competitor pricing URL.",
        outputs: ["pricing_snapshots[]"],
      },
      {
        id: "step-3",
        kind: "skill_invoke",
        name: "Diff against yesterday",
        description:
          "Skill `pricing.diff` compares the snapshot against the previous run, flagging any > 5% movement.",
        outputs: ["deltas[]"],
      },
      {
        id: "step-4",
        kind: "approval",
        name: "Sales lead approves the digest",
        description: "Slack DM with the diff card + one-tap approve before posting to #competitive.",
        approvalGate: {
          reason: "Pricing intel goes to a public channel; sales lead reviews before publication.",
          channel: "Slack",
        },
      },
      {
        id: "step-5",
        kind: "notify",
        name: "Post diff to #competitive",
        description: "Threaded Slack message with a snapshot screenshot and per-tier deltas.",
      },
    ],
  },
  "finance/pdf_invoice_extraction": {
    name: "pdf_invoice_extraction",
    vertical: "finance",
    description:
      "Run sandboxed Python in an E2B box to OCR a batch of PDF invoices, normalize line items, and push to the ledger.",
    version: "v1",
    risk: "approval-required",
    substrate: "e2b",
    steps: [
      {
        id: "step-1",
        kind: "trigger",
        name: "Invoice batch lands in /finance/inbox",
        description: "Watcher fires when a folder of PDFs arrives by SFTP or drag-drop.",
      },
      {
        id: "step-2",
        kind: "tool_call",
        name: "Spin up E2B sandbox + run OCR",
        description:
          "Start an E2B sandboxed Python session, install pdfplumber, extract line items from each PDF.",
        outputs: ["line_items[]"],
      },
      {
        id: "step-3",
        kind: "approval",
        name: "Controller reviews flagged invoices",
        description:
          "Any invoice with an OCR-confidence score below 0.85 surfaces for manual review.",
        approvalGate: {
          reason: "Low-confidence OCR could mis-bill; controller signs off before posting.",
          channel: "Slack",
        },
      },
      {
        id: "step-4",
        kind: "data",
        name: "Write normalized rows to the ledger",
        description: "Inserts into the AP ledger with the source PDF attached for audit.",
      },
    ],
  },
};
