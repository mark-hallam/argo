/**
 * Worker-memory API client.
 *
 * Backs the Memory tab (E4-d) with four data feeds and three
 * mutations:
 *
 *  - `fetchMemory(workerId)` → namespace + facts + active
 *    conversations + recent decisions + voice profile.
 *  - `addFact(workerId, text)` → POST a new fact.
 *  - `editFact(workerId, factId, text)` → PUT updated fact text.
 *  - `deleteFact(workerId, factId)` → DELETE.
 *
 * Mirrors the 404→mock-fallback contract shared by `worker-overview`
 * and `worker-activity`: `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=1` short-
 * circuits straight to the bundled sample, dev tolerates 404 /
 * network errors and falls back, and strict production (`NODE_ENV
 * === 'production'` AND `NEXT_PUBLIC_ENV === 'production'`)
 * propagates every error.
 *
 * The mutation surface keeps the mock dataset in-memory only — page
 * lifetime — because the swarm's sibling brief
 * `mission-gateway-worker-memory-endpoints` lands the real CRUD
 * endpoints. v1 dev-mode persists nothing across page reloads.
 */

import { apiDelete, apiGet, apiPatch, apiPost } from "@/lib/api/client";
import { hasRealSession, isDemoLive } from "@/lib/skycot/demo-mode";
import { ApiError } from "@/lib/api/types";
import {
  findSampleWorkerMemory,
  mockAddFact,
  mockDeleteFact,
  mockEditFact,
} from "@/lib/sample-worker-memory";
import { isMissingOrUnreachable } from "./unreachable";

export type WorkerMemorySource = "gateway" | "mock-offline" | "mock-bypass";

export type VoiceTone = "warm" | "neutral" | "formal";

export interface WorkerFact {
  factId: string;
  /** Plain-English fact text. May carry PII — wrap at render. */
  text: string;
  /** ISO timestamp; surfaced as a tooltip on the row. */
  createdAt: string;
}

export interface ActiveConversation {
  conversationId: string;
  /** PII-redacted at render via `<RedactedText>`. */
  counterpartyHandle: string;
  /** Short topic snippet, also redacted. */
  topic: string;
  /** ISO timestamp of the last message. */
  lastMessageAt: string;
  /** Audit drill-in target for the most recent action on this thread. */
  href: string;
}

export interface RecentDecision {
  decisionId: string;
  /** Short label e.g. "Approved refund for order #18293". */
  label: string;
  /** Outcome chip — "approved" / "refused". */
  outcome: "approved" | "refused";
  /** ISO timestamp; shown as "2h ago" formatted at render. */
  decidedAt: string;
  /** Audit drill-in for the decision record. */
  href: string;
}

export interface VoiceProfile {
  tone: VoiceTone;
  /** Slider value 0–100; surfaced as text per the brief. */
  formality: number;
  /** Short list of phrases the worker leans on. */
  signaturePhrases: string[];
}

export interface WorkerMemory {
  /** Stable namespace id (e.g. `ns_priya_4f7a`). Surfaced as a chip
   *  when the viewer has `permissions.canViewSettings`. */
  namespaceId: string;
  /** Tenant scope label — shown next to the namespace chip. */
  tenantScope: string;
  facts: WorkerFact[];
  activeConversations: ActiveConversation[];
  recentDecisions: RecentDecision[];
  voiceProfile: VoiceProfile;
}

export interface FetchWorkerMemoryResult {
  memory: WorkerMemory;
  source: WorkerMemorySource;
}

export class WorkerMemoryNotFoundError extends Error {
  readonly workerId: string;
  constructor(workerId: string) {
    super(`Worker memory for '${workerId}' not found`);
    this.name = "WorkerMemoryNotFoundError";
    this.workerId = workerId;
  }
}

function isStrictProduction(): boolean {
  return (
    process.env.NODE_ENV === "production" &&
    process.env.NEXT_PUBLIC_ENV === "production"
  );
}

function isAuthBypassed(): boolean {
  // Per-user gate (sprint, 2026-06-07, mock-leak class): a REAL
  // signed-in user (the `skycot-session=1` marker cookie is present) is
  // NEVER served the auth-bypass mock fixtures, even in a demo-flagged
  // deploy. The bypass short-circuit is for ANONYMOUS demo visitors only.
  if (hasRealSession()) return false;
  return (
    process.env.NODE_ENV !== "production" &&
    process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS === "1"
  );
}

function mockResult(workerId: string): FetchWorkerMemoryResult {
  const memory = findSampleWorkerMemory(workerId);
  if (!memory) throw new WorkerMemoryNotFoundError(workerId);
  return { memory, source: "mock-offline" };
}

/** Raw voice-profile as the gateway emits it (snake_case; `formality`
 *  is a descriptor string, not the SPA's 0–100 slider number). */
interface RawGatewayVoiceProfile {
  tone?: string;
  formality?: number | string;
  lexicon?: string[];
  signaturePhrases?: string[];
}

/** A fact as the gateway may emit it — camelCase OR the legacy
 *  snake_case (`fact_id` / `created_at`). Tolerating both keeps the Memory
 *  tab's date column working regardless of which casing the gateway sends
 *  (the snake-only form is what produced the "Invalid Date" render). */
interface RawGatewayFact {
  factId?: string;
  fact_id?: string;
  text?: string;
  createdAt?: string;
  created_at?: string;
}

/** A conversation as the gateway may emit it — every field optional
 *  (a fresh worker omits `href` and may send snake_case ids), so the type
 *  can't lie the way casting raw items to `ActiveConversation` (required
 *  `href: string`) did. Normalized by `mapGatewayConversation`. */
interface RawGatewayConversation {
  conversationId?: string;
  conversation_id?: string;
  counterpartyHandle?: string;
  counterparty_handle?: string;
  topic?: string;
  lastMessageAt?: string;
  last_message_at?: string;
  href?: string;
}

/** A decision as the gateway may emit it — every field optional, both
 *  casings tolerated. Normalized by `mapGatewayDecision`. */
interface RawGatewayDecision {
  decisionId?: string;
  decision_id?: string;
  label?: string;
  outcome?: string;
  decidedAt?: string;
  decided_at?: string;
  href?: string;
}

/** Raw memory payload — gateway snake_case, with camelCase tolerated so
 *  this normalizer is a no-op if the gateway ever switches conventions. */
interface RawGatewayMemory {
  namespace_id?: string | null;
  namespaceId?: string | null;
  tenant_scope?: string | null;
  tenantScope?: string | null;
  facts?: RawGatewayFact[];
  active_conversations?: RawGatewayConversation[];
  activeConversations?: RawGatewayConversation[];
  recent_decisions?: RawGatewayDecision[];
  recentDecisions?: RawGatewayDecision[];
  voice_profile?: RawGatewayVoiceProfile;
  voiceProfile?: RawGatewayVoiceProfile;
}

/** Normalize one raw fact to the SPA's camelCase `WorkerFact`, accepting
 *  either casing for the id + timestamp. `createdAt` defaults to "" (not
 *  `undefined`) so a missing timestamp renders blank rather than the
 *  `new Date(undefined)` → "Invalid Date" the snake-only payload caused. */
function normalizeFact(f: RawGatewayFact): WorkerFact {
  return {
    factId: f.factId ?? f.fact_id ?? "",
    text: f.text ?? "",
    createdAt: f.createdAt ?? f.created_at ?? "",
  };
}

/**
 * Normalize one raw conversation to the SPA's `ActiveConversation`. Mirrors
 * the `worker-overview.ts::mapGatewayActivityRow` gold standard: a fresh
 * worker omits per-item `href`, so the prior code (which cast raw items
 * straight to a type declaring required `href: string`) was lying — any
 * consumer that did NOT guard `href` would crash. Here `href` defaults to the
 * audit drill-in built from the conversation id (dashes stripped to match the
 * `/audit/action/<hex>` route), or "" when there's no id — never `undefined`.
 */
function mapGatewayConversation(c: RawGatewayConversation): ActiveConversation {
  const conversationId = c.conversationId ?? c.conversation_id ?? "";
  const href =
    c.href ??
    (conversationId
      ? `/audit/action/${encodeURIComponent(conversationId.replace(/-/g, ""))}`
      : "");
  return {
    conversationId,
    counterpartyHandle: c.counterpartyHandle ?? c.counterparty_handle ?? "",
    topic: c.topic ?? "",
    lastMessageAt: c.lastMessageAt ?? c.last_message_at ?? "",
    href,
  };
}

/** `RecentDecision.outcome` is the closed set `"approved" | "refused"`;
 *  anything else from the gateway defaults to "refused" (the safe-to-show
 *  label) rather than leaking an unexpected string into the outcome chip. */
function normalizeDecisionOutcome(value: string | undefined): "approved" | "refused" {
  return value === "approved" ? "approved" : "refused";
}

/**
 * Normalize one raw decision to the SPA's `RecentDecision`. Same honesty fix
 * as `mapGatewayConversation`: defaults every required field (incl. `href`)
 * so the declared contract is true and no render guard is load-bearing.
 */
function mapGatewayDecision(d: RawGatewayDecision): RecentDecision {
  const decisionId = d.decisionId ?? d.decision_id ?? "";
  const href =
    d.href ??
    (decisionId
      ? `/audit/action/${encodeURIComponent(decisionId.replace(/-/g, ""))}`
      : "");
  return {
    decisionId,
    label: d.label ?? "",
    outcome: normalizeDecisionOutcome(d.outcome),
    decidedAt: d.decidedAt ?? d.decided_at ?? "",
    href,
  };
}

/** Map a raw conversation list to normalized items; non-arrays → `[]` (a
 *  sparse / absent gateway payload renders the empty state, never crashes). */
function normalizeConversationList(
  raw: RawGatewayConversation[] | undefined,
): ActiveConversation[] {
  return Array.isArray(raw) ? raw.map(mapGatewayConversation) : [];
}

/** Map a raw decision list to normalized items; non-arrays → `[]`. */
function normalizeDecisionList(
  raw: RawGatewayDecision[] | undefined,
): RecentDecision[] {
  return Array.isArray(raw) ? raw.map(mapGatewayDecision) : [];
}

const VOICE_TONES = new Set<string>(["warm", "neutral", "formal"]);

/** Gateway formality is a descriptor word; the SPA renders a 0–100 number. */
function mapFormality(f: number | string | undefined): number {
  if (typeof f === "number") return f;
  switch ((f ?? "").toLowerCase()) {
    case "casual":
    case "conversational":
      return 35;
    case "formal":
    case "professional":
      return 80;
    default:
      return 50;
  }
}

/** Normalize the gateway's snake_case memory payload into the SPA's
 *  camelCase `WorkerMemory`. Without this, `activeConversations` /
 *  `recentDecisions` / `voiceProfile` are `undefined`, and the memory
 *  tab's `.length` reads throw — the worker-detail error boundary then
 *  swallows the whole tab. Lists default to `[]` so a fresh worker (all
 *  empty) renders the empty states instead of crashing. */
function normalizeGatewayMemory(raw: RawGatewayMemory): WorkerMemory {
  const vp = raw.voiceProfile ?? raw.voice_profile ?? {};
  const tone =
    typeof vp.tone === "string" && VOICE_TONES.has(vp.tone)
      ? (vp.tone as VoiceTone)
      : "neutral";
  return {
    namespaceId: raw.namespaceId ?? raw.namespace_id ?? "",
    tenantScope: raw.tenantScope ?? raw.tenant_scope ?? "Current workspace",
    facts: Array.isArray(raw.facts) ? raw.facts.map(normalizeFact) : [],
    activeConversations: normalizeConversationList(
      raw.activeConversations ?? raw.active_conversations,
    ),
    recentDecisions: normalizeDecisionList(
      raw.recentDecisions ?? raw.recent_decisions,
    ),
    voiceProfile: {
      tone,
      formality: mapFormality(vp.formality),
      signaturePhrases: vp.signaturePhrases ?? vp.lexicon ?? [],
    },
  };
}

export async function fetchMemory(
  workerId: string,
): Promise<FetchWorkerMemoryResult> {
  if (isAuthBypassed() && !isDemoLive()) {
    const memory = findSampleWorkerMemory(workerId);
    if (!memory) throw new WorkerMemoryNotFoundError(workerId);
    return { memory, source: "mock-bypass" };
  }

  try {
    const raw = await apiGet<RawGatewayMemory>(
      `/api/v1/workers/${encodeURIComponent(workerId)}/memory`,
    );
    return { memory: normalizeGatewayMemory(raw), source: "gateway" };
  } catch (err) {
    if (!hasRealSession() && !isDemoLive() && !isStrictProduction() && isMissingOrUnreachable(err)) {
      return mockResult(workerId);
    }
    if (err instanceof ApiError && err.status === 404) {
      throw new WorkerMemoryNotFoundError(workerId);
    }
    throw err;
  }
}

export interface AddFactInput {
  text: string;
}

export async function addFact(
  workerId: string,
  input: AddFactInput,
): Promise<WorkerFact> {
  if (isAuthBypassed() && !isDemoLive()) {
    return mockAddFact(workerId, input.text);
  }
  try {
    return await apiPost<WorkerFact, AddFactInput>(
      `/api/v1/workers/${encodeURIComponent(workerId)}/memory/facts`,
      input,
    );
  } catch (err) {
    if (!hasRealSession() && !isDemoLive() && !isStrictProduction() && isMissingOrUnreachable(err)) {
      return mockAddFact(workerId, input.text);
    }
    throw err;
  }
}

export interface EditFactInput {
  text: string;
}

/**
 * PATCH the fact text. The brief mentions PUT; the workspace
 * `client.ts` exposes `apiPatch` (not PUT), and gateway briefs in
 * flight (`mission-gateway-worker-memory-endpoints`) standardise on
 * PATCH for partial updates. PATCH is the correct verb for a single-
 * field edit; the brief's "PUT" is a noted sensible deviation.
 */
export async function editFact(
  workerId: string,
  factId: string,
  input: EditFactInput,
): Promise<WorkerFact> {
  if (isAuthBypassed() && !isDemoLive()) {
    return mockEditFact(workerId, factId, input.text);
  }
  try {
    return await apiPatch<WorkerFact, EditFactInput>(
      `/api/v1/workers/${encodeURIComponent(workerId)}/memory/facts/${encodeURIComponent(factId)}`,
      input,
    );
  } catch (err) {
    if (!hasRealSession() && !isDemoLive() && !isStrictProduction() && isMissingOrUnreachable(err)) {
      return mockEditFact(workerId, factId, input.text);
    }
    throw err;
  }
}

export async function deleteFact(
  workerId: string,
  factId: string,
): Promise<void> {
  if (isAuthBypassed() && !isDemoLive()) {
    mockDeleteFact(workerId, factId);
    return;
  }
  try {
    await apiDelete<void>(
      `/api/v1/workers/${encodeURIComponent(workerId)}/memory/facts/${encodeURIComponent(factId)}`,
    );
  } catch (err) {
    if (!isDemoLive() && !isStrictProduction() && isMissingOrUnreachable(err)) {
      mockDeleteFact(workerId, factId);
      return;
    }
    throw err;
  }
}
