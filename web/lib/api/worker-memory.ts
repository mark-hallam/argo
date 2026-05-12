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
 * and `worker-activity`: `NEXT_PUBLIC_GATEWAY_AUTH_BYPASS=1` short-
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
import { ApiError } from "@/lib/api/types";
import {
  findSampleWorkerMemory,
  mockAddFact,
  mockDeleteFact,
  mockEditFact,
} from "@/lib/sample-worker-memory";

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
  return (
    process.env.NODE_ENV !== "production" &&
    process.env.NEXT_PUBLIC_GATEWAY_AUTH_BYPASS === "1"
  );
}

function isMissingOrUnreachable(err: unknown): boolean {
  if (err instanceof ApiError) return err.status === 404;
  return err instanceof TypeError;
}

function mockResult(workerId: string): FetchWorkerMemoryResult {
  const memory = findSampleWorkerMemory(workerId);
  if (!memory) throw new WorkerMemoryNotFoundError(workerId);
  return { memory, source: "mock-offline" };
}

export async function fetchMemory(
  workerId: string,
): Promise<FetchWorkerMemoryResult> {
  if (isAuthBypassed()) {
    const memory = findSampleWorkerMemory(workerId);
    if (!memory) throw new WorkerMemoryNotFoundError(workerId);
    return { memory, source: "mock-bypass" };
  }

  try {
    const memory = await apiGet<WorkerMemory>(
      `/api/v1/workers/${encodeURIComponent(workerId)}/memory`,
    );
    return { memory, source: "gateway" };
  } catch (err) {
    if (!isStrictProduction() && isMissingOrUnreachable(err)) {
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
  if (isAuthBypassed()) {
    return mockAddFact(workerId, input.text);
  }
  try {
    return await apiPost<WorkerFact, AddFactInput>(
      `/api/v1/workers/${encodeURIComponent(workerId)}/memory/facts`,
      input,
    );
  } catch (err) {
    if (!isStrictProduction() && isMissingOrUnreachable(err)) {
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
  if (isAuthBypassed()) {
    return mockEditFact(workerId, factId, input.text);
  }
  try {
    return await apiPatch<WorkerFact, EditFactInput>(
      `/api/v1/workers/${encodeURIComponent(workerId)}/memory/facts/${encodeURIComponent(factId)}`,
      input,
    );
  } catch (err) {
    if (!isStrictProduction() && isMissingOrUnreachable(err)) {
      return mockEditFact(workerId, factId, input.text);
    }
    throw err;
  }
}

export async function deleteFact(
  workerId: string,
  factId: string,
): Promise<void> {
  if (isAuthBypassed()) {
    mockDeleteFact(workerId, factId);
    return;
  }
  try {
    await apiDelete<void>(
      `/api/v1/workers/${encodeURIComponent(workerId)}/memory/facts/${encodeURIComponent(factId)}`,
    );
  } catch (err) {
    if (!isStrictProduction() && isMissingOrUnreachable(err)) {
      mockDeleteFact(workerId, factId);
      return;
    }
    throw err;
  }
}
