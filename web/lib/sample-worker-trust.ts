/**
 * Mock per-worker trust-grade snapshots used by the Trust tab when
 * the gateway endpoint is unavailable. Mirrors the structure of
 * `sample-worker-activity.ts`: a per-worker map keyed by the sample
 * worker ids (Priya / Sam / Mariah / Jamal / Nori), plus a generic
 * fallback for any worker id not on the demo list.
 *
 * The samples deliberately span the trust ladder so each badge tone
 * (success / warning / destructive) renders in the demo without
 * needing to seed the database.
 */
import type {
  TrustGradeId,
  TrustGradeSnapshot,
  TrustGradeTone,
} from "@/lib/api/worker-trust";

const LABELS: Record<TrustGradeId, string> = {
  learned_unreviewed: "Learned, unreviewed",
  published_community: "Published community",
  tenant: "Tenant",
  published_verified: "Published verified",
  builtin: "Builtin",
};

const TONES: Record<TrustGradeId, TrustGradeTone> = {
  learned_unreviewed: "destructive",
  published_community: "warning",
  tenant: "warning",
  published_verified: "success",
  builtin: "success",
};

const INDEX: Record<TrustGradeId, number> = {
  learned_unreviewed: 0,
  published_community: 1,
  tenant: 2,
  published_verified: 3,
  builtin: 4,
};

const NEXT: Record<TrustGradeId, TrustGradeId | null> = {
  learned_unreviewed: "published_community",
  published_community: "tenant",
  tenant: "published_verified",
  published_verified: "builtin",
  builtin: null,
};

interface SeedCriterion {
  name: string;
  currentValue: string;
  targetValue: string;
  met: boolean;
}

interface SeedSignals {
  trustGradeValue: number;
  hasBeenUsed: boolean;
  lastActiveDaysAgo: number | null;
}

interface Seed {
  grade: TrustGradeId;
  signals: SeedSignals;
  criteria: SeedCriterion[];
}

function build(workerId: string, seed: Seed): TrustGradeSnapshot {
  const next = NEXT[seed.grade];
  return {
    workerId,
    currentGrade: seed.grade,
    currentGradeLabel: LABELS[seed.grade],
    currentGradeTone: TONES[seed.grade],
    currentGradeIndex: INDEX[seed.grade],
    nextGrade: next,
    nextGradeLabel: next ? LABELS[next] : null,
    atMaxGrade: next === null,
    signals: seed.signals,
    nextGradeCriteria: next ? seed.criteria : [],
  };
}

const PRIYA_SEED: Seed = {
  grade: "tenant",
  signals: { trustGradeValue: 2, hasBeenUsed: true, lastActiveDaysAgo: 0 },
  criteria: [
    {
      name: "Worker active in the last 30 days",
      currentValue: "yes",
      targetValue: "yes",
      met: true,
    },
    {
      name: "Has executed at least one action",
      currentValue: "yes",
      targetValue: "yes",
      met: true,
    },
    {
      name: "Operator-promoted to Published verified",
      currentValue: "pending",
      targetValue: "yes",
      met: false,
    },
  ],
};

const SAM_SEED: Seed = {
  grade: "published_community",
  signals: { trustGradeValue: 1, hasBeenUsed: true, lastActiveDaysAgo: 2 },
  criteria: [
    {
      name: "Worker active in the last 30 days",
      currentValue: "yes",
      targetValue: "yes",
      met: true,
    },
    {
      name: "Has executed at least one action",
      currentValue: "yes",
      targetValue: "yes",
      met: true,
    },
    {
      name: "Operator-promoted to Tenant",
      currentValue: "pending",
      targetValue: "yes",
      met: false,
    },
  ],
};

const MARIAH_SEED: Seed = {
  grade: "published_verified",
  signals: { trustGradeValue: 3, hasBeenUsed: true, lastActiveDaysAgo: 1 },
  criteria: [
    {
      name: "Worker active in the last 30 days",
      currentValue: "yes",
      targetValue: "yes",
      met: true,
    },
    {
      name: "Has executed at least one action",
      currentValue: "yes",
      targetValue: "yes",
      met: true,
    },
    {
      name: "Operator-promoted to Builtin",
      currentValue: "pending",
      targetValue: "yes",
      met: false,
    },
  ],
};

const JAMAL_SEED: Seed = {
  grade: "learned_unreviewed",
  signals: { trustGradeValue: 0, hasBeenUsed: false, lastActiveDaysAgo: null },
  criteria: [
    {
      name: "Worker active in the last 30 days",
      currentValue: "no",
      targetValue: "yes",
      met: false,
    },
    {
      name: "Has executed at least one action",
      currentValue: "no",
      targetValue: "yes",
      met: false,
    },
    {
      name: "Operator-promoted to Published community",
      currentValue: "pending",
      targetValue: "yes",
      met: false,
    },
  ],
};

const NORI_SEED: Seed = {
  grade: "builtin",
  signals: { trustGradeValue: 4, hasBeenUsed: true, lastActiveDaysAgo: 0 },
  criteria: [],
};

export const SAMPLE_WORKER_TRUST: Record<string, TrustGradeSnapshot> = {
  wkr_01J0PRIYA: build("wkr_01J0PRIYA", PRIYA_SEED),
  wkr_01J0SAM: build("wkr_01J0SAM", SAM_SEED),
  wkr_01J0MARIAH: build("wkr_01J0MARIAH", MARIAH_SEED),
  wkr_01J0JAMAL: build("wkr_01J0JAMAL", JAMAL_SEED),
  wkr_01J0NORI: build("wkr_01J0NORI", NORI_SEED),
};

const FALLBACK_SEED: Seed = {
  grade: "learned_unreviewed",
  signals: { trustGradeValue: 0, hasBeenUsed: false, lastActiveDaysAgo: null },
  criteria: [
    {
      name: "Worker active in the last 30 days",
      currentValue: "no",
      targetValue: "yes",
      met: false,
    },
    {
      name: "Has executed at least one action",
      currentValue: "no",
      targetValue: "yes",
      met: false,
    },
    {
      name: "Operator-promoted to Published community",
      currentValue: "pending",
      targetValue: "yes",
      met: false,
    },
  ],
};

/** Look up the canonical sample trust-grade snapshot for a worker id.
 *  Unknown ids fall back to a deterministic Learned-unreviewed
 *  baseline so the SPA never crashes on a new worker. */
export function findSampleWorkerTrust(workerId: string): TrustGradeSnapshot {
  return SAMPLE_WORKER_TRUST[workerId] ?? build(workerId, FALLBACK_SEED);
}
