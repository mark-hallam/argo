/**
 * Retrain API client — plain-English worker corrections.
 *
 * The retrain flow (E4-g, journey-retrain-a-worker.md) classifies a
 * delegator's free-text correction into one of five buckets
 * (memory, policy, skill, vocabulary, action-ban), proposes a
 * structural change, surfaces conflicts + regressions, and commits
 * the durable change with a 14-day undo TTL.
 *
 * Endpoints land with sibling brief `mission-gateway-retrain-
 * endpoints`. v1 falls back to a deterministic mock derived from
 * the input string so the dialog flow is testable end-to-end with
 * the gateway offline.
 *
 * Acceptance contract:
 * - Happy path → returns gateway payload with `source: "gateway"`.
 * - 404 / malformed → returns mock payload with `source: "mock"`.
 * - 5xx / network → re-thrown so TanStack Query can retry.
 */

import { apiDelete, apiGet, apiPost } from "@/lib/api/client";
import { ApiError } from "@/lib/api/types";
import { MOCK_REVERSE_CASES, mockProposal, mockRecord } from "@/lib/api/retrain-mocks";

export type {
  ClassifyRetrainRequest,
  ClassifyRetrainResult,
  CommitRetrainRequest,
  CommitRetrainResult,
  ListRecentRetrainsResult,
  RetrainClassification,
  RetrainConflict,
  RetrainProposal,
  RetrainRecord,
  RetrainRegressionDiff,
  RetrainSource,
  ReverseTestCase,
  ReverseTestResult,
} from "@/lib/api/retrain-types";

import type {
  ClassifyRetrainRequest,
  ClassifyRetrainResult,
  CommitRetrainRequest,
  CommitRetrainResult,
  ListRecentRetrainsResult,
  RetrainClassification,
  RetrainProposal,
  RetrainRecord,
  ReverseTestResult,
} from "@/lib/api/retrain-types";

const CLASSIFICATIONS = new Set<RetrainClassification>([
  "memory",
  "policy",
  "skill",
  "vocabulary",
  "action-ban",
]);

function isProposal(value: unknown): value is RetrainProposal {
  if (!value || typeof value !== "object") return false;
  const v = value as Record<string, unknown>;
  return (
    typeof v.classification === "string" &&
    CLASSIFICATIONS.has(v.classification as RetrainClassification) &&
    typeof v.plainEnglish === "string" &&
    typeof v.oneLineSummary === "string" &&
    v.structuralChange !== undefined
  );
}

function isRecord(value: unknown): value is RetrainRecord {
  if (!value || typeof value !== "object") return false;
  const v = value as Record<string, unknown>;
  return (
    typeof v.retrainId === "string" &&
    typeof v.workerId === "string" &&
    typeof v.classification === "string" &&
    typeof v.oneLineSummary === "string" &&
    typeof v.committedAt === "string" &&
    typeof v.undoExpiresAt === "string"
  );
}

function workerPath(workerId: string, suffix: string): string {
  return `/api/v1/workers/${encodeURIComponent(workerId)}${suffix}`;
}

function is404(err: unknown): boolean {
  return err instanceof ApiError && err.status === 404;
}

export async function classifyRetrain(
  req: ClassifyRetrainRequest,
): Promise<ClassifyRetrainResult> {
  try {
    const raw = await apiPost<unknown>(workerPath(req.workerId, "/retrain/classify"), {
      input: req.input,
      triggerContext: req.triggerContext,
    });
    if (!isProposal(raw)) return { proposal: mockProposal(req), source: "mock" };
    return { proposal: raw, source: "gateway" };
  } catch (err) {
    if (is404(err)) return { proposal: mockProposal(req), source: "mock" };
    throw err;
  }
}

export async function commitRetrain(
  req: CommitRetrainRequest,
): Promise<CommitRetrainResult> {
  try {
    const raw = await apiPost<unknown>(workerPath(req.workerId, "/retrain/commit"), {
      proposal: req.proposal,
      acceptedCaseIds: req.acceptedCaseIds,
    });
    if (!isRecord(raw)) return { record: mockRecord(req), source: "mock" };
    return { record: raw, source: "gateway" };
  } catch (err) {
    if (is404(err)) return { record: mockRecord(req), source: "mock" };
    throw err;
  }
}

export async function listRecentRetrains(
  workerId: string,
): Promise<ListRecentRetrainsResult> {
  try {
    const raw = await apiGet<unknown>(workerPath(workerId, "/retrain/recent"));
    if (!Array.isArray(raw) || !raw.every(isRecord)) {
      return { records: [], source: "mock" };
    }
    return { records: raw, source: "gateway" };
  } catch (err) {
    if (is404(err)) return { records: [], source: "mock" };
    throw err;
  }
}

export async function undoRetrain(
  workerId: string,
  retrainId: string,
): Promise<RetrainRecord> {
  try {
    const raw = await apiDelete<unknown>(
      workerPath(workerId, `/retrain/${encodeURIComponent(retrainId)}`),
    );
    if (isRecord(raw)) return raw;
  } catch (err) {
    if (!is404(err)) throw err;
  }
  const now = new Date().toISOString();
  return {
    retrainId,
    workerId,
    classification: "memory",
    oneLineSummary: "(undone)",
    committedAt: now,
    undoExpiresAt: now,
    undoneAt: now,
  };
}

export async function previewReverseTest(
  workerId: string,
  retrainId: string,
): Promise<ReverseTestResult> {
  try {
    const raw = await apiPost<unknown>(
      workerPath(workerId, `/retrain/${encodeURIComponent(retrainId)}/reverse-test`),
      {},
    );
    if (Array.isArray(raw)) {
      return { cases: raw as ReverseTestResult["cases"], source: "gateway" };
    }
  } catch (err) {
    if (!is404(err)) throw err;
  }
  return { cases: MOCK_REVERSE_CASES, source: "mock" };
}
