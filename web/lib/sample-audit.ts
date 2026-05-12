/**
 * Enterprise-flavored sample audit rows used when the gateway
 * endpoint `GET /api/v1/audit` is unavailable (404 in dev) or auth
 * is bypassed. Spreads across the 6 E4-a workers (Priya, Sam,
 * Mariah, Jamal, Nori, Rio); 50+ rows total so cursor pagination
 * exercises ≥2 pages at the locked 50-row PAGE_SIZE.
 *
 * Tuple-shaped seeds keep this file well under the 350-LOC ceiling
 * while preserving the audit verb richness the brief calls for.
 * Timestamps anchor to a stable floor-to-minute boundary so SSR/CSR
 * strings match.
 */
import type { AuditRow, AuditStatus, DataClassification } from "@/lib/types/audit";
import type { ChannelKind } from "@/lib/sample-approvals";

function stableNow(): number {
  const d = new Date();
  d.setSeconds(0, 0);
  return d.getTime();
}

const NOW = stableNow();
const M = 60_000;
const H = 60 * M;
const D = 24 * H;

const PRIYA = { id: "wkr_01J0PRIYA", name: "Priya" };
const SAM = { id: "wkr_01J0SAM", name: "Sam" };
const MARIAH = { id: "wkr_01J0MARIAH", name: "Mariah" };
const JAMAL = { id: "wkr_01J0JAMAL", name: "Jamal" };
const NORI = { id: "wkr_01J0NORI", name: "Nori" };
const RIO = { id: "wkr_01J0RIO", name: "Rio" };

const OPERATOR = { id: "usr_op_mark", name: "Mark" };
const CFO = { id: "usr_cfo_jenna", name: "Jenna" };
const COMPLIANCE = { id: "usr_compliance_aria", name: "Aria" };

interface Extra {
  cost?: number;
  channel?: ChannelKind;
  approver?: { id: string; name: string };
  cls?: DataClassification;
}

type Seed = [worker: typeof PRIYA, verb: string, actionType: string, status: AuditStatus, agoMs: number, extra?: Extra];

const SEEDS: Seed[] = [
  // Priya — Recruiter
  [PRIYA, "screened Diego Alvarez against JD-2024-08", "candidate_screened", "succeeded", 8 * M, { cost: 0.014, channel: "web", cls: "confidential" }],
  [PRIYA, "booked Thursday 10:30 interview with Diego Alvarez", "interview_booked", "succeeded", 22 * M, { channel: "web" }],
  [PRIYA, "drafted rejection email to Sara Lin", "email_drafted", "pending", 47 * M, { cost: 0.009, channel: "web", approver: OPERATOR, cls: "confidential" }],
  [PRIYA, "asked Mark to approve $50 LinkedIn InMail credit", "spend_approval_asked", "pending", 1 * H + 15 * M, { channel: "slack", approver: OPERATOR }],
  [PRIYA, "refused to forward candidate resume outside org", "policy_refusal", "refused", 2 * H + 40 * M, { cls: "restricted" }],
  [PRIYA, "learned scheduling heuristic from operator correction", "skill_learned", "succeeded", 5 * H],
  [PRIYA, "wrote candidate-prefs memory for role JD-2024-08", "memory_written", "autonomous", 23 * H],
  [PRIYA, "escalated background-check anomaly to compliance", "escalation", "succeeded", 1 * D + 4 * H, { channel: "slack", approver: COMPLIANCE, cls: "restricted" }],
  [PRIYA, "sourced 5 new candidates from referral channel", "candidate_sourced", "succeeded", 2 * D + 1 * H],
  [PRIYA, "failed to parse PDF resume — falling back to OCR", "parse_failure", "failed", 4 * D + 2 * H],

  // Sam — AP Specialist
  [SAM, "extracted INV-2024-08741 line items", "invoice_processed", "succeeded", 6 * M, { cost: 0.012, cls: "confidential" }],
  [SAM, "flagged duplicate invoice INV-2024-08739", "duplicate_detected", "refused", 38 * M, { cls: "internal" }],
  [SAM, "queued $14,250 for CFO approval (INV-2024-08741)", "spend_approval_asked", "pending", 1 * H + 32 * M, { approver: CFO, channel: "slack", cls: "confidential" }],
  [SAM, "reconciled vendor statement for Acme Tech", "reconciliation", "autonomous", 3 * H + 4 * M],
  [SAM, "queued $8,400 for CFO approval (INV-2024-08712)", "spend_approval_asked", "succeeded", 6 * H + 18 * M, { approver: CFO, channel: "slack" }],
  [SAM, "failed to parse PDF — manual review required", "parse_failure", "failed", 1 * D + 7 * H],
  [SAM, "matched 12 invoices to POs in NetSuite", "po_matching", "autonomous", 2 * D + 3 * H],
  [SAM, "refused to release payment outside approval policy", "policy_refusal", "refused", 5 * D + 10 * H, { cls: "restricted" }],

  // Mariah — Vendor Manager
  [MARIAH, "drafted RFP response summary for Tomás Industries", "rfp_drafted", "succeeded", 14 * M, { cost: 0.021, cls: "internal" }],
  [MARIAH, "scheduled renewal review with Acme Tech", "renewal_scheduled", "succeeded", 51 * M, { channel: "web" }],
  [MARIAH, "compared 3 vendor quotes for Q3 SaaS bundle", "vendor_compared", "succeeded", 2 * H + 18 * M, { cost: 0.018 }],
  [MARIAH, "refused to disclose contract terms to external party", "policy_refusal", "refused", 4 * H + 5 * M, { cls: "restricted" }],
  [MARIAH, "asked Jenna to approve renewal pricing (+8% YoY)", "spend_approval_asked", "pending", 7 * H + 22 * M, { approver: CFO, channel: "slack" }],
  [MARIAH, "logged vendor risk score for Tomás Industries (B+)", "vendor_assessed", "autonomous", 1 * D + 6 * H],
  [MARIAH, "escalated SLA breach notice to vendor account team", "escalation", "succeeded", 3 * D + 2 * H, { channel: "imessage" }],

  // Jamal — Support
  [JAMAL, "resolved SUP-29382 with KB article SK-413", "ticket_resolved", "succeeded", 4 * M, { cost: 0.004 }],
  [JAMAL, "resolved SUP-29381 with KB article SK-208", "ticket_resolved", "autonomous", 19 * M, { cost: 0.003 }],
  [JAMAL, "escalated refund request SUP-29355 to support lead", "escalation", "succeeded", 1 * H + 2 * M, { channel: "slack", approver: OPERATOR }],
  [JAMAL, "drafted apology reply to customer Maria González", "email_drafted", "pending", 2 * H + 25 * M, { approver: OPERATOR, cls: "confidential" }],
  [JAMAL, "failed to fetch order history — Shopify rate-limited", "api_failure", "failed", 6 * H + 12 * M],
  [JAMAL, "wrote churn-signal memory from SUP-29340 transcript", "memory_written", "autonomous", 1 * D + 9 * H],
  [JAMAL, "refused to share customer PII with unverified caller", "policy_refusal", "refused", 4 * D + 3 * H, { channel: "voice", cls: "restricted" }],

  // Nori — RevOps
  [NORI, "validated QT-2024-7283 against pricing rules", "quote_validated", "succeeded", 11 * M, { cost: 0.008 }],
  [NORI, "flagged 32% discount on QT-2024-7283 for Deal Desk", "discount_flagged", "pending", 31 * M, { channel: "slack", approver: OPERATOR }],
  [NORI, "synced 14 closed-won opportunities to NetSuite", "data_sync", "autonomous", 1 * H + 47 * M],
  [NORI, "drafted commission true-up email to AE Tomás Pereira", "email_drafted", "pending", 3 * H + 9 * M, { cost: 0.011, approver: OPERATOR, cls: "internal" }],
  [NORI, "refused to override deal-stage without operator sign-off", "policy_refusal", "refused", 7 * H + 30 * M],
  [NORI, "validated QT-2024-7271 — auto-approved (within policy)", "quote_validated", "autonomous", 1 * D + 2 * H],
  [NORI, "failed to fetch pipeline export — Salesforce 503", "api_failure", "failed", 2 * D + 4 * H],
  [NORI, "escalated quote-policy gap to RevOps lead", "escalation", "succeeded", 6 * D + 1 * H, { channel: "slack" }],

  // Rio — Real Estate (paused, sparse)
  [RIO, "booked showing for 412 Oak St with buyer Tomás Pereira", "showing_booked", "succeeded", 22 * H, { channel: "sms" }],
  [RIO, "drafted listing summary for 119 Cedar Ave", "listing_drafted", "succeeded", 1 * D + 5 * H, { cost: 0.015, approver: OPERATOR }],
  [RIO, "paused by operator (see worker profile)", "worker_paused", "succeeded", 5 * D + 2 * H, { approver: OPERATOR }],
  [RIO, "wrote buyer-prefs memory for the Pereira family", "memory_written", "autonomous", 6 * D + 8 * H],
  [RIO, "refused to disclose seller's reservation price", "policy_refusal", "refused", 8 * D, { cls: "restricted" }],

  // Older log entries — push beyond 7 days to exercise pagination
  [PRIYA, "closed JD-2024-07 with hire (Sara Lin → backup pool)", "role_closed", "succeeded", 10 * D + 4 * H],
  [SAM, "monthly reconciliation report exported to Drive", "report_exported", "autonomous", 12 * D],
  [MARIAH, "closed RFP cycle 2024-Q2 with 3 finalists", "rfp_closed", "succeeded", 14 * D + 2 * H],
  [JAMAL, "resolved 47 tickets in weekly batch (success rate 96%)", "batch_resolved", "autonomous", 16 * D],
  [NORI, "Q1 commission report finalized + sent to Finance", "report_exported", "succeeded", 38 * D, { approver: CFO }],
  [NORI, "annual quote-policy review — 4 rules updated", "policy_updated", "succeeded", 90 * D, { approver: OPERATOR, cls: "internal" }],
];

function buildRows(seeds: Seed[]): AuditRow[] {
  return seeds.map((s, i) => {
    const [worker, verb, actionType, status, agoMs, extra] = s;
    return {
      id: `act_${worker.id.replace("wkr_", "")}_${i.toString().padStart(3, "0")}`,
      timestamp: new Date(NOW - agoMs).toISOString(),
      worker,
      actionType,
      verb,
      status,
      approver: extra?.approver,
      channel: extra?.channel,
      cost: extra?.cost !== undefined ? { usd: extra.cost } : undefined,
      dataClassification: extra?.cls,
    };
  });
}

export const SAMPLE_AUDIT_ROWS: AuditRow[] = buildRows(SEEDS).sort(
  (a, b) => Date.parse(b.timestamp) - Date.parse(a.timestamp),
);
