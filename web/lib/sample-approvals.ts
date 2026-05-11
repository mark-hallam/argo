/**
 * Mock approval data for the inbox demo. Replaced by gateway
 * fetches in the Track E approvals-inbox slice.
 */

import type { ApprovalDecision } from "@/components/skycot/ApprovalCard";

export type ChannelKind = "slack" | "imessage" | "sms" | "push" | "web" | "voice" | "whatsapp";

export interface ApprovalItem {
  id: string;
  worker: { name: string; avatarUrl?: string };
  verb: string;
  detail: string;
  consequenceIfYes: string;
  consequenceIfNo: string;
  reason?: string;
  channel: ChannelKind;
  receivedAt: string; // ISO
  decision?: ApprovalDecision | null;
}

/**
 * Stable demo timestamps. We anchor against a future fixed date so
 * "Today" / "Yesterday" grouping stays meaningful for the next few
 * months without being affected by Date.now() at SSR vs CSR (which
 * caused a hydration mismatch). Real data from the gateway will
 * carry server-canonical timestamps and won't need this.
 *
 * The values below are computed at module load on a deterministic
 * floor-to-minute-of-the-current-hour. Both SSR and CSR run on the
 * same machine in dev, so this produces identical strings — and in
 * prod the data comes from the server response, so the issue is
 * inherently absent.
 */
function stableNow(): number {
  const d = new Date();
  d.setSeconds(0, 0);
  return d.getTime();
}

const now = stableNow();
const minute = 60_000;
const hour = 60 * minute;

export const SAMPLE_APPROVALS: ApprovalItem[] = [
  {
    id: "ap_01H9Z7",
    worker: { name: "Mariah" },
    verb: "wants to send this listing",
    detail:
      'Bauer Vapor X5, used 2x · $90. Reply to buyer: "Yes still available, $90 firm. Pickup in Auckland this weekend."',
    consequenceIfYes: "goes to Facebook Marketplace, visible to buyer in 30s.",
    consequenceIfNo: "I'll wait for your direction.",
    reason: "You said 'always check counter-offers' on May 4.",
    channel: "slack",
    receivedAt: new Date(now - 2 * minute).toISOString(),
  },
  {
    id: "ap_01H9Z6",
    worker: { name: "Priya" },
    verb: "wants to book an interview slot",
    detail:
      "Candidate: Diego Alvarez (Senior Backend, Stripe). Proposed: Thursday May 14, 10:30 AM NZST, 45 min on Google Meet.",
    consequenceIfYes: "calendar invite sent to candidate + your calendar updated.",
    consequenceIfNo: "I'll suggest two alternative times tomorrow.",
    reason: "Senior-role candidates require human confirm per your hiring policy.",
    channel: "imessage",
    receivedAt: new Date(now - 14 * minute).toISOString(),
  },
  {
    id: "ap_01H9Z5",
    worker: { name: "Jamal" },
    verb: "wants to issue a refund",
    detail:
      "Customer #4821 — order $148.50 — shipping damage on arrival, photos attached. Standard refund + 10% goodwill credit.",
    consequenceIfYes: "$163.35 refunded; customer notified; ticket closed.",
    consequenceIfNo: "I'll escalate to your support lead with the photo evidence.",
    reason: "Refunds over $100 require explicit approval.",
    channel: "web",
    receivedAt: new Date(now - 42 * minute).toISOString(),
  },
  {
    id: "ap_01H9Z4",
    worker: { name: "Mariah" },
    verb: "wants to counter-offer at $85",
    detail:
      "Buyer asked $75 for the CCM helmet (your floor is $80). Mariah proposes meeting in the middle at $85 with free local pickup.",
    consequenceIfYes: "counter sent to buyer; deal expected to close today.",
    consequenceIfNo: "I'll hold firm at the $80 floor and decline.",
    reason: "Counter-offers below 90% of asking always need your OK.",
    channel: "push",
    receivedAt: new Date(now - 1 * hour - 18 * minute).toISOString(),
  },
  {
    id: "ap_01H9Z3",
    worker: { name: "Nori" },
    verb: "wants to publish the Q2 product update",
    detail:
      "Drafted email + in-app banner announcing the new audit-log export feature. Sends to 12,400 customers Tuesday at 09:00 PST.",
    consequenceIfYes: "scheduled; you'll see the draft once more before send time.",
    consequenceIfNo: "I'll park it as draft and ping you to revise tomorrow.",
    reason: "All-customer broadcasts always require your sign-off.",
    channel: "slack",
    receivedAt: new Date(now - 3 * hour - 22 * minute).toISOString(),
  },
  {
    id: "ap_01H9Z2",
    worker: { name: "Sam" },
    verb: "wants to escalate to engineering",
    detail:
      "Three customers reported the same API timeout in the last hour (us-east-1). I've gathered logs + reproduction steps.",
    consequenceIfYes: "PagerDuty incident opened, on-call engineer paged.",
    consequenceIfNo: "I'll keep monitoring and re-escalate if it hits a fourth customer.",
    reason: "On-call pages require approval outside business hours.",
    channel: "sms",
    receivedAt: new Date(now - 5 * hour).toISOString(),
  },
  {
    id: "ap_01H9Z1",
    worker: { name: "Priya" },
    verb: "wants to send a rejection email",
    detail:
      "Candidate Sara Lin — backend role, was a near-miss but team picked a stronger candidate. Drafted a warm rejection with feedback.",
    consequenceIfYes: "email sent; candidate moved to 'rejected — warm pool' for future roles.",
    consequenceIfNo: "I'll hold and you can review the wording first.",
    reason: "Rejection emails always require your eyes — these are sensitive.",
    channel: "imessage",
    receivedAt: new Date(now - 22 * hour).toISOString(),
  },
];

/**
 * Group approvals by "today" / "yesterday" / "earlier this week" / older.
 * UI hint: render each group with a sticky header.
 */
export function groupApprovals(items: ApprovalItem[]) {
  const today: ApprovalItem[] = [];
  const yesterday: ApprovalItem[] = [];
  const earlier: ApprovalItem[] = [];

  const startOfDay = new Date();
  startOfDay.setHours(0, 0, 0, 0);
  const startOfYesterday = new Date(startOfDay);
  startOfYesterday.setDate(startOfYesterday.getDate() - 1);

  for (const item of items) {
    const t = new Date(item.receivedAt).getTime();
    if (t >= startOfDay.getTime()) today.push(item);
    else if (t >= startOfYesterday.getTime()) yesterday.push(item);
    else earlier.push(item);
  }

  return { today, yesterday, earlier };
}
