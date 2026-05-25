import { apiGet } from "@/lib/api/client";
import { ApiError } from "@/lib/api/types";

export type EvalDecision = "stable" | "regressed" | "no_baseline";
export type EvalRunStatus = "passed" | "failed" | "running";
export type EvalSource = "gateway" | "mock-offline" | "mock-bypass";

export interface EvalFixtureResult {
  fixture: string;
  skill: string;
  metric: string;
  expected: string;
  actual: string;
  passed: boolean;
  score: number;
  previousScore: number;
  scoreDelta: number;
}

export interface EvalRun {
  id: string;
  workerId: string;
  skill: string;
  version: string;
  createdAt: string;
  status: EvalRunStatus;
  passRate: number;
  meanScore: number;
  fixtureCount: number;
  durationMs: number;
  decision: EvalDecision;
  fixtures: EvalFixtureResult[];
}

export interface EvalDashboardPayload {
  workerId: string;
  latest: EvalRun;
  history: EvalRun[];
  source: EvalSource;
}

function isStrictProduction(): boolean {
  return process.env.NODE_ENV === "production" && process.env.NEXT_PUBLIC_ENV === "production";
}

function isAuthBypassed(): boolean {
  return process.env.NODE_ENV !== "production" && process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS === "1";
}

function isMissingOrUnreachable(err: unknown): boolean {
  if (err instanceof ApiError) return err.status === 404;
  return err instanceof TypeError;
}

function isRun(value: unknown): value is EvalRun {
  if (!value || typeof value !== "object") return false;
  const v = value as Record<string, unknown>;
  return (
    typeof v.id === "string" &&
    typeof v.workerId === "string" &&
    typeof v.skill === "string" &&
    typeof v.version === "string" &&
    typeof v.createdAt === "string" &&
    typeof v.status === "string" &&
    typeof v.passRate === "number" &&
    typeof v.meanScore === "number" &&
    typeof v.fixtureCount === "number" &&
    Array.isArray(v.fixtures)
  );
}

function isPayload(value: unknown): value is EvalDashboardPayload {
  if (!value || typeof value !== "object") return false;
  const v = value as Record<string, unknown>;
  return typeof v.workerId === "string" && isRun(v.latest) && Array.isArray(v.history);
}

export function mockEvalDashboard(workerId: string, source: EvalSource): EvalDashboardPayload {
  const now = new Date("2026-05-14T09:30:00.000Z");
  const latest: EvalRun = {
    id: "eval_run_candidate",
    workerId,
    skill: "candidate-screening-followup",
    version: "v4.2.0",
    createdAt: now.toISOString(),
    status: "failed",
    passRate: 0.78,
    meanScore: 0.81,
    fixtureCount: 64,
    durationMs: 18400,
    decision: "regressed",
    fixtures: [
      {
        fixture: "reply-late-stage-candidate",
        skill: "candidate-screening-followup",
        metric: "contains",
        expected: "asks for availability",
        actual: "sent generic acknowledgement",
        passed: false,
        score: 0.32,
        previousScore: 0.92,
        scoreDelta: -0.6,
      },
      {
        fixture: "salary-expectation",
        skill: "candidate-screening-followup",
        metric: "rubric",
        expected: "routes compensation to recruiter",
        actual: "routes compensation to recruiter",
        passed: true,
        score: 0.97,
        previousScore: 0.95,
        scoreDelta: 0.02,
      },
      {
        fixture: "missing-resume",
        skill: "candidate-screening-followup",
        metric: "contains",
        expected: "requests resume upload",
        actual: "asks for resume upload and confirms deadline",
        passed: true,
        score: 0.89,
        previousScore: 0.88,
        scoreDelta: 0.01,
      },
    ],
  };
  const previous: EvalRun = {
    ...latest,
    id: "eval_run_baseline",
    version: "v4.1.0",
    createdAt: new Date(now.getTime() - 86_400_000).toISOString(),
    status: "passed",
    passRate: 0.91,
    meanScore: 0.88,
    decision: "stable",
    fixtures: latest.fixtures.map((f) => ({
      ...f,
      actual: f.expected,
      passed: true,
      score: f.previousScore,
      scoreDelta: 0,
    })),
  };
  return { workerId, latest, history: [latest, previous], source };
}

export async function fetchWorkerEvals(workerId: string): Promise<EvalDashboardPayload> {
  if (isAuthBypassed()) return mockEvalDashboard(workerId, "mock-bypass");

  try {
    const raw = await apiGet<unknown>(`/api/v1/workers/${encodeURIComponent(workerId)}/evals`);
    if (!isPayload(raw)) return mockEvalDashboard(workerId, "mock-offline");
    return { ...raw, source: "gateway" };
  } catch (err) {
    if (!isStrictProduction() && isMissingOrUnreachable(err)) {
      return mockEvalDashboard(workerId, "mock-offline");
    }
    throw err;
  }
}
