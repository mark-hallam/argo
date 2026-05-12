/**
 * Mock overview payloads for the 6 detailed sample workers from
 * `sample-workers-detail.ts`. Used when the gateway endpoint
 * `/api/v1/workers/<id>/overview` returns 404 in dev (or when
 * `NEXT_PUBLIC_GATEWAY_AUTH_BYPASS=1`).
 *
 * Enterprise-flavored framing per the brief:
 *   - Charter speaks team-first ("Mariah helps the Procurement team")
 *     rather than worker-first ("Mariah lists items").
 *   - KPIs name vertical-specific operational metrics, not generic
 *     "tasks done" counters.
 *   - Recent activity rows link to `/audit/action/<id>` so the audit
 *     drill-in (E5) is the canonical action-detail surface.
 *
 * Each entry pairs with a `Worker` row by id; one inert entry (Rio)
 * exercises the paused-worker render path with reduced activity.
 */

import type {
  WorkerActivityRow,
  WorkerDuty,
  WorkerKpi,
  WorkerOverview,
} from "@/lib/api/worker-overview";

function activity(
  id: string,
  minutesAgo: number,
  verb: string,
  status: string,
): WorkerActivityRow {
  const at = new Date(Date.now() - minutesAgo * 60_000);
  at.setSeconds(0, 0);
  return { id, at: at.toISOString(), verb, status, href: `/audit/action/${id}` };
}

function duty(id: string, label: string, schedule: string, enabled = true): WorkerDuty {
  return { id, label, schedule, enabled };
}

function kpi(
  id: string,
  label: string,
  value: string,
  series: number[],
  deltaPct: number,
): WorkerKpi {
  const trend = deltaPct > 1 ? "up" : deltaPct < -1 ? "down" : "flat";
  const tone = trend === "up" ? "success" : trend === "down" ? "warning" : "muted";
  return { id, label, value, series, deltaPct, trend, tone };
}

const PRIYA: WorkerOverview = {
  charter:
    "Priya helps the People Ops team manage inbound applications. She screens candidates against the role's must-haves, schedules first-round interviews, and asks the hiring manager before agreeing to anything outside the JD.",
  duties: [
    duty("d-screen", "Screen inbound applicants against role must-haves", "as applications arrive"),
    duty("d-shortlist", "Compile next-day shortlist for hiring manager review", "weekdays 17:30"),
    duty("d-schedule", "Book first-round interviews from shortlist", "weekdays 09:00–17:00"),
    duty("d-rejection", "Draft rejection notes for declined candidates", "weekdays 08:30", false),
  ],
  kpis: [
    kpi("k-screened", "candidates screened today", "8", [4, 6, 5, 7, 9, 11, 8], 12),
    kpi("k-interviews", "interviews booked this week", "14", [2, 3, 2, 3, 2, 1, 1], -8),
    kpi("k-drafts", "pending rejection drafts", "3", [5, 4, 4, 3, 3, 3, 3], -22),
    kpi("k-response", "time-to-first-response avg", "4m 21s", [9, 7, 6, 6, 5, 5, 4], -38),
  ],
  recentActivity: [
    activity("ap_priya_1", 14, "booked interview with candidate #4821", "approved by Marta"),
    activity("ap_priya_2", 38, "screened candidate #4820 — recommended advance", "auto-sent"),
    activity("ap_priya_3", 72, "screened candidate #4819 — soft no", "approved by Marta"),
    activity("ap_priya_4", 145, "drafted rejection for candidate #4811", "needs review"),
    activity("ap_priya_5", 210, "asked hiring manager: relocation budget?", "awaiting reply"),
  ],
};

const MARIAH: WorkerOverview = {
  charter:
    "Mariah helps the Procurement team keep the marketplace storefront fresh. She lists new inventory, re-prices stale items against competitor data, and asks the category buyer before discounting anything below floor.",
  duties: [
    duty("d-list", "List new inventory with photos + tags", "as items arrive"),
    duty("d-reprice", "Re-price stale listings against competitor data", "daily 06:30"),
    duty("d-buyer-chat", "Respond to buyer questions on active listings", "as messages arrive"),
    duty("d-floor", "Flag items dropping below floor price", "daily 06:30"),
  ],
  kpis: [
    kpi("k-live", "listings live", "3", [1, 2, 2, 3, 3, 3, 3], 33),
    kpi("k-replies", "buyer replies sent today", "11", [6, 8, 9, 10, 12, 11, 11], 6),
    kpi("k-floor", "items flagged below floor", "2", [0, 1, 1, 2, 3, 2, 2], 0),
    kpi("k-cycle", "median time-to-list", "9m", [18, 14, 12, 11, 10, 10, 9], -28),
  ],
  recentActivity: [
    activity("ap_mariah_1", 2, "replied to buyer #4821 on listing 'Eames lounge'", "approved by Sam"),
    activity("ap_mariah_2", 18, "re-priced listing 'Herman Miller chair' −$45", "auto-sent"),
    activity("ap_mariah_3", 41, "listed 'Vintage Bauhaus desk' with 6 photos", "approved by Sam"),
    activity("ap_mariah_4", 96, "flagged 'Knoll side table' below floor", "needs review"),
    activity("ap_mariah_5", 175, "replied to buyer #4799 on listing 'Saarinen tulip'", "auto-sent"),
  ],
};

const JAMAL: WorkerOverview = {
  charter:
    "Jamal helps the Customer Experience team keep tier-1 ticket volume manageable. He resolves the standard refund / password / shipping cases end-to-end, and escalates anything touching billing disputes or churn-risk accounts to a human.",
  duties: [
    duty("d-tier1", "Resolve tier-1 tickets (refund / password / shipping)", "as tickets arrive"),
    duty("d-triage", "Triage incoming tickets into queue buckets", "every 5 min"),
    duty("d-escalate", "Escalate billing disputes + churn risks", "as detected"),
    duty("d-csat", "Send CSAT survey after resolution", "on close"),
  ],
  kpis: [
    kpi("k-resolved", "tickets resolved today", "12", [8, 10, 11, 9, 14, 12, 12], 14),
    kpi("k-ttr", "avg time-to-resolution", "4m 02s", [7, 6, 6, 5, 5, 4, 4], -33),
    kpi("k-tier2", "tier-2 escalations today", "1", [3, 2, 2, 1, 1, 1, 1], -50),
    kpi("k-csat", "CSAT this week", "4.6", [4.3, 4.4, 4.4, 4.5, 4.5, 4.6, 4.6], 5),
  ],
  recentActivity: [
    activity("ap_jamal_1", 4, "issued refund $42 — order #18293", "auto-sent"),
    activity("ap_jamal_2", 11, "reset password for user@acme.co", "auto-sent"),
    activity("ap_jamal_3", 27, "escalated billing dispute on order #18254", "approved by Marta"),
    activity("ap_jamal_4", 52, "resolved shipping query — order #18247", "auto-sent"),
    activity("ap_jamal_5", 88, "sent CSAT to user@globex.io", "auto-sent"),
  ],
};

const NORI: WorkerOverview = {
  charter:
    "Nori helps the Brand team keep customer-facing comms on-voice and on-schedule. She drafts product announcements, asks the brand lead before publishing anything net-new, and tracks reach against the team's quarterly target.",
  duties: [
    duty("d-draft", "Draft product announcements from changelog entries", "as releases land"),
    duty("d-voice", "Tone-check drafts against brand voice guide", "on draft save"),
    duty("d-schedule", "Schedule send-times based on segment activity windows", "on approval"),
    duty("d-reach", "Roll up weekly reach + engagement for brand lead", "Mondays 08:00"),
  ],
  kpis: [
    kpi("k-reach", "people reached this week", "12,400", [800, 1100, 1600, 2000, 2400, 2300, 2200], 18),
    kpi("k-drafts", "drafts in review", "4", [2, 3, 3, 4, 4, 4, 4], 11),
    kpi("k-eng", "engagement rate this week", "5.8%", [4.2, 4.6, 5.1, 5.3, 5.6, 5.8, 5.8], 9),
    kpi("k-cycle", "draft → approve median", "1h 24m", [180, 160, 140, 130, 110, 90, 84], -27),
  ],
  recentActivity: [
    activity("ap_nori_1", 22, "drafted Q2 product update announcement", "needs review"),
    activity("ap_nori_2", 60, "tone-checked 'May newsletter' draft", "approved by Brand Lead"),
    activity("ap_nori_3", 130, "scheduled send for 'May newsletter' Mon 09:00", "approved by Brand Lead"),
    activity("ap_nori_4", 240, "rolled up weekly reach (+18% wow)", "auto-sent"),
    activity("ap_nori_5", 480, "drafted 'pricing-page refresh' post", "approved by Brand Lead"),
  ],
};

const SAM: WorkerOverview = {
  charter:
    "Sam helps the Reliability team keep the on-call queue moving. He triages alerts against the runbook, opens incidents when thresholds breach, and pages the secondary on-call engineer when the primary doesn't ack within 5 minutes.",
  duties: [
    duty("d-triage", "Triage incoming alerts against runbook", "as alerts arrive"),
    duty("d-incident", "Open incident when threshold breached", "as detected"),
    duty("d-page", "Page secondary on-call after 5m no-ack", "as detected"),
    duty("d-postmortem", "Draft post-incident timeline from logs", "post-resolution", false),
  ],
  kpis: [
    kpi("k-triaged", "alerts triaged today", "0", [12, 8, 4, 2, 1, 0, 0], -100),
    kpi("k-incidents", "incidents opened this week", "0", [2, 1, 1, 0, 0, 0, 0], -100),
    kpi("k-mttr", "median time-to-ack", "—", [6, 5, 4, 4, 3, 0, 0], 0),
    kpi("k-noise", "noisy-alert rate", "—", [12, 10, 8, 5, 3, 0, 0], 0),
  ],
  recentActivity: [
    activity("ap_sam_1", 300, "PagerDuty integration disconnected — needs re-link", "blocked"),
    activity("ap_sam_2", 1440, "triaged alert 'api-latency-p99' — no action", "auto-sent"),
    activity("ap_sam_3", 2880, "opened incident #INC-0421 for 'edge-cdn-5xx'", "approved by Marta"),
    activity("ap_sam_4", 4320, "paged secondary after 5m no-ack on INC-0420", "auto-sent"),
    activity("ap_sam_5", 5760, "drafted timeline for INC-0418", "approved by Marta"),
  ],
};

const RIO: WorkerOverview = {
  charter:
    "Rio helps the listing agent keep showings booked. He proposes time-slots from agent calendar availability and confirms with prospective buyers — currently paused while we tighten the 'who confirms first' rules.",
  duties: [
    duty("d-propose", "Propose showing slots from agent availability", "as buyer requests arrive", false),
    duty("d-confirm", "Confirm with buyer once agent approves slot", "on agent approval", false),
    duty("d-reminder", "Send buyer reminder 2h before showing", "scheduled", false),
  ],
  kpis: [
    kpi("k-booked", "showings booked this week", "—", [3, 2, 2, 1, 0, 0, 0], 0),
    kpi("k-pending", "pending buyer responses", "—", [4, 4, 3, 3, 2, 0, 0], 0),
    kpi("k-decline", "agent declines this week", "—", [0, 1, 0, 1, 0, 0, 0], 0),
    kpi("k-cycle", "request → confirm median", "—", [120, 110, 90, 80, 60, 0, 0], 0),
  ],
  recentActivity: [
    activity("ap_rio_1", 22 * 60, "paused by you", "manual"),
    activity("ap_rio_2", 26 * 60, "confirmed showing for 1428 Oak St", "approved by listing agent"),
    activity("ap_rio_3", 28 * 60, "proposed 3 slots to buyer #2244", "auto-sent"),
    activity("ap_rio_4", 30 * 60, "agent declined Tuesday slot — proposed Thursday", "auto-sent"),
    activity("ap_rio_5", 32 * 60, "sent reminder for 11am showing today", "auto-sent"),
  ],
};

const OVERVIEWS_BY_ID: Record<string, WorkerOverview> = {
  wkr_01J0PRIYA: PRIYA,
  wkr_01J0MARIAH: MARIAH,
  wkr_01J0JAMAL: JAMAL,
  wkr_01J0NORI: NORI,
  wkr_01J0SAM: SAM,
  wkr_01J0RIO: RIO,
};

export function findSampleWorkerOverview(id: string): WorkerOverview | undefined {
  return OVERVIEWS_BY_ID[id];
}

export const SAMPLE_WORKER_OVERVIEW_IDS = Object.keys(OVERVIEWS_BY_ID);
