/**
 * Worker trust-grade API client (mission-trust-grade-ui).
 *
 * Backs the read-only Trust-grade tab on the worker profile.
 * Mirrors the 404→mock-fallback contract used by `worker-activity.ts`
 * / `team.ts` so dev/demo modes work without a running gateway. The
 * gateway endpoint is `GET /api/v1/workers/<id>/trust-grade`
 * (mission-trust-grade-ui).
 */
import { apiGet } from "@/lib/api/client";
import { ApiError } from "@/lib/api/types";
import { findSampleWorkerTrust } from "@/lib/sample-worker-trust";

/** Canonical trust-grade values returned by the gateway. The names
 *  come straight from `argo_skill::TrustGrade`. */
export type TrustGradeId =
  | "learned_unreviewed"
  | "published_community"
  | "tenant"
  | "published_verified"
  | "builtin";

export const TRUST_GRADE_ORDER: readonly TrustGradeId[] = [
  "learned_unreviewed",
  "published_community",
  "tenant",
  "published_verified",
  "builtin",
] as const;

export type TrustGradeTone = "success" | "warning" | "destructive";

export interface TrustGradeCriterion {
  name: string;
  currentValue: string;
  targetValue: string;
  met: boolean;
}

export interface TrustGradeSignals {
  trustGradeValue: number;
  hasBeenUsed: boolean;
  lastActiveDaysAgo: number | null;
}

export interface TrustGradeSnapshot {
  workerId: string;
  currentGrade: TrustGradeId;
  currentGradeLabel: string;
  currentGradeTone: TrustGradeTone;
  currentGradeIndex: number;
  nextGrade: TrustGradeId | null;
  nextGradeLabel: string | null;
  atMaxGrade: boolean;
  signals: TrustGradeSignals;
  nextGradeCriteria: TrustGradeCriterion[];
}

export type TrustGradeSource = "gateway" | "mock-offline" | "mock-bypass";

export interface FetchTrustGradeResult {
  snapshot: TrustGradeSnapshot;
  source: TrustGradeSource;
}

function isStrictProduction(): boolean {
  return (
    process.env.NODE_ENV === "production" &&
    process.env.NEXT_PUBLIC_ENV === "production"
  );
}

function isAuthBypassed(): boolean {
  return (
    process.env.NODE_ENV !== "production" &&
    process.env.NEXT_PUBLIC_GATEWAY_AUTH_BYPASS === "1"
  );
}

function isMissingOrUnreachable(err: unknown): boolean {
  if (err instanceof ApiError) return err.status === 404;
  return err instanceof TypeError;
}

interface GatewayCriterion {
  name?: unknown;
  current_value?: unknown;
  target_value?: unknown;
  met?: unknown;
}

interface GatewaySignals {
  trust_grade_value?: unknown;
  has_been_used?: unknown;
  last_active_days_ago?: unknown;
}

interface GatewayTrustGradeBody {
  worker_id?: unknown;
  current_grade?: unknown;
  current_grade_label?: unknown;
  current_grade_tone?: unknown;
  current_grade_index?: unknown;
  next_grade?: unknown;
  next_grade_label?: unknown;
  at_max_grade?: unknown;
  signals?: GatewaySignals;
  next_grade_criteria?: GatewayCriterion[];
}

function isTrustGradeId(value: unknown): value is TrustGradeId {
  return typeof value === "string" && (TRUST_GRADE_ORDER as readonly string[]).includes(value);
}

function asTone(value: unknown): TrustGradeTone {
  if (value === "success" || value === "warning" || value === "destructive") return value;
  return "warning";
}

/** Map the gateway's snake_case JSON to the camelCase SPA shape.
 *  Exported so the test suite can drive the converter without
 *  going through `fetch`. */
export function normalizeTrustGrade(
  body: GatewayTrustGradeBody,
  workerIdFallback: string,
): TrustGradeSnapshot {
  const currentGrade = isTrustGradeId(body.current_grade)
    ? body.current_grade
    : "learned_unreviewed";
  const nextGrade = isTrustGradeId(body.next_grade) ? body.next_grade : null;
  const signals = body.signals ?? {};
  const criteria = Array.isArray(body.next_grade_criteria)
    ? body.next_grade_criteria.map((c) => ({
        name: typeof c.name === "string" ? c.name : "",
        currentValue: typeof c.current_value === "string" ? c.current_value : "",
        targetValue: typeof c.target_value === "string" ? c.target_value : "",
        met: c.met === true,
      }))
    : [];
  return {
    workerId: typeof body.worker_id === "string" ? body.worker_id : workerIdFallback,
    currentGrade,
    currentGradeLabel:
      typeof body.current_grade_label === "string" ? body.current_grade_label : currentGrade,
    currentGradeTone: asTone(body.current_grade_tone),
    currentGradeIndex:
      typeof body.current_grade_index === "number" ? body.current_grade_index : 0,
    nextGrade,
    nextGradeLabel: typeof body.next_grade_label === "string" ? body.next_grade_label : null,
    atMaxGrade: body.at_max_grade === true,
    signals: {
      trustGradeValue:
        typeof signals.trust_grade_value === "number" ? signals.trust_grade_value : 0,
      hasBeenUsed: signals.has_been_used === true,
      lastActiveDaysAgo:
        typeof signals.last_active_days_ago === "number" ? signals.last_active_days_ago : null,
    },
    nextGradeCriteria: criteria,
  };
}

export async function fetchWorkerTrustGrade(workerId: string): Promise<FetchTrustGradeResult> {
  if (isAuthBypassed()) {
    return { snapshot: findSampleWorkerTrust(workerId), source: "mock-bypass" };
  }
  const path = `/api/v1/workers/${encodeURIComponent(workerId)}/trust-grade`;
  try {
    const raw = await apiGet<GatewayTrustGradeBody>(path);
    return { snapshot: normalizeTrustGrade(raw, workerId), source: "gateway" };
  } catch (err) {
    if (!isStrictProduction() && isMissingOrUnreachable(err)) {
      return { snapshot: findSampleWorkerTrust(workerId), source: "mock-offline" };
    }
    throw err;
  }
}
