/**
 * Mock teammate roster for the Team page demo. Returned by
 * `fetchTeam()` when the gateway returns 404 or when the dev
 * auth-bypass flag is on. Replaced by the gateway response
 * once the sibling `mission-gateway-team-list-endpoint` brief
 * lands.
 *
 * Persona spine (name + role + vertical + headline + traits) is
 * sourced from `sample-personas.ts` so /team agrees with /audit and
 * /workers/<id>/overview. Status, lastActiveAt and
 * pendingApprovalCount are runtime-ish and remain inline here.
 *
 * Same fixed-timestamp pattern as `sample-approvals.ts`: anchor
 * every `lastActiveAt` to a stable floor-of-minute so SSR and CSR
 * render identical strings and don't trip a hydration mismatch.
 */

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
  status: WorkerStatus;
  lastActiveAt: string;
  pendingApprovalCount?: number;
}

const RUNTIME_OVERLAY: Record<string, RuntimeOverlay> = {
  "018f67d0-0000-7000-8000-000000000001": {
    status: "waiting",
    lastActiveAt: new Date(now - 2 * minute).toISOString(),
    pendingApprovalCount: 2,
  },
  "018f67d0-0000-7000-8000-000000000002": {
    status: "waiting",
    lastActiveAt: new Date(now - 14 * minute).toISOString(),
    pendingApprovalCount: 1,
  },
  "018f67d0-0000-7000-8000-000000000003": {
    status: "working",
    lastActiveAt: new Date(now - 32 * minute).toISOString(),
  },
  "018f67d0-0000-7000-8000-000000000004": {
    status: "idle",
    lastActiveAt: new Date(now - 3 * hour).toISOString(),
  },
  "018f67d0-0000-7000-8000-000000000007": {
    status: "waiting",
    lastActiveAt: new Date(now - 6 * minute).toISOString(),
    pendingApprovalCount: 2,
  },
};

function buildWorker(id: string): Worker {
  const persona = getSamplePersona(id);
  const overlay = RUNTIME_OVERLAY[id];
  if (!overlay) {
    throw new Error(`Missing sample-team runtime overlay for ${id}`);
  }
  return {
    id: persona.id,
    name: persona.name,
    role: { tagline: persona.role.tagline, vertical: persona.role.vertical },
    status: overlay.status,
    headline: persona.headline,
    lastActiveAt: overlay.lastActiveAt,
    pendingApprovalCount: overlay.pendingApprovalCount,
  };
}

export const SAMPLE_TEAM: Worker[] = SAMPLE_PERSONA_IDS.map(buildWorker);
