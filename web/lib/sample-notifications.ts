/**
 * Mock notifications data for the bell-popover demo. Replaced by
 * gateway fetches once `J*-gateway-notifications-json-api` lands.
 *
 * "Notifications" here means *system-level workspace events* —
 * distinct from per-action approval requests (those live as cards
 * in the main inbox).
 */

export type NotificationKind =
  | "budget-approaching"
  | "budget-exceeded"
  | "skill-promoted"
  | "skill-drift-detected"
  | "oauth-needs-reauth"
  | "duty-output-ready"
  | "autonomy-promotion-suggested"
  | "incident";

export type NotificationCtaAction = "dismiss" | "callback";

export interface NotificationCta {
  /** Button label, e.g. "Re-auth Gmail". */
  label: string;
  /** If set, the CTA navigates to this URL. */
  href?: string;
  /** Behavior after the user activates the CTA. */
  action?: NotificationCtaAction;
}

export interface NotificationItem {
  id: string;
  kind: NotificationKind;
  title: string;
  /** ISO timestamp; surfaced as a relative time. */
  createdAt: string;
  /** Read = user has seen the popover with this in it. */
  read?: boolean;
  /** Optional inline action. */
  cta?: NotificationCta;
}

/**
 * Stable demo timestamps. We anchor against a deterministic
 * floor-to-minute-of-the-current-hour to keep SSR/CSR strings
 * identical (same pattern as sample-approvals.ts).
 */
function stableNow(): number {
  const d = new Date();
  d.setSeconds(0, 0);
  return d.getTime();
}

const now = stableNow();
const minute = 60_000;
const hour = 60 * minute;

export const SAMPLE_NOTIFICATIONS: NotificationItem[] = [
  {
    id: "nt_01H9N1",
    kind: "oauth-needs-reauth",
    title: "Gmail connector needs re-authentication — token expired 2m ago.",
    createdAt: new Date(now - 2 * minute).toISOString(),
    cta: { label: "Re-auth Gmail", action: "callback" },
  },
  {
    id: "nt_01H9N2",
    kind: "budget-approaching",
    title: "Mariah is at 82% of today's $40 budget.",
    createdAt: new Date(now - 25 * minute).toISOString(),
  },
  {
    id: "nt_01H9N3",
    kind: "skill-promoted",
    title: "Skill `refund-flow` promoted to v3 — Jamal is now using it.",
    createdAt: new Date(now - 3 * hour).toISOString(),
  },
  {
    id: "nt_01H9N4",
    kind: "duty-output-ready",
    title: "Daily revenue digest is ready for review.",
    createdAt: new Date(now - 5 * hour).toISOString(),
    cta: { label: "Open digest", href: "/duties/revenue-digest", action: "dismiss" },
  },
  {
    id: "nt_01H9N5",
    kind: "skill-drift-detected",
    title: "Behavioral drift detected in `support-triage` (eval score -8%).",
    createdAt: new Date(now - 27 * hour).toISOString(),
    cta: { label: "Review drift", href: "/skills/support-triage", action: "dismiss" },
  },
  {
    id: "nt_01H9N6",
    kind: "autonomy-promotion-suggested",
    title: "Trust ratchet suggests promoting Sam to L2 autonomy (38 approvals, 0 reversals).",
    createdAt: new Date(now - 30 * hour).toISOString(),
    read: true,
  },
  {
    id: "nt_01H9N7",
    kind: "budget-exceeded",
    title: "Priya exceeded yesterday's $25 budget by $4.20 (interview-tool overrun).",
    createdAt: new Date(now - 28 * hour).toISOString(),
    read: true,
  },
  {
    id: "nt_01H9N8",
    kind: "incident",
    title: "Gateway returned 502 for 4 calls in a 60s window (us-east-1).",
    createdAt: new Date(now - 4 * 24 * hour).toISOString(),
    read: true,
  },
  {
    id: "nt_01H9N9",
    kind: "duty-output-ready",
    title: "Weekly customer-health report compiled — 3 accounts flagged at-risk.",
    createdAt: new Date(now - 6 * 24 * hour).toISOString(),
    read: true,
    cta: { label: "Open report", href: "/duties/customer-health", action: "dismiss" },
  },
];

/**
 * Group notifications by "today" / "yesterday" / "earlier".
 * UI hint: render each group with its own header.
 */
export function groupNotifications(items: NotificationItem[]) {
  const today: NotificationItem[] = [];
  const yesterday: NotificationItem[] = [];
  const earlier: NotificationItem[] = [];

  const startOfDay = new Date();
  startOfDay.setHours(0, 0, 0, 0);
  const startOfYesterday = new Date(startOfDay);
  startOfYesterday.setDate(startOfYesterday.getDate() - 1);

  for (const item of items) {
    const t = new Date(item.createdAt).getTime();
    if (t >= startOfDay.getTime()) today.push(item);
    else if (t >= startOfYesterday.getTime()) yesterday.push(item);
    else earlier.push(item);
  }

  return { today, yesterday, earlier };
}
