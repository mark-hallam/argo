/**
 * Mock per-worker audit feed used by the Activity tab when the
 * gateway endpoint `/api/v1/workers/<id>/activity` is unavailable
 * (404 in dev, or the companion brief
 * `mission-gateway-worker-activity-endpoint` hasn't landed yet).
 *
 * Each entry is enterprise-shaped per the E4-a sample workers
 * (Mariah/Priya/Sam/Jamal/Nori) — verbs read like real audit lines
 * rather than placeholder lorem. Verbs match the worker's
 * canonical role in `sample-personas.ts` so the activity tab
 * agrees with /team and /audit. Time-stamps anchor to a stable
 * floor-to-minute boundary so SSR/CSR strings match (same trick
 * as `sample-notifications.ts`).
 */
import { getSamplePersona } from "@/lib/sample-personas";
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
    verb: "triaged alert 'api-latency-p99' against runbook",
    actionType: "message_sent",
    status: "succeeded",
    agoMs: 22 * MINUTE,
    costUsd: 0.006,
  },
  {
    verb: "suppressed flapping alert 'cache-evict-rate'",
    actionType: "refusal_issued",
    status: "refused",
    agoMs: 1 * HOUR + 15 * MINUTE,
  },
  {
    verb: "queued runbook restart (edge-cdn) for SRE approval",
    actionType: "approval_asked",
    status: "pending",
    agoMs: 3 * HOUR,
  },
  {
    verb: "failed to fetch Grafana dashboard — falling back to logs",
    actionType: "message_sent",
    status: "failed",
    agoMs: 1 * DAY + 2 * HOUR,
  },
];

const MARIAH: Seed[] = [
  {
    verb: "drafted listing copy for 'Eames lounge chair'",
    actionType: "draft_queued",
    status: "succeeded",
    agoMs: 38 * MINUTE,
    costUsd: 0.021,
  },
  {
    verb: "scheduled re-price sweep for stale inventory",
    actionType: "message_sent",
    status: "succeeded",
    agoMs: 4 * HOUR,
  },
  {
    verb: "refused to use stock imagery on a live listing",
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
    verb: "tone-checked 'Q2 product update' draft against brand voice",
    actionType: "message_sent",
    status: "succeeded",
    agoMs: 1 * HOUR + 4 * MINUTE,
    costUsd: 0.008,
  },
  {
    verb: "flagged 'AI hype' wording for brand-lead review",
    actionType: "approval_asked",
    status: "pending",
    agoMs: 3 * HOUR,
  },
];

function build(workerId: string, seeds: Seed[]): WorkerActivity[] {
  const persona = getSamplePersona(workerId);
  return seeds.map((seed, i) => ({
    id: `act_${workerId}_${i.toString().padStart(3, "0")}`,
    workerId: persona.id,
    workerName: persona.name,
    workerInitials: persona.initials,
    timestamp: new Date(NOW - seed.agoMs).toISOString(),
    verb: seed.verb,
    actionType: seed.actionType,
    status: seed.status,
    costUsd: seed.costUsd,
    costBreakdown: seed.costBreakdown,
  }));
}

export const SAMPLE_WORKER_ACTIVITY: Record<string, WorkerActivity[]> = {
  "018f67d0-0000-7000-8000-000000000002": build("018f67d0-0000-7000-8000-000000000002", PRIYA),
  "018f67d0-0000-7000-8000-000000000005": build("018f67d0-0000-7000-8000-000000000005", SAM),
  "018f67d0-0000-7000-8000-000000000001": build("018f67d0-0000-7000-8000-000000000001", MARIAH),
  "018f67d0-0000-7000-8000-000000000003": build("018f67d0-0000-7000-8000-000000000003", JAMAL),
  "018f67d0-0000-7000-8000-000000000004": build("018f67d0-0000-7000-8000-000000000004", NORI),
};

/** Look up the canonical sample feed for a worker id (newest first). */
export function findSampleWorkerActivity(workerId: string): WorkerActivity[] {
  const rows = SAMPLE_WORKER_ACTIVITY[workerId] ?? [];
  return [...rows].sort(
    (a, b) => Date.parse(b.timestamp) - Date.parse(a.timestamp),
  );
}
