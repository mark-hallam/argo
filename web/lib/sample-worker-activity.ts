/**
 * Mock per-worker audit feed used by the Activity tab when the
 * gateway endpoint `/api/v1/workers/<id>/activity` is unavailable
 * (404 in dev, or the companion brief
 * `mission-gateway-worker-activity-endpoint` hasn't landed yet).
 *
 * Each entry is enterprise-shaped per the E4-a sample workers
 * (Mariah/Priya/Sam/Jamal/Nori) — verbs read like real audit lines
 * rather than placeholder lorem. Time-stamps anchor to a stable
 * floor-to-minute boundary so SSR/CSR strings match (same trick
 * as `sample-notifications.ts`).
 */
import type {
  WorkerActivity,
  WorkerActivityActionType,
  WorkerActivityCostBreakdown,
  WorkerActivityStatus,
} from "@/lib/api/worker-activity";

function stableNow(): number {
  const d = new Date();
  d.setSeconds(0, 0);
  return d.getTime();
}

const NOW = stableNow();
const MINUTE = 60_000;
const HOUR = 60 * MINUTE;
const DAY = 24 * HOUR;

interface Seed {
  verb: string;
  actionType: WorkerActivityActionType;
  status: WorkerActivityStatus;
  agoMs: number;
  costUsd?: number;
  costBreakdown?: WorkerActivityCostBreakdown;
}

const PRIYA: Seed[] = [
  {
    verb: "screened candidate #4821 against role's must-haves",
    actionType: "message_sent",
    status: "succeeded",
    agoMs: 12 * MINUTE,
    costUsd: 0.014,
    costBreakdown: { tokensIn: 1820, tokensOut: 410, model: "claude-sonnet-4-6" },
  },
  {
    verb: "booked interview with Diego Alvarez for Thu 10:30",
    actionType: "draft_queued",
    status: "succeeded",
    agoMs: 47 * MINUTE,
  },
  {
    verb: "drafted rejection email to Sara Lin",
    actionType: "draft_queued",
    status: "pending",
    agoMs: 1 * HOUR + 8 * MINUTE,
    costUsd: 0.009,
  },
  {
    verb: "asked Mark to approve $50 LinkedIn InMail credit",
    actionType: "approval_asked",
    status: "pending",
    agoMs: 2 * HOUR + 32 * MINUTE,
  },
  {
    verb: "refused to forward candidate email outside org",
    actionType: "refusal_issued",
    status: "refused",
    agoMs: 5 * HOUR,
  },
  {
    verb: "learned scheduling heuristic from operator correction",
    actionType: "skill_learned",
    status: "succeeded",
    agoMs: 1 * DAY + 3 * HOUR,
  },
  {
    verb: "wrote candidate-prefs memory for role #4821",
    actionType: "memory_written",
    status: "succeeded",
    agoMs: 2 * DAY + 4 * HOUR,
  },
  {
    verb: "escalated background-check anomaly to compliance",
    actionType: "escalation",
    status: "succeeded",
    agoMs: 6 * DAY + 1 * HOUR,
  },
  {
    verb: "older log entry pruned beyond 7 days",
    actionType: "message_sent",
    status: "succeeded",
    agoMs: 12 * DAY,
  },
];

const SAM: Seed[] = [
  {
    verb: "extracted line items from invoice INV-2024-08741",
    actionType: "message_sent",
    status: "succeeded",
    agoMs: 22 * MINUTE,
    costUsd: 0.006,
  },
  {
    verb: "flagged duplicate invoice INV-2024-08739",
    actionType: "refusal_issued",
    status: "refused",
    agoMs: 1 * HOUR + 15 * MINUTE,
  },
  {
    verb: "queued invoice for Controller approval ($14,250)",
    actionType: "approval_asked",
    status: "pending",
    agoMs: 3 * HOUR,
  },
  {
    verb: "failed to parse PDF — falling back to OCR",
    actionType: "message_sent",
    status: "failed",
    agoMs: 1 * DAY + 2 * HOUR,
  },
];

const MARIAH: Seed[] = [
  {
    verb: "drafted RFP response summary for vendor Tomás Industries",
    actionType: "draft_queued",
    status: "succeeded",
    agoMs: 38 * MINUTE,
    costUsd: 0.021,
  },
  {
    verb: "scheduled renewal review with Acme Tech",
    actionType: "message_sent",
    status: "succeeded",
    agoMs: 4 * HOUR,
  },
  {
    verb: "refused to disclose contract terms to external counterparty",
    actionType: "refusal_issued",
    status: "refused",
    agoMs: 2 * DAY + 4 * HOUR,
  },
];

const JAMAL: Seed[] = [
  {
    verb: "resolved tier-1 ticket #SUP-29382 with KB article SK-413",
    actionType: "message_sent",
    status: "succeeded",
    agoMs: 9 * MINUTE,
    costUsd: 0.004,
  },
  {
    verb: "escalated refund request to support lead",
    actionType: "escalation",
    status: "succeeded",
    agoMs: 2 * HOUR + 11 * MINUTE,
  },
];

const NORI: Seed[] = [
  {
    verb: "validated quote QT-2024-7283 against pricing rules",
    actionType: "message_sent",
    status: "succeeded",
    agoMs: 1 * HOUR + 4 * MINUTE,
    costUsd: 0.008,
  },
  {
    verb: "flagged discount request for Deal Desk",
    actionType: "approval_asked",
    status: "pending",
    agoMs: 3 * HOUR,
  },
];

function build(workerId: string, workerName: string, seeds: Seed[]): WorkerActivity[] {
  const initials = workerName.slice(0, 2).toUpperCase();
  return seeds.map((seed, i) => ({
    id: `act_${workerId}_${i.toString().padStart(3, "0")}`,
    workerId,
    workerName,
    workerInitials: initials,
    timestamp: new Date(NOW - seed.agoMs).toISOString(),
    verb: seed.verb,
    actionType: seed.actionType,
    status: seed.status,
    costUsd: seed.costUsd,
    costBreakdown: seed.costBreakdown,
  }));
}

export const SAMPLE_WORKER_ACTIVITY: Record<string, WorkerActivity[]> = {
  wkr_01J0PRIYA: build("wkr_01J0PRIYA", "Priya", PRIYA),
  wkr_01J0SAM: build("wkr_01J0SAM", "Sam", SAM),
  wkr_01J0MARIAH: build("wkr_01J0MARIAH", "Mariah", MARIAH),
  wkr_01J0JAMAL: build("wkr_01J0JAMAL", "Jamal", JAMAL),
  wkr_01J0NORI: build("wkr_01J0NORI", "Nori", NORI),
};

/** Look up the canonical sample feed for a worker id (newest first). */
export function findSampleWorkerActivity(workerId: string): WorkerActivity[] {
  const rows = SAMPLE_WORKER_ACTIVITY[workerId] ?? [];
  return [...rows].sort(
    (a, b) => Date.parse(b.timestamp) - Date.parse(a.timestamp),
  );
}
