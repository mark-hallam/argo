/**
 * Genesis conversational-intake turn client — the "Talk it through" half of
 * `/genesis` (UF Phase-A · A3, DEC-039).
 *
 * Calls `POST /api/v1/genesis/conversation` one turn at a time. The backend
 * (#2724) owns a deterministic slot state machine + two bounded fail-safe LLM
 * calls per turn + tenant-scoped persistence; this module is only the typed
 * boundary. It does NOT fork the one-shot `streamGenesis` / `hireFromGenesis`
 * calls in `genesis.ts` — on confirm, the gathered slots are handed back to
 * that SAME materialize → review → hire path (no parallel client-side
 * materializer; see the page's completion handler).
 *
 * Defensive by construction: the response is normalized so a missing /
 * malformed field degrades to a safe empty default rather than throwing,
 * keeping the chat moving even against an older or partial gateway.
 */

import { apiPost } from "@/lib/api/client";
import type { SuccessCriterion } from "@/lib/api/genesis";

/** Turn status the engine reports: still gathering, ready to confirm, or
 *  finalized after `confirm: true`. */
export type ConversationStatus = "asking" | "ready" | "done";

/** The running "what we've captured" block the panel renders live. Mirrors the
 *  backend's `captured` object; every field defaults empty until filled. */
export interface ConversationCaptured {
  /** The teammate's mission / charter (one paragraph). */
  charter: string;
  /** Voice + working-style persona. */
  persona: string;
  /** What the teammate is responsible for. */
  responsibilities: string[];
  /** WHEN it runs — free-text cadence/schedule. */
  cadence: string;
  /** Carve-outs the teammate must hand off, not act on. */
  exceptions: string[];
  /** HOW success is measured — typed, measurable criteria. */
  success_criteria: SuccessCriterion[];
}

/** One transcript line surfaced as a chat bubble. */
export interface ConversationTurn {
  speaker: "assistant" | "operator";
  text: string;
}

/** Normalized turn response the UI consumes. */
export interface ConversationResponse {
  conversation_id: string;
  status: ConversationStatus;
  /** The slot the engine is currently gathering (e.g. `"cadence"`). */
  slot?: string;
  /** The assistant's next prompt / acknowledgement. */
  assistant_message?: string;
  captured: ConversationCaptured;
  /** The assembled spec the backend echoes on `ready`/`done`. The SPA does NOT
   *  materialize from this — it hands the gathered slots to the one-shot flow —
   *  so it's surfaced opaquely for display/debug only. */
  spec?: unknown;
  transcript: ConversationTurn[];
}

/** A turn request. The opening turn sends no `message`; the client owns the
 *  stable `conversationId`. `confirm: true` finalizes a `ready` conversation. */
export interface ConversationTurnRequest {
  conversationId: string;
  message?: string;
  confirm?: boolean;
  /** Per-browser tenant id, forwarded as `X-Argo-Tenant-Id` so the persisted
   *  conversation + the eventual hire share one tenant segment (mirrors
   *  `streamGenesis`). */
  tenantId: string;
  signal?: AbortSignal;
}

/** The empty captured block a parent can seed state with. */
export const EMPTY_CAPTURED: ConversationCaptured = {
  charter: "",
  persona: "",
  responsibilities: [],
  cadence: "",
  exceptions: [],
  success_criteria: [],
};

function asString(value: unknown): string {
  return typeof value === "string" ? value : "";
}

function asStringArray(value: unknown): string[] {
  if (!Array.isArray(value)) return [];
  return value.filter((v): v is string => typeof v === "string");
}

function asSuccessCriteria(value: unknown): SuccessCriterion[] {
  if (!Array.isArray(value)) return [];
  const out: SuccessCriterion[] = [];
  for (const raw of value) {
    if (!raw || typeof raw !== "object") continue;
    const c = raw as Record<string, unknown>;
    out.push({
      name: asString(c.name),
      kind: typeof c.kind === "string" ? c.kind : "count",
      target: typeof c.target === "number" ? c.target : 0,
      comparator: typeof c.comparator === "string" ? c.comparator : "gte",
    });
  }
  return out;
}

function normalizeCaptured(value: unknown): ConversationCaptured {
  if (!value || typeof value !== "object") return { ...EMPTY_CAPTURED };
  const c = value as Record<string, unknown>;
  return {
    charter: asString(c.charter),
    persona: asString(c.persona),
    responsibilities: asStringArray(c.responsibilities),
    cadence: asString(c.cadence),
    exceptions: asStringArray(c.exceptions),
    success_criteria: asSuccessCriteria(c.success_criteria),
  };
}

function normalizeStatus(value: unknown): ConversationStatus {
  return value === "ready" || value === "done" ? value : "asking";
}

function normalizeTranscript(value: unknown): ConversationTurn[] {
  if (!Array.isArray(value)) return [];
  const out: ConversationTurn[] = [];
  for (const raw of value) {
    if (!raw || typeof raw !== "object") continue;
    const t = raw as Record<string, unknown>;
    const speaker = t.speaker === "operator" ? "operator" : "assistant";
    const text = asString(t.text);
    if (text.length === 0) continue;
    out.push({ speaker, text });
  }
  return out;
}

/** Normalize a raw turn payload into the typed [[ConversationResponse]]. */
export function normalizeConversationResponse(
  raw: unknown,
  fallbackConversationId: string,
): ConversationResponse {
  const v = raw && typeof raw === "object" ? (raw as Record<string, unknown>) : {};
  return {
    conversation_id: asString(v.conversation_id) || fallbackConversationId,
    status: normalizeStatus(v.status),
    slot: typeof v.slot === "string" ? v.slot : undefined,
    assistant_message:
      typeof v.assistant_message === "string" ? v.assistant_message : undefined,
    captured: normalizeCaptured(v.captured),
    spec: v.spec,
    transcript: normalizeTranscript(v.transcript),
  };
}

/**
 * POST one conversational-intake turn and return the normalized response.
 * Throws `ApiError` on non-2xx / network so the caller can surface a retry.
 */
export async function postConversationTurn(
  req: ConversationTurnRequest,
): Promise<ConversationResponse> {
  const body: Record<string, unknown> = { conversation_id: req.conversationId };
  if (req.message !== undefined && req.message.length > 0) body.message = req.message;
  if (req.confirm) body.confirm = true;
  const raw = await apiPost<unknown>("/api/v1/genesis/conversation", body, {
    headers: { "X-Argo-Tenant-Id": req.tenantId },
    signal: req.signal,
  });
  return normalizeConversationResponse(raw, req.conversationId);
}
