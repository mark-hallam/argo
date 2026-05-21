/**
 * Genesis-lite classification API client.
 *
 * Calls `POST /api/v1/genesis/classify` via the typed gateway client.
 * v1 falls back to the local `STUB_QUESTIONS` set on 404 / malformed
 * response so the hire flow keeps moving when the gateway-side
 * endpoint isn't wired yet (sibling brief
 * `mission-gateway-genesis-classify-endpoint`). 5xx and network
 * errors are re-thrown so the upstream TanStack Query retry loop can
 * apply backoff before falling back to the stub.
 *
 * Acceptance contract:
 * - Happy path → returns `{ questions, source: "gateway" }`.
 * - 404 / malformed → returns `{ questions: STUB_QUESTIONS, source: "stub" }`.
 * - 5xx / network → throws (caller retries; eventually falls back to stub).
 */

import { apiPost } from "@/lib/api/client";
import { ApiError } from "@/lib/api/types";
import { STUB_QUESTIONS, stubWorkerDraft } from "@/lib/hire/genesis-stub";
import {
  fromGatewayWorkerDraft,
  type GatewayDraftResponse,
  type GatewayWorkerDraft,
  type WorkerDraft,
} from "@/lib/hire/draft-types";
import type {
  AnswerType,
  ClassifyResponse,
  Question,
  QuestionAnswer,
} from "@/lib/hire/genesis-types";

export type GenesisSource = "gateway" | "stub";

export interface ClassifyResult {
  questions: Question[];
  source: GenesisSource;
  classification?: string;
}

export interface ClassifyRequest {
  problem: string;
}

const ANSWER_TYPES: ReadonlySet<AnswerType> = new Set<AnswerType>([
  "single-select",
  "multi-select",
  "free-text",
  "yes-no",
]);

function isQuestion(value: unknown): value is Question {
  if (!value || typeof value !== "object") return false;
  const q = value as Record<string, unknown>;
  if (typeof q.id !== "string" || q.id.length === 0) return false;
  if (typeof q.text !== "string" || q.text.length === 0) return false;
  if (typeof q.answer_type !== "string") return false;
  if (!ANSWER_TYPES.has(q.answer_type as AnswerType)) return false;
  if (q.answer_type === "single-select" || q.answer_type === "multi-select") {
    if (!Array.isArray(q.options) || q.options.length === 0) return false;
    if (!q.options.every((opt) => typeof opt === "string")) return false;
  }
  return true;
}

function isClassifyResponse(value: unknown): value is ClassifyResponse {
  if (!value || typeof value !== "object") return false;
  const v = value as Record<string, unknown>;
  if (!Array.isArray(v.questions)) return false;
  if (v.questions.length === 0 || v.questions.length > 3) return false;
  return v.questions.every(isQuestion);
}

function stubResult(): ClassifyResult {
  return { questions: [...STUB_QUESTIONS], source: "stub" };
}

/**
 * Classify a Step 1 problem statement into clarifying questions.
 *
 * Returns the gateway's response on success, or a stub fallback when
 * the gateway returns 404 or a malformed payload. Throws on 5xx /
 * network so the upstream retry-with-backoff loop can re-attempt
 * before flipping to the stub.
 */
export async function classifyRole(req: ClassifyRequest): Promise<ClassifyResult> {
  let raw: unknown;
  try {
    raw = await apiPost<unknown>("/api/v1/genesis/classify", { problem: req.problem });
  } catch (err) {
    if (err instanceof ApiError && err.status === 404) {
      return stubResult();
    }
    throw err;
  }

  if (!isClassifyResponse(raw)) {
    return stubResult();
  }

  return {
    questions: raw.questions,
    source: "gateway",
    classification: raw.classification,
  };
}

export interface DraftWorkerRequest {
  problem: string;
  answers: Record<string, QuestionAnswer>;
  /**
   * Free-text re-draft hint surfaced by Step 4's Adjust drawer
   * (E2-e). Server-side Genesis treats this as additional context
   * ("make her more formal", "shorter replies", etc.). v1 stub
   * ignores it — the user still gets the generic draft until the
   * gateway endpoint lands. Optional so Step 3's first-pass draft
   * call stays unchanged.
   */
  adjustment?: string;
}

export interface DraftWorkerResult {
  draft: WorkerDraft;
  source: GenesisSource;
}

function isStringArray(value: unknown): value is string[] {
  return Array.isArray(value) && value.every((v) => typeof v === "string");
}

/**
 * Type-guard for the canonical wire shape returned by
 * `POST /api/v1/genesis/draft-worker` — the flat string-only struct
 * defined at `crates/argo-cli/src/handlers/gateway/routes_genesis.rs`.
 * The richer nested TS [[WorkerDraft]] is reconstructed from this by
 * [[fromGatewayWorkerDraft]] at the boundary. Prior to the
 * `web-hire-workerdraft-schema-mirror-only` brief, this validator
 * mistakenly expected the rich shape and silently fell back to the
 * STUB draft on every gateway hit — root cause of the "5 hires →
 * 5 Mariah clones" bug.
 */
function isGatewayWorkerDraft(value: unknown): value is GatewayWorkerDraft {
  if (!value || typeof value !== "object") return false;
  const v = value as Record<string, unknown>;
  if (typeof v.suggestedName !== "string") return false;
  if (typeof v.persona !== "string") return false;
  if (typeof v.charter !== "string") return false;
  if (!isStringArray(v.duties)) return false;
  if (!isStringArray(v.exampleReplies)) return false;
  if (typeof v.approvalPolicy !== "string") return false;
  return typeof v.suggestedVertical === "string";
}

function isGatewayDraftResponse(value: unknown): value is GatewayDraftResponse {
  if (!value || typeof value !== "object") return false;
  const v = value as Record<string, unknown>;
  if (!isGatewayWorkerDraft(v.worker_draft)) return false;
  return v.cost_usd === null || typeof v.cost_usd === "number";
}

/**
 * Generate a worker draft from a Step 1 problem statement + Step 2
 * answers. Returns the gateway's draft on success, or the stub
 * fallback when the gateway returns 404 / malformed payload. Throws
 * on 5xx / network so the caller can apply a retry strategy.
 */
export async function draftWorker(req: DraftWorkerRequest): Promise<DraftWorkerResult> {
  let raw: unknown;
  const payload: Record<string, unknown> = {
    problem: req.problem,
    answers: req.answers,
  };
  if (req.adjustment !== undefined && req.adjustment.length > 0) {
    payload.adjustment = req.adjustment;
  }
  try {
    raw = await apiPost<unknown>("/api/v1/genesis/draft-worker", payload);
  } catch (err) {
    if (err instanceof ApiError && err.status === 404) {
      return { draft: stubWorkerDraft(req.problem), source: "stub" };
    }
    throw err;
  }

  if (!isGatewayDraftResponse(raw)) {
    return { draft: stubWorkerDraft(req.problem), source: "stub" };
  }

  return {
    draft: fromGatewayWorkerDraft(raw.worker_draft),
    source: "gateway",
  };
}
