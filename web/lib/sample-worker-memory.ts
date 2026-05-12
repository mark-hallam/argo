/**
 * Mock memory payloads for the 6 detailed sample workers
 * (`sample-workers-detail.ts`). Used when the gateway endpoint
 * `/api/v1/workers/<id>/memory` 404s in dev, when
 * `NEXT_PUBLIC_GATEWAY_AUTH_BYPASS=1`, or when running the Memory
 * tab e2e against a worker whose backend endpoints haven't shipped.
 *
 * Enterprise-shaped: facts read like real policy / customer / vendor
 * rules an operator would teach the worker, with 5+ facts per worker
 * so the search filter has something to work with. The mutation
 * helpers (`mockAddFact` / `mockEditFact` / `mockDeleteFact`) mutate
 * the in-memory dataset so the page round-trips a CRUD cycle without
 * a running gateway. State resets on page reload — v1 dev-mode
 * persistence stops at the React tree.
 */

import type {
  ActiveConversation,
  RecentDecision,
  VoiceProfile,
  WorkerFact,
  WorkerMemory,
} from "@/lib/api/worker-memory";
import { WorkerMemoryNotFoundError } from "@/lib/api/worker-memory";

function stableNow(): number {
  const d = new Date();
  d.setSeconds(0, 0);
  return d.getTime();
}

const NOW = stableNow();
const MIN = 60_000;
const HR = 60 * MIN;
const DAY = 24 * HR;

const iso = (agoMs: number) => new Date(NOW - agoMs).toISOString();
const f = (id: string, text: string, ago: number): WorkerFact => ({
  factId: id, text, createdAt: iso(ago),
});
const c = (
  id: string, handle: string, topic: string, ago: number, href: string,
): ActiveConversation => ({
  conversationId: id, counterpartyHandle: handle, topic, lastMessageAt: iso(ago), href,
});
const d = (
  id: string, label: string, outcome: "approved" | "refused", ago: number,
): RecentDecision => ({
  decisionId: id, label, outcome, decidedAt: iso(ago), href: `/audit/action/${id}`,
});
const v = (
  tone: VoiceProfile["tone"], formality: number, signaturePhrases: string[],
): VoiceProfile => ({ tone, formality, signaturePhrases });

const PRIYA: WorkerMemory = {
  namespaceId: "ns_priya_4f7a",
  tenantScope: "Tenant-scoped ✓",
  facts: [
    f("f_priya_1", "Pre-screen all senior engineering candidates against the 6 must-haves in JD-2024-08.", 2 * DAY),
    f("f_priya_2", "Diversity hiring goals: balanced gender + 30% underrepresented minorities; flagged for HR review at offer stage.", 5 * DAY),
    f("f_priya_3", "Use 'guests' not 'candidates' externally per employer brand voice.", 9 * DAY),
    f("f_priya_4", "Never agree to compensation above the role's posted range without VP People sign-off.", 12 * DAY),
    f("f_priya_5", "Friday afternoons: queue rejection emails for Monday morning send (no rejection emails on Fridays per company policy).", 18 * DAY),
    f("f_priya_6", "Workato integration: candidates from referral program get priority screening.", 23 * DAY),
  ],
  activeConversations: [
    c("c_priya_1", "Diego Alvarez", "Interview scheduling — Thu 10:30 confirmation", 18 * MIN, "/audit/action/ap_priya_1"),
    c("c_priya_2", "Marta (Hiring Manager)", "Relocation budget question — JD-2024-08", 3 * HR + 12 * MIN, "/audit/action/ap_priya_2"),
  ],
  recentDecisions: [
    d("d_priya_1", "Approved interview slot for candidate #4821 (within policy)", "approved", 45 * MIN),
    d("d_priya_2", "Refused to forward candidate email outside org (PII policy)", "refused", 6 * HR),
    d("d_priya_3", "Approved fast-track screening for referral candidate #4799", "approved", 1 * DAY + 4 * HR),
  ],
  voiceProfile: v("warm", 35, [
    "Thanks for getting back to me",
    "Looking forward to chatting with you",
    "Let me know what works on your end",
  ]),
};

const MARIAH: WorkerMemory = {
  namespaceId: "ns_mariah_8b21",
  tenantScope: "Tenant-scoped ✓",
  facts: [
    f("f_mariah_1", "Vendor onboarding requires Compliance review for any vendor over $50K/year spend.", 3 * DAY),
    f("f_mariah_2", "Annual renewals route to Procurement + the business owner.", 8 * DAY),
    f("f_mariah_3", "Use 'strategic partners' not 'vendors' for Tier 1 vendors per Operations branding.", 14 * DAY),
    f("f_mariah_4", "Listings: never discount below floor price without category buyer sign-off.", 20 * DAY),
    f("f_mariah_5", "Photos required for every listing — minimum 3, maximum 8.", 25 * DAY),
  ],
  activeConversations: [
    c("c_mariah_1", "vendor#tomas-industries", "Q3 renewal terms", 22 * MIN, "/audit/action/ap_mariah_1"),
    c("c_mariah_2", "buyer#4821", "Question on 'Eames lounge' listing", 2 * HR + 5 * MIN, "/audit/action/ap_mariah_2"),
  ],
  recentDecisions: [
    d("d_mariah_1", "Approved re-price of 'Herman Miller chair' −$45 (within bounds)", "approved", 18 * MIN),
    d("d_mariah_2", "Refused buyer offer below floor on 'Knoll side table'", "refused", 4 * HR),
    d("d_mariah_3", "Approved new vendor application from Tomas Industries (<$50K)", "approved", 2 * DAY + 1 * HR),
  ],
  voiceProfile: v("neutral", 55, [
    "Happy to walk you through the listing",
    "Per our terms",
    "Let me check with the category buyer",
  ]),
};

const JAMAL: WorkerMemory = {
  namespaceId: "ns_jamal_3c19",
  tenantScope: "Tenant-scoped ✓",
  facts: [
    f("f_jamal_1", "Refunds under $50 auto-approve; $50-$500 require operator confirmation; >$500 escalate to Marta.", 4 * DAY),
    f("f_jamal_2", "Billing disputes always escalate — never resolve directly even when the answer is obvious.", 10 * DAY),
    f("f_jamal_3", "Churn-risk accounts (>$10K ARR) get a CS handoff within 2 hours of any complaint.", 16 * DAY),
    f("f_jamal_4", "Send CSAT survey only after the ticket is fully closed, not on partial resolution.", 21 * DAY),
    f("f_jamal_5", "Use 'order' not 'transaction' in customer-facing copy — internal tooling uses 'transaction'.", 28 * DAY),
  ],
  activeConversations: [
    c("c_jamal_1", "user@acme.co", "Password reset — second attempt", 11 * MIN, "/audit/action/ap_jamal_2"),
  ],
  recentDecisions: [
    d("d_jamal_1", "Approved $42 refund on order #18293 (within policy)", "approved", 4 * MIN),
    d("d_jamal_2", "Escalated billing dispute on order #18254 to Marta", "approved", 27 * MIN),
    d("d_jamal_3", "Refused to override $640 refund without operator review", "refused", 3 * HR + 40 * MIN),
  ],
  voiceProfile: v("warm", 30, [
    "Got it — let's get this sorted",
    "Thanks for hanging in there",
    "I'll keep you posted",
  ]),
};

const NORI: WorkerMemory = {
  namespaceId: "ns_nori_6d44",
  tenantScope: "Tenant-scoped ✓",
  facts: [
    f("f_nori_1", "Brand voice: confident, never apologetic. Avoid 'we humbly believe' / 'we feel'.", 6 * DAY),
    f("f_nori_2", "All product announcements require brand lead approval before publish — no exceptions.", 11 * DAY),
    f("f_nori_3", "Use customer name brand 'Skycot Cloud' not 'Skycot Platform' in all customer-facing copy.", 19 * DAY),
    f("f_nori_4", "Schedule sends for the segment's local 9-11am window; never weekends without explicit sign-off.", 24 * DAY),
    f("f_nori_5", "Weekly reach roundup goes to brand lead by Monday 08:00 — include WoW delta + top-performer.", 28 * DAY),
  ],
  activeConversations: [
    c("c_nori_1", "Brand Lead", "Q2 product update — voice tweaks", 28 * MIN, "/audit/action/ap_nori_1"),
  ],
  recentDecisions: [
    d("d_nori_1", "Approved 'May newsletter' tone-check (within brand)", "approved", 60 * MIN),
    d("d_nori_2", "Refused to schedule announcement send on a Saturday", "refused", 3 * HR + 15 * MIN),
    d("d_nori_3", "Approved pricing-page refresh post for Monday 09:00 send", "approved", 8 * HR),
  ],
  voiceProfile: v("formal", 70, ["We're announcing", "Here's what's shipping", "Read the full update"]),
};

const SAM: WorkerMemory = {
  namespaceId: "ns_sam_2e88",
  tenantScope: "Tenant-scoped ✓",
  facts: [
    f("f_sam_1", "Page secondary on-call after 5m no-ack on P1/P2 alerts.", 7 * DAY),
    f("f_sam_2", "Never close an incident without an operator-approved post-mortem timeline.", 13 * DAY),
    f("f_sam_3", "PagerDuty integration alias: 'oncall-primary' = current rotation, refreshed nightly.", 17 * DAY),
    f("f_sam_4", "Mute window applies to runbook noise alerts only — never auto-suppress thresholded breaches.", 22 * DAY),
  ],
  activeConversations: [],
  recentDecisions: [
    d("d_sam_1", "Approved post-mortem timeline for INC-0418", "approved", 4 * DAY + 12 * HR),
    d("d_sam_2", "Refused to auto-close INC-0420 without ack", "refused", 6 * DAY + 3 * HR),
  ],
  voiceProfile: v("neutral", 60, ["Acked", "Paging now", "Timeline updated"]),
};

const RIO: WorkerMemory = {
  namespaceId: "ns_rio_9a55",
  tenantScope: "Tenant-scoped ✓",
  facts: [
    f("f_rio_1", "Showings require listing agent slot confirmation BEFORE buyer commitment.", 9 * DAY),
    f("f_rio_2", "Send reminder 2 hours before showing; never reschedule without buyer + agent agreement.", 15 * DAY),
    f("f_rio_3", "Use 'showing' not 'viewing' externally per listing-agent preference.", 22 * DAY),
  ],
  activeConversations: [],
  recentDecisions: [],
  voiceProfile: v("warm", 40, [
    "Happy to find a time that works", "I'll confirm with the agent", "See you then",
  ]),
};

const MEMORY_BY_ID: Record<string, WorkerMemory> = {
  wkr_01J0PRIYA: PRIYA, wkr_01J0MARIAH: MARIAH, wkr_01J0JAMAL: JAMAL,
  wkr_01J0NORI: NORI, wkr_01J0SAM: SAM, wkr_01J0RIO: RIO,
};

export function findSampleWorkerMemory(id: string): WorkerMemory | undefined {
  return MEMORY_BY_ID[id];
}

export const SAMPLE_WORKER_MEMORY_IDS = Object.keys(MEMORY_BY_ID);

let factCounter = 1000;
function nextFactId(): string {
  factCounter += 1;
  return `f_mock_${factCounter}`;
}

export function mockAddFact(workerId: string, text: string): WorkerFact {
  const memory = MEMORY_BY_ID[workerId];
  if (!memory) throw new WorkerMemoryNotFoundError(workerId);
  const created: WorkerFact = { factId: nextFactId(), text, createdAt: new Date().toISOString() };
  memory.facts = [created, ...memory.facts];
  return created;
}

export function mockEditFact(workerId: string, factId: string, text: string): WorkerFact {
  const memory = MEMORY_BY_ID[workerId];
  if (!memory) throw new WorkerMemoryNotFoundError(workerId);
  const idx = memory.facts.findIndex((row) => row.factId === factId);
  if (idx === -1) throw new Error(`Fact '${factId}' not found`);
  const next: WorkerFact = { ...memory.facts[idx], text };
  memory.facts = [...memory.facts.slice(0, idx), next, ...memory.facts.slice(idx + 1)];
  return next;
}

export function mockDeleteFact(workerId: string, factId: string): void {
  const memory = MEMORY_BY_ID[workerId];
  if (!memory) throw new WorkerMemoryNotFoundError(workerId);
  memory.facts = memory.facts.filter((row) => row.factId !== factId);
}
