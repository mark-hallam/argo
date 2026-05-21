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
 *
 * Worker refs come from `sample-personas.ts` so the actions shown
 * here line up with the role on /team and the overview on
 * /workers/<id>/overview (wave-4 data-integrity fix).
 */
import { personaRef } from "@/lib/sample-personas";
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

const WORKSPACE = { id: "sys_workspace", name: "Workspace" };
const PRIYA = personaRef("wkr_01J0PRIYA");
const SAM = personaRef("wkr_01J0SAM");
const MARIAH = personaRef("wkr_01J0MARIAH");
const JAMAL = personaRef("wkr_01J0JAMAL");
const NORI = personaRef("wkr_01J0NORI");
const RIO = personaRef("wkr_01J0RIO");

const OPERATOR = { id: "usr_op_mark", name: "Mark" };
const SRE_LEAD = { id: "usr_sre_neha", name: "Neha" };
const BRAND_LEAD = { id: "usr_brand_aoife", name: "Aoife" };
const SELLER = { id: "usr_seller_marco", name: "Marco" };
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

  // Sam — On-call Triager (Reliability)
  [SAM, "triaged alert 'api-latency-p99' against runbook", "alert_triaged", "succeeded", 6 * M, { cost: 0.012, cls: "internal" }],
  [SAM, "suppressed flapping alert 'cache-evict-rate'", "alert_suppressed", "refused", 38 * M, { cls: "internal" }],
  [SAM, "queued runbook restart (edge-cdn) for SRE approval", "action_approval_asked", "pending", 1 * H + 32 * M, { approver: SRE_LEAD, channel: "slack", cls: "internal" }],
  [SAM, "opened incident INC-0421 for 'edge-cdn-5xx'", "incident_opened", "autonomous", 3 * H + 4 * M],
  [SAM, "paged secondary on-call after 5m no-ack on INC-0420", "paged_secondary", "succeeded", 6 * H + 18 * M, { approver: SRE_LEAD, channel: "slack" }],
  [SAM, "failed to fetch Grafana dashboard — falling back to logs", "api_failure", "failed", 1 * D + 7 * H],
  [SAM, "matched 12 alerts to runbook entries", "alert_matched", "autonomous", 2 * D + 3 * H],
  [SAM, "refused to silence alert without runbook entry", "policy_refusal", "refused", 5 * D + 10 * H, { cls: "restricted" }],

  // Mariah — Marketplace Lister (Procurement)
  [MARIAH, "drafted listing copy for 'Eames lounge chair'", "listing_drafted", "succeeded", 14 * M, { cost: 0.021, cls: "internal" }],
  [MARIAH, "scheduled re-price sweep for stale inventory", "reprice_scheduled", "succeeded", 51 * M, { channel: "web" }],
  [MARIAH, "compared 3 competitor listings for 'Herman Miller chair'", "market_compared", "succeeded", 2 * H + 18 * M, { cost: 0.018 }],
  [MARIAH, "refused to use stock imagery on a live listing", "policy_refusal", "refused", 4 * H + 5 * M, { cls: "internal" }],
  [MARIAH, "asked Marco to approve re-price below floor (−12%)", "spend_approval_asked", "pending", 7 * H + 22 * M, { approver: SELLER, channel: "slack" }],
  [MARIAH, "logged buyer-question-rate score for 'Saarinen tulip' (B+)", "listing_assessed", "autonomous", 1 * D + 6 * H],
  [MARIAH, "escalated buyer complaint on 'Knoll side table' to seller", "escalation", "succeeded", 3 * D + 2 * H, { channel: "imessage", approver: SELLER }],

  // Jamal — Support
  [JAMAL, "resolved SUP-29382 with KB article SK-413", "ticket_resolved", "succeeded", 4 * M, { cost: 0.004 }],
  [JAMAL, "resolved SUP-29381 with KB article SK-208", "ticket_resolved", "autonomous", 19 * M, { cost: 0.003 }],
  [JAMAL, "escalated refund request SUP-29355 to support lead", "escalation", "succeeded", 1 * H + 2 * M, { channel: "slack", approver: OPERATOR }],
  [JAMAL, "drafted apology reply to customer Maria González", "email_drafted", "pending", 2 * H + 25 * M, { approver: OPERATOR, cls: "confidential" }],
  [JAMAL, "failed to fetch order history — Shopify rate-limited", "api_failure", "failed", 6 * H + 12 * M],
  [JAMAL, "wrote churn-signal memory from SUP-29340 transcript", "memory_written", "autonomous", 1 * D + 9 * H],
  [JAMAL, "refused to share customer PII with unverified caller", "policy_refusal", "refused", 4 * D + 3 * H, { channel: "voice", cls: "restricted" }],

  // Nori — Brand Comms (Brand)
  [NORI, "tone-checked 'Q2 product update' draft against brand voice", "tone_checked", "succeeded", 11 * M, { cost: 0.008 }],
  [NORI, "flagged 'AI hype' wording in 'May newsletter' for brand lead", "wording_flagged", "pending", 31 * M, { channel: "slack", approver: BRAND_LEAD }],
  [NORI, "synced 14 published posts to the content calendar", "data_sync", "autonomous", 1 * H + 47 * M],
  [NORI, "drafted 'pricing-page refresh' announcement post", "email_drafted", "pending", 3 * H + 9 * M, { cost: 0.011, approver: BRAND_LEAD, cls: "internal" }],
  [NORI, "refused to publish without brand-lead sign-off", "policy_refusal", "refused", 7 * H + 30 * M],
  [NORI, "scheduled 'May newsletter' send — auto-approved (within policy)", "post_scheduled", "autonomous", 1 * D + 2 * H],
  [NORI, "failed to fetch reach numbers — analytics 503", "api_failure", "failed", 2 * D + 4 * H],
  [NORI, "escalated brand-voice ambiguity to brand lead", "escalation", "succeeded", 6 * D + 1 * H, { channel: "slack", approver: BRAND_LEAD }],

  // Rio — Real Estate (paused, sparse)
  [RIO, "booked showing for 412 Oak St with buyer Tomás Pereira", "showing_booked", "succeeded", 22 * H, { channel: "sms" }],
  [RIO, "drafted listing summary for 119 Cedar Ave", "listing_drafted", "succeeded", 1 * D + 5 * H, { cost: 0.015, approver: OPERATOR }],
  [RIO, "paused by operator (see worker profile)", "worker_paused", "succeeded", 5 * D + 2 * H, { approver: OPERATOR }],
  [RIO, "wrote buyer-prefs memory for the Pereira family", "memory_written", "autonomous", 6 * D + 8 * H],
  [RIO, "refused to disclose seller's reservation price", "policy_refusal", "refused", 8 * D, { cls: "restricted" }],

  // API-key lifecycle — workspace-level events from /settings/api-keys
  [WORKSPACE, "API key created: 'CI pipeline'", "api_key_created", "succeeded", 14 * D, { approver: OPERATOR, cls: "internal" }],
  [WORKSPACE, "API key created: 'Data warehouse sync'", "api_key_created", "succeeded", 47 * D, { approver: OPERATOR, cls: "internal" }],
  [WORKSPACE, "API key revoked: 'Legacy migration tool'", "api_key_revoked", "succeeded", 30 * D, { approver: OPERATOR, cls: "internal" }],

  // Older log entries — push beyond 7 days to exercise pagination
  [PRIYA, "closed JD-2024-07 with hire (Sara Lin → backup pool)", "role_closed", "succeeded", 10 * D + 4 * H],
  [SAM, "monthly on-call digest exported to Drive", "report_exported", "autonomous", 12 * D],
  [MARIAH, "closed listing cycle 2024-Q2 with 3 sell-throughs", "listing_closed", "succeeded", 14 * D + 2 * H],
  [JAMAL, "resolved 47 tickets in weekly batch (success rate 96%)", "batch_resolved", "autonomous", 16 * D],
  [NORI, "Q1 reach + engagement report finalized + sent to brand lead", "report_exported", "succeeded", 38 * D, { approver: BRAND_LEAD }],
  [NORI, "annual brand-voice review — 4 rules updated", "policy_updated", "succeeded", 90 * D, { approver: OPERATOR, cls: "internal" }],

  // Cohort-scope events — surfaces /settings/members access changes
  // and runtime denials in the standard audit feed.
  [PRIYA, "Access changed: cohort scope set to [Sales · Established] for Diego Alvarez", "access_changed", "succeeded", 35 * M, { approver: OPERATOR, cls: "confidential" }],
  [SAM, "Cohort-scope-denied: Diego Alvarez tried to access Sam (On-call Triager)", "cohort_scope_denied", "refused", 1 * H + 50 * M, { cls: "internal" }],
  [JAMAL, "Access changed: cohort scope set to [Support · Trusted, Support · Developing] for Theo Park", "access_changed", "succeeded", 1 * D + 3 * H, { approver: OPERATOR, cls: "confidential" }],
];

function buildRows(seeds: Seed[]): AuditRow[] {
  return seeds.map((s, i) => {
    const [worker, verb, actionType, status, agoMs, extra] = s;
    const id = `act_${worker.id.replace("wkr_", "")}_${i.toString().padStart(3, "0")}`;
    const timestamp = new Date(NOW - agoMs).toISOString();
    return {
      id,
      timestamp,
      worker,
      actionType,
      verb,
      status,
      approver: extra?.approver,
      channel: extra?.channel,
      cost: extra?.cost !== undefined ? { usd: extra.cost } : undefined,
      dataClassification: extra?.cls,
      hash: rowHash(`${worker.id}|${verb}|${timestamp}`),
    };
  });
}

/** Deterministic 64-hex digest seeded by `worker.id|verb|timestamp`.
 *  Synchronous so the seed array stays array-typed (callers iterate it
 *  directly); `crypto.subtle.digest` would force a Promise and reshape
 *  every consumer. Eight independent FNV-1a passes with distinct
 *  offsets produce 8×32 bits = 64 hex chars — matches the visual
 *  length of a SHA-256 digest so the demo MP4 reads as forensic-grade.
 *  Run-to-run identical for fixed input. Not a cryptographic hash;
 *  the real chain hash arrives from the gateway in production. */
function rowHash(seed: string): string {
  const OFFSETS: readonly number[] = [
    0x811c9dc5, 0x9e3779b1, 0x85ebca6b, 0xc2b2ae35,
    0x27d4eb2f, 0x165667b1, 0xd3a2646c, 0xfd7046c5,
  ];
  return OFFSETS.map((offset) =>
    fnv1a(seed, offset, 0x01000193).toString(16).padStart(8, "0"),
  ).join("");
}

function fnv1a(input: string, offset: number, prime: number): number {
  let h = offset >>> 0;
  for (let i = 0; i < input.length; i++) {
    h ^= input.charCodeAt(i);
    h = Math.imul(h, prime);
  }
  return h >>> 0;
}

export const SAMPLE_AUDIT_ROWS: AuditRow[] = buildRows(SEEDS).sort(
  (a, b) => Date.parse(b.timestamp) - Date.parse(a.timestamp),
);
