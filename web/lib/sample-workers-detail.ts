/**
 * Detailed mock workers for the worker-profile page. Each entry
 * mirrors an E3-a `sample-team` row by id but adds the
 * org-hierarchy + kpiToday + permissions fields the profile page
 * needs. Used as the fallback dataset for `useWorker(id)` when the
 * gateway endpoint `/api/v1/workers/<id>` 404s.
 *
 * Enterprise-flavored framing: each worker sits ON a team inside a
 * department. The hero card surfaces both above the role tagline so
 * an operator can tell at a glance where a worker fits in the org.
 * One inert ("Atlas") entry has no department/team/kpiToday — the
 * hero must fall back gracefully when those fields are absent.
 */

import {
  DEFAULT_DELEGATOR_PERMISSIONS,
  type Worker,
} from "@/lib/types/worker";

function stableNow(): number {
  const d = new Date();
  d.setSeconds(0, 0);
  return d.getTime();
}

const now = stableNow();
const minute = 60_000;
const hour = 60 * minute;

export const SAMPLE_WORKERS_DETAIL: Worker[] = [
  {
    id: "wkr_01J0MARIAH",
    name: "Mariah",
    role: { tagline: "lists and re-prices items for sale", vertical: "marketplace" },
    status: "waiting",
    department: { id: "operations", name: "Operations" },
    team: { id: "procurement", name: "Procurement" },
    kpiToday: { label: "listings live", value: "3" },
    permissions: DEFAULT_DELEGATOR_PERMISSIONS,
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
    department: { id: "people-and-culture", name: "People & Culture" },
    team: { id: "people-ops", name: "People Ops" },
    kpiToday: { label: "candidates screened", value: "8" },
    permissions: DEFAULT_DELEGATOR_PERMISSIONS,
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
    department: { id: "customer-success", name: "Customer Success" },
    team: { id: "customer-experience", name: "Customer Experience" },
    kpiToday: { label: "tickets resolved", value: "12" },
    permissions: DEFAULT_DELEGATOR_PERMISSIONS,
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
    department: { id: "marketing", name: "Marketing" },
    team: { id: "brand", name: "Brand" },
    kpiToday: { label: "people reached", value: "12,400" },
    permissions: DEFAULT_DELEGATOR_PERMISSIONS,
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
    department: { id: "operations", name: "Operations" },
    team: { id: "reliability", name: "Reliability" },
    kpiToday: { label: "incidents triaged", value: "0" },
    permissions: DEFAULT_DELEGATOR_PERMISSIONS,
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
    // Intentionally NO department / team / kpiToday / permissions —
    // exercises the "missing-fields" fallback in WorkerHero.
    headline: {
      bullets: ["paused by you on May 7"],
    },
    lastActiveAt: new Date(now - 22 * hour).toISOString(),
  },
];

export function findSampleWorker(id: string): Worker | undefined {
  return SAMPLE_WORKERS_DETAIL.find((w) => w.id === id);
}
