/**
 * Workers API client — single-worker reads + worker-level mutations.
 *
 * Routes through `lib/api/client.ts`.
 *
 * **Reads (E4-a)**: `fetchWorker` calls `GET /api/v1/workers/<id>` and
 * falls back to bundled sample data on 404/network-unreachable in
 * dev. In strict production a 404 raises `WorkerNotFoundError` so the
 * page can render a real not-found state.
 *
 * **Mutations (E3-b)**: `pauseWorker` / `resumeWorker` call
 * `POST /api/v1/workers/<id>/pause|resume` (owned by sibling brief
 * `mission-gateway-worker-pause-endpoint`); until that lands, 404s
 * and gateway-unreachable errors fall back to a synthetic
 * `{ ok: true }` envelope in dev so the demo keeps working.
 *
 * **Hire commit (E2-e)**: `hireWorker` POSTs `/api/v1/workers` with
 * `{ workerDraft, problem, answers }` and resolves to a `{ workerId,
 * success: true }` envelope. The dev fallback returns
 * `{ workerId: "stub-<uuid>", success: true }` on 404 / gateway-
 * unreachable; in strict production every error propagates so the
 * Step 4 UI can surface a toast + stay-on-step. The architectural
 * target (server-side hire endpoint, mandatory in prod) closes via
 * `mission-enterprise-hire-commit-mandatory`.
 *
 * Acceptance contract:
 * - `pauseWorker` / `resumeWorker` resolve to a `{ ok, workerId,
 *   status }` envelope.
 * - `hireWorker` resolves to `{ workerId, success: true }`.
 * - 404 from the gateway → mock success in dev, propagates in prod.
 * - NetworkError (gateway unreachable) → mock success in dev,
 *   propagates in prod.
 * - `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=1` → mock success immediately,
 *   no network call.
 * - Strict production (`NODE_ENV === 'production'` AND
 *   `NEXT_PUBLIC_ENV === 'production'`): every error propagates so it
 *   surfaces in the UI rather than silently faking success.
 */

import { apiGet, apiPost } from "@/lib/api/client";
import { ApiError } from "@/lib/api/types";
import {
  findSampleWorker,
  SAMPLE_WORKERS_DETAIL,
} from "@/lib/sample-workers-detail";
import type { WorkerDraft } from "@/lib/hire/draft-types";
import type { QuestionAnswer } from "@/lib/hire/genesis-types";
import { getOrCreateIdempotencyKey } from "@/lib/hire/idempotency-key";
import type { Worker, WorkerStatus } from "@/lib/types/worker";

/** Thrown when the worker id doesn't resolve. */
export class WorkerNotFoundError extends Error {
  readonly workerId: string;
  constructor(workerId: string) {
    super(`Worker '${workerId}' not found`);
    this.name = "WorkerNotFoundError";
    this.workerId = workerId;
  }
}

/**
 * Thrown when a worker fetch races the SPA's session-token hydration
 * and the gateway returns 401. The session peek (`/api/auth/session`)
 * is asynchronous; on a fresh navigation the first React-Query fetch
 * can fire before the cookie-derived token is in memory, so the
 * Authorization header is absent and the gateway rejects the request.
 *
 * Surfacing this as its own error lets the React-Query layer retry
 * the request after a short backoff (by which time the session peek
 * has resolved) and lets the layout keep showing the loading skeleton
 * rather than the not-found shell. See brief
 * `fix-worker-detail-session-token-race-not-found`.
 */
export class AuthNotReadyError extends Error {
  readonly workerId: string;
  constructor(workerId: string) {
    super(`Auth not ready for worker '${workerId}'`);
    this.name = "AuthNotReadyError";
    this.workerId = workerId;
  }
}

export interface WorkerActionResult {
  ok: true;
  workerId: string;
  status: WorkerStatus;
}

interface WorkerEnvelope {
  worker: Worker;
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
    process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS === "1"
  );
}

function isMissingOrUnreachable(err: unknown): boolean {
  if (err instanceof ApiError) return err.status === 404;
  return err instanceof TypeError;
}

function shouldFallBackToMock(err: unknown): boolean {
  // Dev-only: gateway absent or returned 404. Anything else is a real
  // error — surface it.
  if (process.env.NODE_ENV === "production") return false;
  return isMissingOrUnreachable(err);
}

export async function fetchWorker(workerId: string): Promise<Worker> {
  try {
    const response = await apiGet<Worker | WorkerEnvelope>(
      `/api/v1/workers/${encodeURIComponent(workerId)}`,
    );
    // Gateway handlers may return either a bare Worker or a { worker } envelope.
    if (response && typeof response === "object" && "worker" in response) {
      return response.worker;
    }
    return response;
  } catch (err) {
    // 401 on a worker fetch is almost always the session-token race
    // (`/api/auth/session` hasn't resolved yet on this navigation).
    // Translate to AuthNotReadyError so `useWorker` retries instead
    // of the layout falling through to the not-found shell. Real
    // expired-session 401s redirect to /auth/signin inside
    // `parseError` before they reach here.
    if (err instanceof ApiError && err.status === 401) {
      throw new AuthNotReadyError(workerId);
    }
    if (shouldFallBackToMock(err)) {
      const sample = findSampleWorker(workerId);
      if (sample) {
        await new Promise((r) => setTimeout(r, 80));
        return sample;
      }
      throw new WorkerNotFoundError(workerId);
    }
    if (err instanceof ApiError && err.status === 404) {
      throw new WorkerNotFoundError(workerId);
    }
    throw err;
  }
}

async function postWorkerAction(
  workerId: string,
  action: "pause" | "resume",
  nextStatus: WorkerStatus,
): Promise<WorkerActionResult> {
  if (isAuthBypassed()) {
    return { ok: true, workerId, status: nextStatus };
  }
  try {
    return await apiPost<WorkerActionResult>(
      `/api/v1/workers/${workerId}/${action}`,
    );
  } catch (err) {
    if (!isStrictProduction() && isMissingOrUnreachable(err)) {
      return { ok: true, workerId, status: nextStatus };
    }
    throw err;
  }
}

export function pauseWorker(workerId: string): Promise<WorkerActionResult> {
  return postWorkerAction(workerId, "pause", "paused");
}

export function resumeWorker(workerId: string): Promise<WorkerActionResult> {
  return postWorkerAction(workerId, "resume", "idle");
}

export interface HireWorkerRequest {
  workerDraft: WorkerDraft;
  problem: string;
  answers: Record<string, QuestionAnswer>;
  /**
   * `true` when the worker draft came from the local stub fallback
   * (Step 3 Genesis call failed: 404 / malformed / 5xx / network).
   * Forwarded to the gateway as `"genesis_fallback": true` so the
   * hire handler can emit `AuditKind::GenesisFallbackUsed`, giving
   * operators visibility in `/audit` when their workspace's Genesis
   * is failing. Caller wiring (Step 4 → `hireWorker`) lands in a
   * follow-up brief — this field stays optional so the gateway
   * pass-through can ship without forcing all call sites to update.
   */
  genesisFallback?: boolean;
  /**
   * Optional caller-provided idempotency key (UUID v7 recommended). If
   * omitted, [[hireWorker]] sources the per-flow key from the
   * Step-1-generated localStorage entry via
   * [[getOrCreateIdempotencyKey]]. The gateway de-dupes on
   * `(tenant, key, created_at > now - 1h)` so a double-click or
   * page-reload during Step 4 cannot create a second worker.
   */
  idempotencyKey?: string;
}

export interface HireWorkerResult {
  workerId: string;
  success: true;
  /**
   * Set to `true` on a duplicate idempotency_key submission. The
   * `workerId` is the existing row's id and the parent UI can route
   * to it unchanged (the user sees the same Done state either way).
   */
  duplicate?: boolean;
}

function stubWorkerId(): string {
  // Browser-native UUID is sufficient for dev stubs; no extra dep.
  const rand =
    typeof crypto !== "undefined" && typeof crypto.randomUUID === "function"
      ? crypto.randomUUID()
      : Math.random().toString(36).slice(2, 10);
  return `stub-${rand}`;
}

function isStubFallbackEligible(err: unknown): boolean {
  if (isStrictProduction()) return false;
  return isMissingOrUnreachable(err);
}

export interface FreezeRecord {
  worker_id: string;
  frozen_at: string;
  frozen_by: string;
  reason: string;
}

export interface UnfreezeResult {
  worker_id: string;
  unfrozen: boolean;
  unfrozen_by: string;
}

/**
 * Freeze a worker via the Next.js proxy route. The proxy forwards
 * the session bearer token to `POST /api/v1/workers/<id>/freeze` on
 * the gateway; gateway enforces `worker.freeze` capability.
 */
export async function freezeWorker(
  workerId: string,
  reason: string,
): Promise<FreezeRecord> {
  return postProxy<FreezeRecord>(`/api/workers/${encodeURIComponent(workerId)}/freeze`, {
    reason,
  });
}

/**
 * Unfreeze a worker via the Next.js proxy route. Single-click — the
 * safe direction; the gateway accepts an empty body.
 */
export async function unfreezeWorker(workerId: string): Promise<UnfreezeResult> {
  return postProxy<UnfreezeResult>(
    `/api/workers/${encodeURIComponent(workerId)}/unfreeze`,
    undefined,
  );
}

async function postProxy<T>(path: string, body: unknown): Promise<T> {
  const init: RequestInit = {
    method: "POST",
    headers: { "Content-Type": "application/json", Accept: "application/json" },
    cache: "no-store",
  };
  if (body !== undefined) init.body = JSON.stringify(body);
  const res = await fetch(path, init);
  const text = await res.text();
  let parsed: unknown = null;
  if (text) {
    try {
      parsed = JSON.parse(text);
    } catch {
      /* leave parsed null */
    }
  }
  if (!res.ok) {
    const envelope =
      parsed && typeof parsed === "object" && "error" in parsed
        ? (parsed as { error?: { code?: string; message?: string } }).error
        : undefined;
    throw new ApiError(
      res.status,
      envelope?.message ?? res.statusText ?? `HTTP ${res.status}`,
      envelope?.code,
      parsed ?? text,
    );
  }
  return parsed as T;
}

/**
 * The gateway hire handler parses `worker_draft` (snake_case) as the
 * envelope key, while the SPA tracks `workerDraft` (camelCase) in
 * memory. The conversion lives at this single boundary so the rest of
 * the SPA can keep using the camelCase name. The inner field names of
 * the draft are already camelCase on both sides — the Rust `HireDraft`
 * struct in `crates/argo-identity/src/store_hire.rs` carries
 * `#[serde(rename_all = "camelCase")]`, so suggestedName /
 * suggestedVertical / etc. ride through unchanged.
 *
 * `workerId` on the response stays camelCase — the gateway already
 * emits camelCase there.
 */
function buildHireBody(req: HireWorkerRequest): Record<string, unknown> {
  const { workerDraft, problem, answers, genesisFallback, idempotencyKey } = req;
  const key = idempotencyKey ?? getOrCreateIdempotencyKey() ?? undefined;
  const body: Record<string, unknown> = {
    worker_draft: workerDraft,
    problem,
    answers,
  };
  if (genesisFallback) {
    body.genesis_fallback = true;
  }
  if (key) body.idempotency_key = key;
  return body;
}

export async function hireWorker(req: HireWorkerRequest): Promise<HireWorkerResult> {
  if (isAuthBypassed()) {
    return { workerId: stubWorkerId(), success: true };
  }
  try {
    const raw = await apiPost<unknown>("/api/v1/workers", buildHireBody(req));
    if (
      raw &&
      typeof raw === "object" &&
      typeof (raw as { workerId?: unknown }).workerId === "string"
    ) {
      return {
        workerId: (raw as { workerId: string }).workerId,
        success: true,
      };
    }
    if (isStrictProduction()) {
      throw new ApiError(
        502,
        "Hire endpoint returned an unexpected payload",
        "INVALID_HIRE_RESPONSE",
      );
    }
    return { workerId: stubWorkerId(), success: true };
  } catch (err) {
    if (isStubFallbackEligible(err)) {
      return { workerId: stubWorkerId(), success: true };
    }
    throw err;
  }
}

/**
 * List workers in one vertical for the workflow → schedule drawer.
 *
 * Calls `GET /api/v1/workers?vertical=<id>` (placeholder endpoint
 * authored alongside the SPA flow — until the gateway lands the
 * collection route it 404s, and we fall through to a sample-data
 * filter so the drawer is testable end-to-end with the gateway dark,
 * mirroring [[fetchWorker]]'s 404-fallback contract).
 */
export async function listWorkersInVertical(verticalId: string): Promise<Worker[]> {
  const path = `/api/v1/workers?vertical=${encodeURIComponent(verticalId)}`;
  try {
    const raw = await apiGet<unknown>(path);
    if (Array.isArray(raw)) return raw as Worker[];
    if (raw && typeof raw === "object") {
      const env = raw as { workers?: unknown };
      if (Array.isArray(env.workers)) return env.workers as Worker[];
    }
    return sampleWorkersForVertical(verticalId);
  } catch (err) {
    if (shouldFallBackToMock(err)) return sampleWorkersForVertical(verticalId);
    throw err;
  }
}

function sampleWorkersForVertical(verticalId: string): Worker[] {
  return SAMPLE_WORKERS_DETAIL.filter((w) => w.role.vertical === verticalId);
}
