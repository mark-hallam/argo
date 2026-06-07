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
import { appendDemoWorker, findDemoWorker } from "@/lib/api/demo-roster-store";
import type { WorkerOverview } from "@/lib/api/worker-overview";
import {
  findSampleWorker,
  SAMPLE_WORKERS_DETAIL,
} from "@/lib/sample-workers-detail";
import type { WorkerDraft } from "@/lib/hire/draft-types";
import type { QuestionAnswer } from "@/lib/hire/genesis-types";
import { getOrCreateIdempotencyKey, uuidV7 } from "@/lib/hire/idempotency-key";
import { hasRealSession, isDemoLive } from "@/lib/skycot/demo-mode";
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

function isMissingOrUnreachable(err: unknown): boolean {
  // 404 (not_found) and 400 (bad_request, e.g. invalid_worker_id from the
  // gateway's UUID parse) both surface as "this worker id doesn't resolve"
  // to the user. The demo wizard mints non-UUID synthetic ids like
  // `demo-worker-marketing-intern-mpn1b10z`; the gateway rejects those
  // with 400 + "invalid worker_id" reason — without this branch React
  // Query retries indefinitely (the WorkerLayoutShell shows "Loading…"
  // for ~15s before settling on a friendly error). See
  // fix-fetchworker-400-bad-request-no-branch-infinite-retry.
  if (err instanceof ApiError) return err.status === 404 || err.status === 400;
  return err instanceof TypeError;
}

function shouldFallBackToMock(err: unknown): boolean {
  // Dev-only: gateway absent or returned 404. Anything else is a real
  // error — surface it.
  if (process.env.NODE_ENV === "production") return false;
  // A REAL signed-in user never falls back to the sample roster — they get
  // an honest error on a genuine gateway failure (mock-leak class). The
  // `isDemoLive()` guard alone can't tell a real user apart from an
  // anonymous dev visitor, since both read it as false.
  if (hasRealSession()) return false;
  if (isDemoLive()) return false;
  return isMissingOrUnreachable(err);
}

export async function fetchWorker(workerId: string): Promise<Worker> {
  // Auth-bypass mocks the gateway; DEMO_LIVE=1 opts back into real
  // gateway calls so the recording-day SPA can prove a real round-
  // trip. Outside DEMO_LIVE we resolve from the bundled / session-
  // local rosters directly and throw `WorkerNotFoundError` for any
  // id we don't seed — never fabricate a worker for an unknown URL.
  if (isAuthBypassed() && !isDemoLive()) {
    const demo = findDemoWorker(workerId);
    if (demo) return demo;
    const sample = findSampleWorker(workerId);
    if (sample) return sample;
    throw new WorkerNotFoundError(workerId);
  }
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
      // Demo-recording session-local hires take precedence over the
      // bundled fixture so a brand-new hire's overview page resolves.
      const demo = findDemoWorker(workerId);
      if (demo) return demo;
      const sample = findSampleWorker(workerId);
      if (sample) {
        await new Promise((r) => setTimeout(r, 80));
        return sample;
      }
      throw new WorkerNotFoundError(workerId);
    }
    if (err instanceof ApiError && (err.status === 404 || err.status === 400)) {
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
  if (isAuthBypassed() && !isDemoLive()) {
    return { ok: true, workerId, status: nextStatus };
  }
  try {
    return await apiPost<WorkerActionResult>(
      `/api/v1/workers/${workerId}/${action}`,
    );
  } catch (err) {
    if (!isDemoLive() && !isStrictProduction() && isMissingOrUnreachable(err)) {
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

function buildDemoOverview(draft: WorkerDraft): WorkerOverview {
  const duties = draft.duties.map((d, idx) => ({
    id: `d-new-${idx + 1}`,
    label: d.label,
    schedule:
      d.schedule.kind === "cron"
        ? `cron: ${d.schedule.spec}`
        : `event: ${d.schedule.spec}`,
    enabled: d.defaultEnabled,
  }));
  return {
    charter: draft.charter,
    duties,
    // The new hire has no track record yet — surface honest zeros
    // rather than borrowing another worker's numbers. The demo
    // narration ("we just hired her") frames empty KPIs correctly.
    kpis: [
      {
        id: "k-actions",
        label: "actions taken today",
        value: "0",
        series: [0, 0, 0, 0, 0, 0, 0],
        deltaPct: 0,
        trend: "flat",
        tone: "muted",
      },
      {
        id: "k-approvals",
        label: "approvals awaiting you",
        value: "0",
        series: [0, 0, 0, 0, 0, 0, 0],
        deltaPct: 0,
        trend: "flat",
        tone: "muted",
      },
      {
        id: "k-shadow",
        label: "shadow-mode hours",
        value: "0h",
        series: [0, 0, 0, 0, 0, 0, 0],
        deltaPct: 0,
        trend: "flat",
        tone: "muted",
      },
      {
        id: "k-onboarding",
        label: "onboarding progress",
        value: "Just hired",
        series: [0, 0, 0, 0, 0, 0, 0],
        deltaPct: 0,
        trend: "flat",
        tone: "muted",
      },
    ],
    recentActivity: [],
  };
}

function buildDemoWorker(workerId: string, draft: WorkerDraft): Worker {
  return {
    id: workerId,
    name: draft.suggestedName,
    role: {
      tagline: draft.persona.tagline,
      vertical: draft.suggestedVertical,
    },
    status: "idle",
    lastActiveAt: new Date().toISOString(),
  };
}

function isStubFallbackEligible(err: unknown): boolean {
  if (isStrictProduction()) return false;
  if (isDemoLive()) return false;
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
 * `POST /api/v1/workers/<id>/run` — manual run-now trigger.
 *
 * `action_id` and `audit_id` both carry the lower-hex audit row id so
 * the SPA can deep-link to `/audit/action/<action_id>`. v1 emits the
 * audit row only; runner enqueue lands in a follow-up brief.
 */
export interface WorkerRunResult {
  action_id: string;
  audit_id: string;
  trigger_id: string;
  queued_at: string;
}

export async function triggerWorkerRun(
  workerId: string,
  note?: string,
): Promise<WorkerRunResult> {
  const body: Record<string, unknown> = { trigger_source: "manual_run_now" };
  if (note && note.trim()) body.note = note.trim();
  return postProxy<WorkerRunResult>(
    `/api/workers/${encodeURIComponent(workerId)}/run`,
    body,
  );
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
  if (isAuthBypassed() && !isDemoLive()) {
    // Plain auth-bypass mode: persist the new hire into a session-local
    // roster so /team gains a card and /workers/<id>/overview renders
    // the just-hired persona instead of 404ing. The id is a real-
    // shaped UUIDv7 (not `stub-` prefixed) so the URL + browser tab
    // title don't out the demo. See `fix-hire-demo-mode-stub-uuid-
    // worker-roster-invisibility`.
    const workerId = uuidV7();
    appendDemoWorker({
      worker: buildDemoWorker(workerId, req.workerDraft),
      overview: buildDemoOverview(req.workerDraft),
    });
    return { workerId, success: true };
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
    // The live endpoint responded but in a shape we don't recognize.
    // Only the dev/bypass path renders the bundled sample roster; a
    // production (or demo-live) session gets an honest empty list so a
    // real signed-in user never sees fixtures (NO THEATER, ws3-3).
    if (
      process.env.NODE_ENV !== "production" &&
      !hasRealSession() &&
      !isDemoLive()
    ) {
      return sampleWorkersForVertical(verticalId);
    }
    return [];
  } catch (err) {
    if (shouldFallBackToMock(err)) return sampleWorkersForVertical(verticalId);
    throw err;
  }
}

function sampleWorkersForVertical(verticalId: string): Worker[] {
  return SAMPLE_WORKERS_DETAIL.filter((w) => w.role.vertical === verticalId);
}
