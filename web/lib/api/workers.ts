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
 * Acceptance contract:
 * - `pauseWorker` / `resumeWorker` resolve to a `{ ok, workerId,
 *   status }` envelope.
 * - 404 from the gateway → mock success in dev, propagates in prod.
 * - NetworkError (gateway unreachable) → mock success in dev,
 *   propagates in prod.
 * - `NEXT_PUBLIC_GATEWAY_AUTH_BYPASS=1` → mock success immediately,
 *   no network call.
 * - Strict production (`NODE_ENV === 'production'` AND
 *   `NEXT_PUBLIC_ENV === 'production'`): every error propagates so it
 *   surfaces in the UI rather than silently faking success.
 *
 * E2-e's `hireWorker` lives alongside this file in its own brief.
 */

import { apiGet, apiPost } from "@/lib/api/client";
import { ApiError } from "@/lib/api/types";
import { findSampleWorker } from "@/lib/sample-workers-detail";
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

export interface WorkerActionResult {
  ok: true;
  workerId: string;
  status: WorkerStatus;
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

function shouldFallBackToMock(err: unknown): boolean {
  // Dev-only: gateway absent or returned 404. Anything else is a real
  // error — surface it.
  if (process.env.NODE_ENV === "production") return false;
  return isMissingOrUnreachable(err);
}

export async function fetchWorker(workerId: string): Promise<Worker> {
  try {
    return await apiGet<Worker>(`/api/v1/workers/${encodeURIComponent(workerId)}`);
  } catch (err) {
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
