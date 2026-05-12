/**
 * Workers API client.
 *
 * Routes through `lib/api/client.ts`. The gateway endpoint
 * `GET /api/v1/workers/<id>` is being authored under
 * `mission-gateway-worker-detail-endpoint`; until that lands, every
 * call here falls back to the bundled mock dataset in dev so the
 * profile page renders without a running gateway.
 *
 * Production posture (toggled via NODE_ENV): on a 404 we re-throw an
 * `ApiError` so the page can render a "worker not found" state with
 * confidence. The mock fallback is dev-only.
 *
 * E3-b's pause/resume mutations and E2-e's hireWorker live alongside
 * this file in their own briefs; this file currently exposes only
 * `fetchWorker` plus a typed export of the not-found error.
 */

import { apiGet } from "@/lib/api/client";
import { ApiError } from "@/lib/api/types";
import { findSampleWorker } from "@/lib/sample-workers-detail";
import type { Worker } from "@/lib/types/worker";

/** Thrown when the worker id doesn't resolve. */
export class WorkerNotFoundError extends Error {
  readonly workerId: string;
  constructor(workerId: string) {
    super(`Worker '${workerId}' not found`);
    this.name = "WorkerNotFoundError";
    this.workerId = workerId;
  }
}

function shouldFallBackToMock(err: unknown): boolean {
  // Dev-only: gateway absent or returned 404. Anything else is a real
  // error — surface it.
  if (process.env.NODE_ENV === "production") return false;
  if (err instanceof ApiError) return err.status === 404;
  return err instanceof TypeError; // network/fetch failure
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
