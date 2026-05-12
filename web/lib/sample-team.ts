/**
 * Mock teammate roster for the Team page demo. Returned by
 * `fetchTeam()` when the gateway returns 404 or when the dev
 * auth-bypass flag is on. Replaced by the gateway response
 * once the sibling `mission-gateway-team-list-endpoint` brief
 * lands.
 *
 * Same fixed-timestamp pattern as `sample-approvals.ts`: anchor
 * every `lastActiveAt` to a stable floor-of-minute so SSR and CSR
 * render identical strings and don't trip a hydration mismatch.
 *
 * Six teammates, matching the five persona archetypes from the
 * personas doc + one inert (paused) worker to exercise that branch
 * of the status filter.
 */

import type { Worker } from "@/lib/types/worker";

function stableNow(): number {
  const d = new Date();
  d.setSeconds(0, 0);
  return d.getTime();
}

const now = stableNow();
const minute = 60_000;
const hour = 60 * minute;

export const SAMPLE_TEAM: Worker[] = [
  {
    id: "wkr_01J0MARIAH",
    name: "Mariah",
    role: { tagline: "lists and re-prices items for sale", vertical: "marketplace" },
    status: "waiting",
    headline: {
      bullets: ["3 listings live", "2 buyers waiting on you"],
      tomorrow: "morning re-price sweep at 06:30",
    },
    lastActiveAt: new Date(now - 2 * minute).toISOString(),
    pendingApprovalCount: 2,
  },
  {
    id: "wkr_01J0PRIYA",
    name: "Priya",
    role: { tagline: "screens candidates and books interviews", vertical: "recruiting" },
    status: "waiting",
    headline: {
      bullets: ["8 candidates screened today", "1 interview pending your slot"],
      tomorrow: "next-day shortlist review",
    },
    lastActiveAt: new Date(now - 14 * minute).toISOString(),
    pendingApprovalCount: 1,
  },
  {
    id: "wkr_01J0JAMAL",
    name: "Jamal",
    role: { tagline: "handles customer support tickets", vertical: "support" },
    status: "working",
    headline: {
      bullets: ["12 tickets resolved", "median reply 4m"],
    },
    lastActiveAt: new Date(now - 32 * minute).toISOString(),
  },
  {
    id: "wkr_01J0NORI",
    name: "Nori",
    role: { tagline: "drafts product comms and announcements", vertical: "marketing" },
    status: "idle",
    headline: {
      bullets: ["last campaign: 12,400 reached"],
      tomorrow: "Q2 product update draft at 09:00",
    },
    lastActiveAt: new Date(now - 3 * hour).toISOString(),
  },
  {
    id: "wkr_01J0SAM",
    name: "Sam",
    role: { tagline: "watches the on-call queue", vertical: "ops" },
    status: "blocked",
    headline: {
      bullets: ["needs PagerDuty re-link"],
    },
    lastActiveAt: new Date(now - 5 * hour).toISOString(),
  },
  {
    id: "wkr_01J0RIO",
    name: "Rio",
    role: { tagline: "books showings for the listing agent", vertical: "real-estate" },
    status: "paused",
    headline: {
      bullets: ["paused by you on May 7"],
    },
    lastActiveAt: new Date(now - 22 * hour).toISOString(),
  },
];
