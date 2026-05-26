/**
 * Mock teammate roster for the Team page demo. Returned by
 * `fetchTeam()` when the gateway returns 404 or when the dev
 * auth-bypass flag is on. Replaced by the gateway response
 * once the sibling `mission-gateway-team-list-endpoint` brief
 * lands.
 *
 * Persona spine (name + role + vertical + headline + traits) is
 * sourced from `sample-personas.ts` so /team agrees with /audit and
 * /workers/<id>/overview. `lastActiveAt` and the baseline
 * working/idle status remain inline here; `pendingApprovalCount`
 * (and the "waiting on you" status override that flows from it)
 * is *derived* from `SAMPLE_APPROVALS` so the /team chip count
 * and the /approvals inbox cannot disagree by construction —
 * fix-approvals-inbox-waiting-on-you-fixture-drift.
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
  /** Status to use when the worker has no pending approvals. */
  baselineStatus: WorkerStatus;
  lastActiveAt: string;
}

const RUNTIME_OVERLAY: Record<string, RuntimeOverlay> = {
  "018f67d0-0000-7000-8000-000000000001": {
    baselineStatus: "working",
    lastActiveAt: new Date(now - 2 * minute).toISOString(),
  },
  "018f67d0-0000-7000-8000-000000000002": {
    baselineStatus: "working",
    lastActiveAt: new Date(now - 14 * minute).toISOString(),
  },
  "018f67d0-0000-7000-8000-000000000003": {
    baselineStatus: "working",
    lastActiveAt: new Date(now - 32 * minute).toISOString(),
  },
  "018f67d0-0000-7000-8000-000000000004": {
    baselineStatus: "idle",
    lastActiveAt: new Date(now - 3 * hour).toISOString(),
  },
  "018f67d0-0000-7000-8000-000000000007": {
    baselineStatus: "working",
    lastActiveAt: new Date(now - 6 * minute).toISOString(),
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
  // A pending approval means the worker is *literally* waiting on
  // the operator — override the baseline so the "Waiting on you"
  // chip cannot diverge from the inbox count.
  const status: WorkerStatus = pending > 0 ? "waiting" : overlay.baselineStatus;
  return {
    id: persona.id,
    name: persona.name,
    role: { tagline: persona.role.tagline, vertical: persona.role.vertical },
    status,
    headline: persona.headline,
    lastActiveAt: overlay.lastActiveAt,
    pendingApprovalCount: pending > 0 ? pending : undefined,
  };
}

export const SAMPLE_TEAM: Worker[] = SAMPLE_PERSONA_IDS.map(buildWorker);
