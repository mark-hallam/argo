/**
 * Enterprise-flavored sample audit-detail payloads used when the
 * gateway endpoint `GET /api/v1/audit/<actionId>` is unavailable
 * (404 in dev) or auth is bypassed. Five canonical hand-shaped
 * entries cover the canonical demo roster (Priya, Peta, Mariah,
 * Jamal, Nori); every other action id known to the feed
 * synthesises a "good-enough" detail from the underlying `AuditRow`
 * so click-through from the audit feed never hits a 404 in dev.
 *
 * Timestamps anchor against `SAMPLE_AUDIT_ROWS` where applicable so
 * SSR/CSR strings match.
 */
import { SAMPLE_AUDIT_ROWS } from "@/lib/sample-audit";
import {
  ACTIVITY_TAB_SEEDS,
  activityIdFor,
  type ActivityTabSeed,
} from "@/lib/sample-worker-activity";
import type { AuditDetail, DecisionStep } from "@/lib/types/audit-detail";
import type { AuditRow, AuditStatus, DataClassification } from "@/lib/types/audit";
import type { WorkerActivityActionType } from "@/lib/api/worker-activity";

const PRIYA = { id: "019e6785-928d-78b1-a6b2-136d9ed263ce", name: "Priya" };
const PETA = { id: "019e6785-928c-72b3-85c6-436fe5004c50", name: "Peta" };
const MARIAH = { id: "019e6785-928a-7270-a766-f945b6cd7161", name: "Mariah" };
const JAMAL = { id: "019e6785-928e-7f20-803b-55266e526011", name: "Jamal" };
const NORI = { id: "019e6785-928f-7281-bf8d-e062bc277478", name: "Nori" };
const PORTIA = { id: "019e6785-9290-7a41-9c2e-4b8d10f5c7a2", name: "Portia" };
const GRANT = { id: "019e6785-9291-7b52-8d3f-6c9e21a6d8b3", name: "Grant" };
const MARCUS = { id: "019e6785-9292-7c63-9e40-7d0f32b7e9c4", name: "Marcus" };

const OPERATOR = { id: "usr_op_mark", name: "Mark" };
const CFO = { id: "usr_cfo_jenna", name: "Jenna" };

function isoMinutesAgo(min: number): string {
  const d = new Date();
  d.setSeconds(0, 0);
  return new Date(d.getTime() - min * 60_000).toISOString();
}

function defaultChain(skillId: string, skillVersion: string): DecisionStep[] {
  return [
    { n: 1, title: "Memory loaded", body: "context (cached) · pricing rules" },
    { n: 2, title: `Skill: ${skillId} v${skillVersion} (trust 92%)`, body: "Confidence above autonomy threshold." },
    { n: 3, title: "Plan generated", body: "Primary plan + 2 alternatives.", details: { alternates: 2 } },
    { n: 4, title: "Plan selected by score", body: "Top score 0.87; alternates 0.74 / 0.62." },
    { n: 5, title: "Policy check", body: "Action permitted by policy." },
    { n: 6, title: "Approval", body: "Autonomous — no approval required." },
    { n: 7, title: "Execution", body: "Executor dispatched." },
    { n: 8, title: "Outcome", body: "Completed successfully." },
  ];
}

const PRIYA_001: AuditDetail = {
  id: "act_01J0PRIYA_001",
  timestamp: isoMinutesAgo(8),
  worker: PRIYA,
  actionType: "candidate_screened",
  verb: "screened Diego Alvarez against JD-2024-08",
  status: "succeeded",
  cost: { usd: 0.014 },
  dataClassification: "confidential",
  inputs: {
    triggerKind: "message",
    triggerSummary: "Application received from Diego Alvarez via Greenhouse webhook.",
    predecessor: { id: "act_01J0PRIYA_000", verb: "received Greenhouse webhook for JD-2024-08" },
  },
  decisionChain: [
    { n: 1, title: "Memory loaded", body: "must-have skills · candidate-prefs (cached) [hash 4f3a…]", details: { hashes: ["4f3a9c", "b21d77"] } },
    { n: 2, title: "Skill: recruiter-screen v4 (trust 94%)", body: "Confidence above autonomy threshold." },
    { n: 3, title: "Plan generated", body: "Match against must-haves → 6/6, nice-to-haves → 4/5.", details: { matched: 6, total: 6 } },
    { n: 4, title: "Plan selected by score", body: "Top plan score 0.93 — auto-advance." },
    { n: 5, title: "Policy: recruiter-screen-policy v2.0 → PASS", body: "No flags; resume PII matched DLP policy." },
    { n: 6, title: "Approval", body: "Autonomous — within policy envelope." },
    { n: 7, title: "Execution", body: "Wrote score to Greenhouse · pushed to interview queue." },
    { n: 8, title: "Outcome", body: "Sent, 200 OK · pipeline updated." },
  ],
  provenance: {
    model: { id: "claude-opus-4-7", version: "2026-04-30" },
    promptTemplate: { id: "recruiter.screen", hash: "8a3b9f1c2d4e5f6a" },
    skill: { id: "recruiter-screen", version: "4.2.0" },
    memorySnapshot: { id: "mem_4f3a9c12" },
    policy: { id: "recruiter-screen-policy", version: "2.0", hash: "c91e2b4f5a6d7c8e" },
  },
  output: {
    kind: "json",
    body: { score: 0.93, mustHaves: { matched: 6, total: 6 }, recommendation: "advance" },
    delivery: { sentAt: isoMinutesAgo(8), receivedAt: isoMinutesAgo(8), downstreamId: "gh_ev_4f1c" },
    followUp: { id: "act_01J0PRIYA_002", verb: "booked Thursday 10:30 interview with Diego Alvarez" },
  },
};

const PETA_001: AuditDetail = {
  id: "act_01J0PETA_001",
  timestamp: isoMinutesAgo(6),
  worker: PETA,
  actionType: "budget_shift_queued",
  verb: "queued sprint-14 contractor-hours budget shift for CFO approval",
  status: "succeeded",
  cost: { usd: 0.012 },
  dataClassification: "confidential",
  inputs: {
    triggerKind: "message",
    triggerSummary: "Sprint-14 reshuffle requires +$14,250 in contractor hours to hit the release date.",
  },
  decisionChain: [
    { n: 1, title: "Memory loaded", body: "project plan · contractor day-rate sheet (cached) [hash 7e2f…]" },
    { n: 2, title: "Skill: project-coord-plan v3 (trust 96%)", body: "Reshuffle confidence 0.94 across affected workstreams." },
    { n: 3, title: "Plan generated", body: "Reallocate 95 hours · 4 contractors · 2 sprints." },
    { n: 4, title: "Plan selected by score", body: "Top plan 0.96; alternate (delay) 0.41." },
    { n: 5, title: "Policy: budget-shift-policy v1.4 → ASK (over $10k → CFO)", body: "Amount $14,250 above CFO threshold." },
    { n: 6, title: "Approval queued", body: "Sent to Jenna via Slack (CFO).", details: { channel: "slack", approver: "Jenna" } },
    { n: 7, title: "Execution", body: "Posted plan · awaiting CFO sign-off for release." },
    { n: 8, title: "Outcome", body: "Drafted, queued for approval." },
  ],
  provenance: {
    model: { id: "claude-opus-4-7", version: "2026-04-30" },
    promptTemplate: { id: "pmo.budget-shift", hash: "1a2b3c4d5e6f7a8b" },
    skill: { id: "project-coord-plan", version: "3.1.0" },
    memorySnapshot: { id: "mem_7e2f88aa" },
    policy: { id: "budget-shift-policy", version: "1.4", hash: "9b8a7c6d5e4f3a2b" },
  },
  output: {
    kind: "json",
    body: { projectId: "PROJ-14-Q3", shiftUsd: 14250, hoursReallocated: 95, contractors: 4 },
    delivery: { sentAt: isoMinutesAgo(6), downstreamId: "linear_proj_14_q3" },
    followUp: { id: "act_01J0PETA_002", verb: "queued $14,250 sprint-14 budget shift for CFO approval" },
  },
};

const MARIAH_001: AuditDetail = {
  id: "act_01J0MARIAH_001",
  timestamp: isoMinutesAgo(14),
  worker: MARIAH,
  actionType: "rfp_drafted",
  verb: "drafted RFP response summary for Tomás Industries",
  status: "succeeded",
  cost: { usd: 0.021 },
  dataClassification: "internal",
  inputs: {
    triggerKind: "dependent_action",
    triggerSummary: "RFP cycle 2024-Q3 stage transition: research-complete → drafting.",
    predecessor: { id: "act_01J0MARIAH_000", verb: "completed RFP research for Tomás Industries" },
  },
  decisionChain: [
    { n: 1, title: "Memory loaded", body: "tone profile · prior RFP wins (cached) [hash 9c8d…]" },
    { n: 2, title: "Multi-skill plan", body: "research · draft · format — chained.", details: { skills: ["rfp-research", "rfp-draft", "rfp-format"] } },
    { n: 3, title: "Plan generated", body: "12-section draft, 4 differentiators highlighted." },
    { n: 4, title: "Plan selected by score", body: "0.88; alternate (terse) 0.61." },
    { n: 5, title: "Policy: contract-disclosure v3.0 → PASS", body: "No restricted terms referenced." },
    { n: 6, title: "Approval", body: "Routed to Mark for final review (queued)." },
    { n: 7, title: "Execution", body: "Wrote draft to shared Drive · linked to RFP record." },
    { n: 8, title: "Outcome", body: "Draft saved, review pending." },
  ],
  provenance: {
    model: { id: "claude-opus-4-7", version: "2026-04-30" },
    promptTemplate: { id: "vendor.rfp-summary", hash: "ee11ff22aabb33cc" },
    skill: { id: "rfp-draft", version: "2.0.1" },
    memorySnapshot: { id: "mem_9c8d4411" },
    policy: { id: "contract-disclosure", version: "3.0", hash: "44dd55ee66ff7788" },
  },
  output: {
    kind: "text",
    body: "Re: RFP-2024-Q3-Tomás\n\nThank you for the opportunity to respond. Our differentiators:\n  • Migration path proven across 12 enterprise rollouts.\n  • Tracking dashboard at https://app.linear.app/skycot/issue/RFP-184\n  • SLA 99.95% with cred to 99.99% by Q1.\n\nFull pricing in attached schedule.\n",
    delivery: { sentAt: isoMinutesAgo(14), downstreamId: "drive_doc_aabb22" },
  },
};

const JAMAL_001: AuditDetail = {
  id: "act_01J0JAMAL_001",
  timestamp: isoMinutesAgo(4),
  worker: JAMAL,
  actionType: "ticket_resolved",
  verb: "resolved SUP-29382 with KB article SK-413",
  status: "succeeded",
  cost: { usd: 0.004 },
  inputs: {
    triggerKind: "message",
    triggerSummary: "Customer ticket SUP-29382 opened via Zendesk webhook.",
  },
  decisionChain: [
    { n: 1, title: "Memory loaded", body: "KB index · customer-history (cached) [hash a1b2…]" },
    { n: 2, title: "Skill: support-resolve v5 (trust 93%)", body: "Topic match: shipping-eta · confidence 0.91." },
    { n: 3, title: "Plan generated", body: "Reply with KB SK-413 + tracking link." },
    { n: 4, title: "Plan selected by score", body: "0.91; alternate (escalate) 0.34." },
    { n: 5, title: "Policy: support-reply-autonomous v1.2 → PASS", body: "Below escalation threshold." },
    { n: 6, title: "Approval", body: "Autonomous — within policy envelope." },
    { n: 7, title: "Execution", body: "Posted reply via Zendesk · marked resolved." },
    { n: 8, title: "Outcome", body: "Sent, 200 OK · customer notified." },
  ],
  provenance: {
    model: { id: "claude-haiku-4-5", version: "2026-04-15" },
    promptTemplate: { id: "support.resolve", hash: "deadbeef11223344" },
    skill: { id: "support-resolve", version: "5.0.2" },
    memorySnapshot: { id: "mem_a1b2c3d4" },
    policy: { id: "support-reply-autonomous", version: "1.2", hash: "55667788aabbccdd" },
  },
  output: {
    kind: "text",
    body: "Hi Maria — your order is on track for Thursday delivery. Tracking: https://github.com/skycot/ops/issues/2912 covers the carrier coordination. KB article: https://help.skycot.com/SK-413.\n\nThanks,\nJamal",
    delivery: { sentAt: isoMinutesAgo(4), receivedAt: isoMinutesAgo(3), downstreamId: "zd_msg_29382" },
  },
};

const NORI_001: AuditDetail = {
  id: "act_01J0NORI_001",
  timestamp: isoMinutesAgo(11),
  worker: NORI,
  actionType: "quote_validated",
  verb: "validated QT-2024-7283 against pricing rules",
  status: "succeeded",
  cost: { usd: 0.008 },
  dataClassification: "internal",
  inputs: {
    triggerKind: "message",
    triggerSummary: "Quote QT-2024-7283 submitted by AE Tomás Pereira for validation.",
  },
  decisionChain: [
    { n: 1, title: "Memory loaded", body: "pricing rules · discount ladder (cached) [hash 88ee…]" },
    { n: 2, title: "Skill: revops-validate v2 (trust 91%)", body: "Within tier-2 deal-size envelope." },
    { n: 3, title: "Plan generated", body: "Discount 32% — above 25% threshold → Deal Desk referral." },
    { n: 4, title: "Plan selected by score", body: "Top plan 0.79; refuse-and-bounce 0.18." },
    { n: 5, title: "Policy: revops-discount v1.7 → ASK (>25% → Deal Desk)", body: "Routing to Deal Desk via Slack." },
    { n: 6, title: "Approval queued", body: "Slack → Mark (Deal Desk). Pending.", details: { channel: "slack", approver: "Mark" } },
    { n: 7, title: "Execution", body: "Validation written to NetSuite · queued for Deal Desk." },
    { n: 8, title: "Outcome", body: "Pending approval — flagged in pipeline." },
  ],
  provenance: {
    model: { id: "claude-opus-4-7", version: "2026-04-30" },
    promptTemplate: { id: "revops.validate-quote", hash: "1122aabb33445566" },
    skill: { id: "revops-validate", version: "2.4.0" },
    memorySnapshot: { id: "mem_88ee9911" },
    policy: { id: "revops-discount", version: "1.7", hash: "abcdef1234567890" },
  },
  output: {
    kind: "json",
    body: { quoteId: "QT-2024-7283", discountPct: 32, action: "refer-deal-desk", priceBookEffective: "2026-05-01" },
    delivery: { sentAt: isoMinutesAgo(11), downstreamId: "ns_quote_7283" },
  },
};

/** Lightweight `AuditDetail` builder used to seed the 5-per-persona
 *  drill-in destinations for the `/workers/<id>/overview` "Recent
 *  activity" card. The hand-shaped `_001` entries above tell deeper
 *  product-story arcs (replay, hash-chain copy, cross-system
 *  jump-links); these `_011..._015` entries match the activity-card
 *  rows 1:1 so the camera-side drill-in lands on a real-shaped
 *  detail page with the correct verb, timestamp, and worker rather
 *  than the "Action detail unavailable" placeholder fallthrough. */
interface OverviewActivitySeed {
  idSuffix: number;
  minutesAgo: number;
  verb: string;
  actionType: string;
  status: AuditStatus;
  approver?: { id: string; name: string };
  skill?: { id: string; version: string };
  cost?: number;
  dataClassification?: DataClassification;
}

function overviewActivityDetail(
  worker: { id: string; name: string },
  personaSlug: string,
  seed: OverviewActivitySeed,
): AuditDetail {
  const skill = seed.skill ?? { id: "operator-routine", version: "1.0.0" };
  const approvalStep: DecisionStep = seed.approver
    ? {
        n: 6,
        title: "Approval",
        body: `Approved by ${seed.approver.name}.`,
        details: { approver: seed.approver.name },
      }
    : seed.status === "autonomous"
      ? { n: 6, title: "Approval", body: "Autonomous — within policy envelope." }
      : { n: 6, title: "Approval", body: "Pending operator review." };
  const sentAt = isoMinutesAgo(seed.minutesAgo);
  return {
    id: `act_01J0${personaSlug}_0${seed.idSuffix.toString().padStart(2, "0")}`,
    timestamp: sentAt,
    worker,
    actionType: seed.actionType,
    verb: seed.verb,
    status: seed.status,
    approver: seed.approver,
    cost: seed.cost !== undefined ? { usd: seed.cost } : undefined,
    dataClassification: seed.dataClassification,
    inputs: {
      triggerKind: "message",
      triggerSummary: `Surfaced from the worker overview recent-activity feed · ${seed.actionType}.`,
    },
    decisionChain: [
      { n: 1, title: "Memory loaded", body: "context (cached) · operator routines" },
      { n: 2, title: `Skill: ${skill.id} v${skill.version} (trust 92%)`, body: "Confidence above autonomy threshold." },
      { n: 3, title: "Plan generated", body: "Primary plan + 2 alternates considered." },
      { n: 4, title: "Plan selected by score", body: "Top plan 0.87; alternates 0.74 / 0.62." },
      { n: 5, title: "Policy check", body: "Action permitted by policy." },
      approvalStep,
      { n: 7, title: "Execution", body: "Executor dispatched." },
      { n: 8, title: "Outcome", body: seed.status === "pending" ? "Awaiting reviewer." : "Completed." },
    ],
    provenance: {
      model: { id: "claude-opus-4-7", version: "2026-04-30" },
      promptTemplate: { id: `${skill.id}.activity`, hash: "a01b02c03d04e05f" },
      skill,
      memorySnapshot: { id: `mem_${personaSlug.toLowerCase()}_overview` },
      policy: { id: `${personaSlug.toLowerCase()}-operator-policy`, version: "1.0", hash: "11223344aabbccdd" },
    },
    output: {
      kind: "text",
      body: `${seed.verb}.\n\nSurfaced via the worker overview recent-activity feed.`,
      delivery: { sentAt },
    },
  };
}

const MARTA = { id: "usr_hm_marta", name: "Marta" };
const SAM = { id: "usr_buyer_sam", name: "Sam" };
const BRAND_LEAD = { id: "usr_brand_aoife", name: "Brand Lead" };
const MARK = OPERATOR;

const PRIYA_OVERVIEW: OverviewActivitySeed[] = [
  { idSuffix: 11, minutesAgo: 14, verb: "booked interview with candidate #4821", actionType: "interview_booked", status: "succeeded", approver: MARTA, skill: { id: "recruiter-schedule", version: "2.0.0" }, dataClassification: "confidential" },
  { idSuffix: 12, minutesAgo: 38, verb: "screened candidate #4820 — recommended advance", actionType: "candidate_screened", status: "autonomous", skill: { id: "recruiter-screen", version: "4.2.0" }, cost: 0.014, dataClassification: "confidential" },
  { idSuffix: 13, minutesAgo: 72, verb: "screened candidate #4819 — soft no", actionType: "candidate_screened", status: "succeeded", approver: MARTA, skill: { id: "recruiter-screen", version: "4.2.0" }, cost: 0.014, dataClassification: "confidential" },
  { idSuffix: 14, minutesAgo: 145, verb: "drafted rejection for candidate #4811", actionType: "email_drafted", status: "pending", skill: { id: "recruiter-reject-draft", version: "1.3.0" }, cost: 0.009, dataClassification: "confidential" },
  { idSuffix: 15, minutesAgo: 210, verb: "asked hiring manager: relocation budget?", actionType: "spend_approval_asked", status: "pending", approver: MARTA, skill: { id: "recruiter-clarify", version: "1.0.0" } },
];

const MARIAH_OVERVIEW: OverviewActivitySeed[] = [
  { idSuffix: 11, minutesAgo: 2, verb: "replied to buyer #4821 on listing 'Eames lounge'", actionType: "buyer_replied", status: "succeeded", approver: SAM, skill: { id: "marketplace-reply", version: "1.4.0" }, dataClassification: "internal" },
  { idSuffix: 12, minutesAgo: 18, verb: "re-priced listing 'Herman Miller chair' −$45", actionType: "listing_repriced", status: "autonomous", skill: { id: "marketplace-reprice", version: "2.0.0" }, cost: 0.018, dataClassification: "internal" },
  { idSuffix: 13, minutesAgo: 41, verb: "listed 'Vintage Bauhaus desk' with 6 photos", actionType: "listing_drafted", status: "succeeded", approver: SAM, skill: { id: "marketplace-list", version: "1.2.0" }, cost: 0.021, dataClassification: "internal" },
  { idSuffix: 14, minutesAgo: 96, verb: "flagged 'Knoll side table' below floor", actionType: "listing_flagged", status: "pending", skill: { id: "marketplace-floor-check", version: "1.0.0" } },
  { idSuffix: 15, minutesAgo: 175, verb: "replied to buyer #4799 on listing 'Saarinen tulip'", actionType: "buyer_replied", status: "autonomous", skill: { id: "marketplace-reply", version: "1.4.0" }, dataClassification: "internal" },
];

const JAMAL_OVERVIEW: OverviewActivitySeed[] = [
  { idSuffix: 11, minutesAgo: 4, verb: "issued refund $42 — order #18293", actionType: "refund_issued", status: "autonomous", skill: { id: "support-refund", version: "3.1.0" }, cost: 0.004 },
  { idSuffix: 12, minutesAgo: 11, verb: "reset password for user@acme.co", actionType: "password_reset", status: "autonomous", skill: { id: "support-password-reset", version: "1.0.0" } },
  { idSuffix: 13, minutesAgo: 27, verb: "escalated billing dispute on order #18254", actionType: "escalation", status: "succeeded", approver: MARTA, skill: { id: "support-escalate", version: "2.0.0" }, dataClassification: "confidential" },
  { idSuffix: 14, minutesAgo: 52, verb: "resolved shipping query — order #18247", actionType: "ticket_resolved", status: "autonomous", skill: { id: "support-resolve", version: "5.0.2" }, cost: 0.004 },
  { idSuffix: 15, minutesAgo: 88, verb: "sent CSAT to user@globex.io", actionType: "csat_sent", status: "autonomous", skill: { id: "support-csat", version: "1.0.0" } },
];

const NORI_OVERVIEW: OverviewActivitySeed[] = [
  { idSuffix: 11, minutesAgo: 22, verb: "drafted Q2 product update announcement", actionType: "email_drafted", status: "pending", skill: { id: "brand-draft", version: "1.0.0" }, cost: 0.011, dataClassification: "internal" },
  { idSuffix: 12, minutesAgo: 60, verb: "tone-checked 'May newsletter' draft", actionType: "tone_checked", status: "succeeded", approver: BRAND_LEAD, skill: { id: "brand-tone", version: "1.2.0" }, cost: 0.008 },
  { idSuffix: 13, minutesAgo: 130, verb: "scheduled send for 'May newsletter' Mon 09:00", actionType: "post_scheduled", status: "succeeded", approver: BRAND_LEAD, skill: { id: "brand-schedule", version: "1.0.0" } },
  { idSuffix: 14, minutesAgo: 240, verb: "rolled up weekly reach (+18% wow)", actionType: "report_exported", status: "autonomous", skill: { id: "brand-rollup", version: "2.0.0" } },
  { idSuffix: 15, minutesAgo: 480, verb: "drafted 'pricing-page refresh' post", actionType: "email_drafted", status: "succeeded", approver: BRAND_LEAD, skill: { id: "brand-draft", version: "1.0.0" }, cost: 0.011, dataClassification: "internal" },
];

const PETA_OVERVIEW: OverviewActivitySeed[] = [
  { idSuffix: 11, minutesAgo: 6, verb: "drafted standup digest for engineering pod", actionType: "standup_drafted", status: "autonomous", skill: { id: "project-coord-standup", version: "1.2.0" }, cost: 0.012, dataClassification: "internal" },
  { idSuffix: 12, minutesAgo: 92, verb: "chased action item PROJ-2284 — owner replied", actionType: "action_item_chased", status: "autonomous", skill: { id: "project-coord-chase", version: "1.0.0" } },
  { idSuffix: 13, minutesAgo: 180, verb: "escalated stalled task PROJ-721 to PM", actionType: "escalation", status: "succeeded", approver: MARK, skill: { id: "project-coord-escalate", version: "1.0.0" }, dataClassification: "internal" },
  { idSuffix: 14, minutesAgo: 1440, verb: "rolled up weekly status (3 wins / 2 risks)", actionType: "report_exported", status: "autonomous", skill: { id: "project-coord-rollup", version: "1.0.0" } },
  { idSuffix: 15, minutesAgo: 2880, verb: "asked PM about priority shift for sprint 14", actionType: "spend_approval_asked", status: "pending", approver: MARK, skill: { id: "project-coord-clarify", version: "1.0.0" } },
];

const PORTIA_OVERVIEW: OverviewActivitySeed[] = [
  { idSuffix: 11, minutesAgo: 12, verb: "matched investor Q on ARR to approved fact", actionType: "canonical_qa_matched", status: "autonomous", skill: { id: "pitch-canonical-qa", version: "1.0.0" }, cost: 0.006, dataClassification: "confidential" },
  { idSuffix: 12, minutesAgo: 34, verb: "refused unsupported financials claim — escalated to founder", actionType: "escalation", status: "pending", approver: MARK, skill: { id: "pitch-truth-boundary", version: "1.0.0" }, dataClassification: "confidential" },
  { idSuffix: 13, minutesAgo: 88, verb: "ran Series A rehearsal pass — 18 questions", actionType: "rehearsal_run", status: "autonomous", skill: { id: "pitch-rehearsal", version: "1.0.0" }, cost: 0.022 },
  { idSuffix: 14, minutesAgo: 160, verb: "drafted recap for yesterday's investor call", actionType: "email_drafted", status: "succeeded", approver: MARK, skill: { id: "pitch-recap", version: "1.0.0" }, cost: 0.009, dataClassification: "confidential" },
  { idSuffix: 15, minutesAgo: 300, verb: "asked founder to confirm headcount figure", actionType: "spend_approval_asked", status: "pending", approver: MARK, skill: { id: "pitch-clarify", version: "1.0.0" } },
];

const GRANT_OVERVIEW: OverviewActivitySeed[] = [
  { idSuffix: 11, minutesAgo: 9, verb: "scored signup acme.co — high fit, queued outreach", actionType: "lead_scored", status: "autonomous", skill: { id: "gtm-lead-score", version: "1.0.0" }, cost: 0.005, dataClassification: "internal" },
  { idSuffix: 12, minutesAgo: 26, verb: "drafted first-touch email for globex.io", actionType: "email_drafted", status: "pending", skill: { id: "gtm-outreach-draft", version: "1.0.0" }, cost: 0.011, dataClassification: "internal" },
  { idSuffix: 13, minutesAgo: 64, verb: "flagged enterprise signup initech.com to a human", actionType: "escalation", status: "succeeded", approver: MARK, skill: { id: "gtm-handoff", version: "1.0.0" }, dataClassification: "internal" },
  { idSuffix: 14, minutesAgo: 132, verb: "nurtured warming trial — booked demo slot", actionType: "demo_booked", status: "autonomous", skill: { id: "gtm-nurture", version: "1.0.0" } },
  { idSuffix: 15, minutesAgo: 280, verb: "scored signup hooli.xyz — low fit, no outreach", actionType: "lead_scored", status: "autonomous", skill: { id: "gtm-lead-score", version: "1.0.0" }, cost: 0.005, dataClassification: "internal" },
];

const MARCUS_OVERVIEW: OverviewActivitySeed[] = [
  { idSuffix: 11, minutesAgo: 4, verb: "drafted an X reply to a 'just fancy autocomplete' post", actionType: "marketing_reply_drafted", status: "pending", skill: { id: "marketing-reply-draft", version: "1.0.0" }, cost: 0.009, dataClassification: "public" },
  { idSuffix: 12, minutesAgo: 30, verb: "drafted a LinkedIn article on ideas as the human moat", actionType: "marketing_article_drafted", status: "pending", skill: { id: "marketing-article-draft", version: "1.0.0" }, cost: 0.018, dataClassification: "public" },
  { idSuffix: 13, minutesAgo: 75, verb: "held a draft that touched a revenue claim — escalated to founder", actionType: "truth_boundary_escalation", status: "succeeded", approver: MARK, skill: { id: "marketing-truth-boundary", version: "1.0.0" }, dataClassification: "confidential" },
  { idSuffix: 14, minutesAgo: 150, verb: "summarized a mention so the operator could decide", actionType: "mention_summarized", status: "autonomous", skill: { id: "marketing-mention-summary", version: "1.0.0" }, cost: 0.006, dataClassification: "public" },
  { idSuffix: 15, minutesAgo: 310, verb: "drafted an X reply welcoming a new launch in the space", actionType: "marketing_reply_drafted", status: "pending", skill: { id: "marketing-reply-draft", version: "1.0.0" }, cost: 0.009, dataClassification: "public" },
];

/** Build an `AuditDetail` for one row of the full Activity-tab feed
 * (`/workers/<id>/activity`). Mirrors `overviewActivityDetail` but
 * sources the verb / actionType / status / cost from the Activity-tab
 * Seed shape, which is a strict subset of the Overview seed's fields.
 * IDs round-trip through `activityIdFor(persona.initials, index)`. */
function activityTabDetail(
  worker: { id: string; name: string },
  personaSlug: string,
  initials: string,
  seed: ActivityTabSeed,
  index: number,
): AuditDetail {
  const sentAt = new Date(Date.now() - seed.agoMs).toISOString();
  const skill = { id: `${personaSlug.toLowerCase()}-activity`, version: "1.0.0" };
  return {
    id: activityIdFor(initials, index),
    timestamp: sentAt,
    worker,
    actionType: seed.actionType as unknown as string,
    verb: seed.verb,
    status: seed.status as AuditStatus,
    cost: seed.costUsd !== undefined ? { usd: seed.costUsd } : undefined,
    inputs: {
      triggerKind: "message",
      triggerSummary: `Surfaced from the worker activity tab · ${seed.actionType}.`,
    },
    decisionChain: [
      { n: 1, title: "Memory loaded", body: "context (cached) · operator routines" },
      { n: 2, title: `Skill: ${skill.id} v${skill.version} (trust 90%)`, body: "Confidence above autonomy threshold." },
      { n: 3, title: "Plan generated", body: "Primary plan + 2 alternates considered." },
      { n: 4, title: "Plan selected by score", body: "Top plan 0.86; alternates 0.71 / 0.59." },
      { n: 5, title: "Policy check", body: "Action permitted by policy." },
      seed.status === "pending"
        ? { n: 6, title: "Approval", body: "Pending operator review." }
        : seed.status === "refused"
          ? { n: 6, title: "Approval", body: "Refused — policy boundary." }
          : { n: 6, title: "Approval", body: "Autonomous — within policy envelope." },
      { n: 7, title: "Execution", body: "Executor dispatched." },
      {
        n: 8,
        title: "Outcome",
        body:
          seed.status === "succeeded"
            ? "Completed."
            : seed.status === "refused"
              ? "Action refused; no side-effects."
              : seed.status === "failed"
                ? "Executor returned error; rolled back."
                : "Awaiting reviewer.",
      },
    ],
    provenance: {
      model: { id: "claude-opus-4-7", version: "2026-04-30" },
      promptTemplate: { id: `${skill.id}.activity`, hash: "a02b03c04d05e06f" },
      skill,
      memorySnapshot: { id: `mem_${personaSlug.toLowerCase()}_activity` },
      policy: { id: `${personaSlug.toLowerCase()}-operator-policy`, version: "1.0", hash: "22334455bbccddee" },
    },
    output: {
      kind: "text",
      body: `${seed.verb}.\n\nSurfaced via the worker activity tab.`,
      delivery: { sentAt },
    },
  };
}

const ACTIVITY_TAB_DETAILS: AuditDetail[] = [
  ...ACTIVITY_TAB_SEEDS.PRIYA.map((s, i) => activityTabDetail(PRIYA, "PRIYA", "PR", s, i)),
  ...ACTIVITY_TAB_SEEDS.MARIAH.map((s, i) => activityTabDetail(MARIAH, "MARIAH", "MA", s, i)),
  ...ACTIVITY_TAB_SEEDS.JAMAL.map((s, i) => activityTabDetail(JAMAL, "JAMAL", "JA", s, i)),
  ...ACTIVITY_TAB_SEEDS.NORI.map((s, i) => activityTabDetail(NORI, "NORI", "NO", s, i)),
  ...ACTIVITY_TAB_SEEDS.PETA.map((s, i) => activityTabDetail(PETA, "PETA", "PE", s, i)),
];

export const SAMPLE_AUDIT_DETAILS: AuditDetail[] = [
  PRIYA_001,
  PETA_001,
  MARIAH_001,
  JAMAL_001,
  NORI_001,
  ...PRIYA_OVERVIEW.map((s) => overviewActivityDetail(PRIYA, "PRIYA", s)),
  ...MARIAH_OVERVIEW.map((s) => overviewActivityDetail(MARIAH, "MARIAH", s)),
  ...JAMAL_OVERVIEW.map((s) => overviewActivityDetail(JAMAL, "JAMAL", s)),
  ...NORI_OVERVIEW.map((s) => overviewActivityDetail(NORI, "NORI", s)),
  ...PETA_OVERVIEW.map((s) => overviewActivityDetail(PETA, "PETA", s)),
  ...PORTIA_OVERVIEW.map((s) => overviewActivityDetail(PORTIA, "PORTIA", s)),
  ...GRANT_OVERVIEW.map((s) => overviewActivityDetail(GRANT, "GRANT", s)),
  ...MARCUS_OVERVIEW.map((s) => overviewActivityDetail(MARCUS, "MARCUS", s)),
  ...ACTIVITY_TAB_DETAILS,
];

/** Synthesise a "good-enough" detail from a feed `AuditRow` so click-
 *  through from the audit feed never hits a 404 in dev. Real samples
 *  in `SAMPLE_AUDIT_DETAILS` take precedence. */
export function synthesizeAuditDetail(row: AuditRow): AuditDetail {
  return {
    ...row,
    inputs: {
      triggerKind: "message",
      triggerSummary: `Trigger reconstructed from feed row · ${row.actionType}.`,
    },
    decisionChain: defaultChain("generic-skill", "1.0"),
    provenance: {
      model: { id: "claude-opus-4-7", version: "2026-04-30" },
      promptTemplate: { id: "generic.action", hash: "feedfeedfeedfeed" },
      skill: { id: "generic-skill", version: "1.0.0" },
      memorySnapshot: { id: "mem_synthetic" },
      policy: { id: "generic-policy", version: "1.0", hash: "policepolicepolicy" },
    },
    output: {
      kind: "text",
      body: `Synthetic output reconstructed from feed row ${row.id}. Production wires this to the gateway detail endpoint.`,
      delivery: { sentAt: row.timestamp },
    },
  };
}

/** Resolve a detail by id. First checks the hand-crafted list, then
 *  synthesises from any matching `SAMPLE_AUDIT_ROWS` entry; returns
 *  `undefined` if neither matches. */
export function findSampleAuditDetail(actionId: string): AuditDetail | undefined {
  const hit = SAMPLE_AUDIT_DETAILS.find((d) => d.id === actionId);
  if (hit) return hit;
  const row = SAMPLE_AUDIT_ROWS.find((r) => r.id === actionId);
  if (row) return synthesizeAuditDetail(row);
  return undefined;
}

/** Last-resort placeholder detail built from JUST an action id. Used
 *  when the gateway returns a row in the list endpoint whose detail
 *  endpoint then 4xx/5xxs and demo-mode fallback can find no matching
 *  seed (e.g. live-gateway demo data with ULID-shaped ids that aren't
 *  in `SAMPLE_AUDIT_ROWS`). Renders enough scaffolding for the drill-
 *  in page to land — action_type / timestamp / verb / status — instead
 *  of the global 500 error boundary. */
export function placeholderAuditDetail(actionId: string): AuditDetail {
  return {
    id: actionId,
    timestamp: new Date().toISOString(),
    worker: { id: "wkr_unknown", name: "Unknown worker" },
    actionType: "audit.detail.viewed",
    verb: "Action detail unavailable — drill into another row for full provenance.",
    status: "succeeded",
    inputs: {
      triggerKind: "message",
      triggerSummary: "Detail payload not available for this action id.",
    },
    decisionChain: defaultChain("generic-skill", "1.0"),
    provenance: {
      model: { id: "claude-opus-4-7", version: "2026-04-30" },
      promptTemplate: { id: "generic.placeholder", hash: "placehold00000000" },
      skill: { id: "generic-skill", version: "1.0.0" },
      memorySnapshot: { id: "mem_placeholder" },
      policy: { id: "generic-policy", version: "1.0", hash: "policyplaceholder" },
    },
    output: {
      kind: "text",
      body: `Detail for action ${actionId} is not currently available. The audit feed remains the source of truth; drill into another row for full provenance.`,
    },
  };
}
