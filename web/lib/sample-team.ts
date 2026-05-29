/**
 * Mock teammate roster for the Team page demo. Returned by
 * `fetchTeam()` when the gateway returns 404 or when the dev
 * auth-bypass flag is on. Replaced by the gateway response
 * once the sibling `mission-gateway-team-list-endpoint` brief
 * lands.
 *
 * Persona spine (name + role + vertical + headline + traits) is
 * sourced from `sample-personas.ts` so /team agrees with /audit and
 * /workers/<id>/overview. `lastActiveAt` and the canonical status
 * (working / waiting / idle / blocked / paused) live inline here in
 * the overlay below — this is the single source of truth for what
 * the /team card pill displays. `pendingApprovalCount` is *derived*
 * from `SAMPLE_APPROVALS` by name match so the /team Worker.count
 * field cannot disagree with the /approvals inbox by construction
 * (fix-approvals-inbox-waiting-on-you-fixture-drift).
 *
 * Status and pending-count are *independent* signals — a worker can
 * be "working" while still having pending approvals queued (the count
 * surfaces via `pendingApprovalCount`; the pill reflects the worker's
 * actual runtime state). The earlier pending-→-"waiting" override
 * collapsed every persona to "waiting" because every sample persona
 * has at least one pending approval, defeating the "seven teammates,
 * one team" deck design (fix-teammate-card-all-five-show-waiting-on-you).
 *
 * Same fixed-timestamp pattern as `sample-approvals.ts`: anchor
 * every `lastActiveAt` to a stable floor-of-minute so SSR and CSR
 * render identical strings and don't trip a hydration mismatch.
 */

import { SAMPLE_APPROVALS, type ApprovalItem } from "@/lib/sample-approvals";
import { getSamplePersona, SAMPLE_PERSONA_IDS } from "@/lib/sample-personas";
import type { Worker, WorkerStatus } from "@/lib/types/worker";

function stableNow(): number {
  const d = new Date();
  d.setSeconds(0, 0);
  return d.getTime();
}

const now = stableNow();
const minute = 60_000;
const hour = 60 * minute;

interface RuntimeOverlay {
  /** Canonical status shown on the /team card pill. */
  status: WorkerStatus;
  lastActiveAt: string;
}

const RUNTIME_OVERLAY: Record<string, RuntimeOverlay> = {
  "019e6785-928a-7270-a766-f945b6cd7161": {
    // Mariah — waiting on operator OK for two pending listing actions.
    status: "waiting",
    lastActiveAt: new Date(now - 2 * minute).toISOString(),
  },
  "019e6785-928d-78b1-a6b2-136d9ed263ce": {
    // Priya — waiting on operator for the interview-slot booking.
    status: "waiting",
    lastActiveAt: new Date(now - 14 * minute).toISOString(),
  },
  "019e6785-928e-7f20-803b-55266e526011": {
    // Jamal — actively handling tickets; one refund queued for review.
    status: "working",
    lastActiveAt: new Date(now - 32 * minute).toISOString(),
  },
  "019e6785-928f-7281-bf8d-e062bc277478": {
    // Nori — between campaigns; next draft scheduled tomorrow.
    status: "idle",
    lastActiveAt: new Date(now - 3 * hour).toISOString(),
  },
  "019e6785-928c-72b3-85c6-436fe5004c50": {
    // Peta — waiting on operator approval for the P0 incident page.
    status: "waiting",
    lastActiveAt: new Date(now - 6 * minute).toISOString(),
  },
  "019e6785-9290-7a41-9c2e-4b8d10f5c7a2": {
    // Portia — mid-rehearsal, matching investor questions to approved facts.
    status: "working",
    lastActiveAt: new Date(now - 12 * minute).toISOString(),
  },
  "019e6785-9291-7b52-8d3f-6c9e21a6d8b3": {
    // Grant — working the inbound queue; outreach drafts queued for review.
    status: "working",
    lastActiveAt: new Date(now - 9 * minute).toISOString(),
  },
};

function isPending(item: ApprovalItem): boolean {
  return item.decision == null;
}

function pendingCountByName(name: string): number {
  return SAMPLE_APPROVALS.filter(
    (item) => isPending(item) && item.worker.name === name,
  ).length;
}

function buildWorker(id: string): Worker {
  const persona = getSamplePersona(id);
  const overlay = RUNTIME_OVERLAY[id];
  if (!overlay) {
    throw new Error(`Missing sample-team runtime overlay for ${id}`);
  }
  const pending = pendingCountByName(persona.name);
  return {
    id: persona.id,
    name: persona.name,
    role: { tagline: persona.role.tagline, vertical: persona.role.vertical },
    status: overlay.status,
    headline: persona.headline,
    lastActiveAt: overlay.lastActiveAt,
    pendingApprovalCount: pending > 0 ? pending : undefined,
  };
}

export const SAMPLE_TEAM: Worker[] = SAMPLE_PERSONA_IDS.map(buildWorker);
