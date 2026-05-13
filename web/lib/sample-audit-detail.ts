/**
 * Enterprise-flavored sample audit-detail payloads used when the
 * gateway endpoint `GET /api/v1/audit/<actionId>` is unavailable
 * (404 in dev) or auth is bypassed. Five canonical hand-shaped
 * entries cover the personas the brief calls out (Priya, Sam,
 * Mariah, Jamal, Nori); every other action id known to the feed
 * synthesises a "good-enough" detail from the underlying `AuditRow`
 * so click-through from the audit feed never hits a 404 in dev.
 *
 * Timestamps anchor against `SAMPLE_AUDIT_ROWS` where applicable so
 * SSR/CSR strings match.
 */
import { SAMPLE_AUDIT_ROWS } from "@/lib/sample-audit";
import type { AuditDetail, DecisionStep } from "@/lib/types/audit-detail";
import type { AuditRow } from "@/lib/types/audit";

const PRIYA = { id: "wkr_01J0PRIYA", name: "Priya" };
const SAM = { id: "wkr_01J0SAM", name: "Sam" };
const MARIAH = { id: "wkr_01J0MARIAH", name: "Mariah" };
const JAMAL = { id: "wkr_01J0JAMAL", name: "Jamal" };
const NORI = { id: "wkr_01J0NORI", name: "Nori" };

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
  id: "act_priya_001",
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
    predecessor: { id: "act_priya_000", verb: "received Greenhouse webhook for JD-2024-08" },
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
    followUp: { id: "act_priya_002", verb: "booked Thursday 10:30 interview with Diego Alvarez" },
  },
};

const SAM_001: AuditDetail = {
  id: "act_sam_001",
  timestamp: isoMinutesAgo(6),
  worker: SAM,
  actionType: "invoice_processed",
  verb: "extracted INV-2024-08741 line items",
  status: "succeeded",
  cost: { usd: 0.012 },
  dataClassification: "confidential",
  inputs: {
    triggerKind: "message",
    triggerSummary: "PDF invoice INV-2024-08741 received via accounts@acmeco.com.",
  },
  decisionChain: [
    { n: 1, title: "Memory loaded", body: "vendor catalog · PO map (cached) [hash 7e2f…]" },
    { n: 2, title: "Skill: invoice-parse v3 (trust 96%)", body: "OCR confidence 0.98 across all pages." },
    { n: 3, title: "Plan generated", body: "Extract 14 line items · match to PO #PO-2024-1132." },
    { n: 4, title: "Plan selected by score", body: "Top plan 0.96; no viable alternates." },
    { n: 5, title: "Policy: ap-payment-policy v1.4 → ASK (over $10k → CFO)", body: "Amount $14,250 above CFO threshold." },
    { n: 6, title: "Approval queued", body: "Sent to Jenna via Slack (CFO).", details: { channel: "slack", approver: "Jenna" } },
    { n: 7, title: "Execution", body: "Posted extraction · awaiting CFO sign-off for release." },
    { n: 8, title: "Outcome", body: "Extracted, queued for approval." },
  ],
  provenance: {
    model: { id: "claude-opus-4-7", version: "2026-04-30" },
    promptTemplate: { id: "ap.invoice-parse", hash: "1a2b3c4d5e6f7a8b" },
    skill: { id: "invoice-parse", version: "3.1.0" },
    memorySnapshot: { id: "mem_7e2f88aa" },
    policy: { id: "ap-payment-policy", version: "1.4", hash: "9b8a7c6d5e4f3a2b" },
  },
  output: {
    kind: "json",
    body: { invoiceId: "INV-2024-08741", lineItems: 14, totalUsd: 14250, poMatch: "PO-2024-1132" },
    delivery: { sentAt: isoMinutesAgo(6), downstreamId: "netsuite_iv_8741" },
    followUp: { id: "act_sam_002", verb: "queued $14,250 for CFO approval (INV-2024-08741)" },
  },
};

const MARIAH_001: AuditDetail = {
  id: "act_mariah_001",
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
    predecessor: { id: "act_mariah_000", verb: "completed RFP research for Tomás Industries" },
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
    body: "Re: RFP-2024-Q3-Tomás\n\nThank you for the opportunity to respond. Our differentiators:\n  • Migration path proven across 12 enterprise rollouts.\n  • Tracking dashboard at https://app.linear.app/acmeco/issue/RFP-184\n  • SLA 99.95% with cred to 99.99% by Q1.\n\nFull pricing in attached schedule.\n",
    delivery: { sentAt: isoMinutesAgo(14), downstreamId: "drive_doc_aabb22" },
  },
};

const JAMAL_001: AuditDetail = {
  id: "act_jamal_001",
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
    body: "Hi Maria — your order is on track for Thursday delivery. Tracking: https://github.com/acmeco/ops/issues/2912 covers the carrier coordination. KB article: https://help.acmeco.com/SK-413.\n\nThanks,\nJamal",
    delivery: { sentAt: isoMinutesAgo(4), receivedAt: isoMinutesAgo(3), downstreamId: "zd_msg_29382" },
  },
};

const NORI_001: AuditDetail = {
  id: "act_nori_001",
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

export const SAMPLE_AUDIT_DETAILS: AuditDetail[] = [
  PRIYA_001,
  SAM_001,
  MARIAH_001,
  JAMAL_001,
  NORI_001,
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
