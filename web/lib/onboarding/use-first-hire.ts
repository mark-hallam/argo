"use client";

import { useMutation } from "@tanstack/react-query";
import { apiGet, apiPost, ApiError } from "@/lib/api/client";
import { findWorkerByName } from "@/lib/api/team";
import {
  SAMPLE_JAMAL_ID as SAMPLE_JAMAL,
  SAMPLE_MARIAH_ID as SAMPLE_MARIAH,
  SAMPLE_NORI_ID as SAMPLE_NORI,
  SAMPLE_PETA_ID as SAMPLE_PETA,
  SAMPLE_PRIYA_ID as SAMPLE_PRIYA,
} from "@/lib/sample-personas";
import { isDemoMode } from "@/lib/skycot/demo-mode";

const VERIFY_ATTEMPTS = 3;
const VERIFY_BACKOFF_MS = 500;

export const FIRST_HIRE_VERIFY_FAILED_MESSAGE =
  "Hiring succeeded — your teammate is provisioning. Refresh in a few seconds; if they don't appear, the Help link in the top right has next steps.";

export interface FirstHireRequest {
  description: string;
  verticalId: string;
  tenantId?: string | null;
}

export interface FirstHireResult {
  workerId: string;
  workerName: string;
  /**
   * `true` when the gateway accepted the request but Genesis will
   * finish hiring asynchronously. The wizard still redirects to the
   * profile page; the profile renders a "hiring…" placeholder until
   * the live worker_id is ready.
   */
  pending: boolean;
}

interface FirstHirePayload {
  description: string;
  vertical_id: string;
}

interface FirstHireResponse {
  worker_id: string;
  worker_name?: string;
  pending?: boolean;
}

interface FirstHireVerifyFailedAuditPayload {
  action: "wizard.first_hire_verify_failed";
  metadata: {
    worker_id: string;
    tenant_id: string | null;
    vertical_id: string;
  };
  tenant: string;
}

function isFirstHireResponse(value: unknown): value is FirstHireResponse {
  if (!value || typeof value !== "object") return false;
  const v = value as Record<string, unknown>;
  return typeof v.worker_id === "string" && v.worker_id.length > 0;
}

function waitForRetry(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

async function emitFirstHireVerifyFailed(
  workerId: string,
  tenantId: string | null | undefined,
  verticalId: string,
): Promise<void> {
  const payload: FirstHireVerifyFailedAuditPayload = {
    action: "wizard.first_hire_verify_failed",
    metadata: {
      worker_id: workerId,
      tenant_id: tenantId ?? null,
      vertical_id: verticalId,
    },
    tenant: tenantId ?? "unknown",
  };
  try {
    await apiPost<void, FirstHireVerifyFailedAuditPayload>("/api/v1/audit", payload);
  } catch (err) {
    if (typeof console !== "undefined") {
      console.warn("first-hire verification audit emission failed", err);
    }
  }
}

async function verifyWorkerReadable(workerId: string): Promise<void> {
  for (let attempt = 1; attempt <= VERIFY_ATTEMPTS; attempt += 1) {
    try {
      await apiGet<unknown>(
        `/api/v1/workers/${encodeURIComponent(workerId)}/overview`,
      );
      return;
    } catch (err) {
      const retryableNotFound = err instanceof ApiError && err.status === 404;
      if (!retryableNotFound || attempt === VERIFY_ATTEMPTS) {
        throw err;
      }
      await waitForRetry(VERIFY_BACKOFF_MS);
    }
  }
}

/**
 * Vertical → sample-worker id mapping. Picks the closest persona by
 * role so the post-hire redirect lands on a real-rendered C2 card
 * matching the operator's chosen template. Unknown verticals fall
 * through to Mariah (the marketplace-lister), which the Scene-3
 * demo-runbook treats as the canonical "first hire" placeholder.
 *
 * IDs are imported from `sample-personas.ts` so the SPA and the
 * `argo demo seed` gateway-side roster share one UUID universe.
 */
const VERTICAL_TO_SAMPLE_WORKER: Record<string, { id: string; name: string }> = {
  "marketplace-seller": { id: SAMPLE_MARIAH, name: "Mariah" },
  recruiter: { id: SAMPLE_PRIYA, name: "Priya" },
  "customer-support": { id: SAMPLE_JAMAL, name: "Jamal" },
  "marketing-intern": { id: SAMPLE_NORI, name: "Nori" },
  "project-coordination": { id: SAMPLE_PETA, name: "Peta" },
};

const DEMO_FALLBACK_WORKER = { id: SAMPLE_MARIAH, name: "Mariah" };

function buildDemoFirstHireResult(verticalId: string): FirstHireResult {
  // Per `feedback_no_theater_in_demo`: the wizard MUST redirect to a
  // worker the destination page can actually render. The prior
  // `demo-worker-${vertical}-${ts}` factory minted a synthetic id that
  // matched no gateway worker and no sample fallback, so Scene 3's
  // headline beat ended on `<WorkerNotFound>`. Route through the
  // canonical sample roster instead — the destination /workers/<id>
  // page renders a real C2 personality card via `findSampleWorker`.
  const chosen = VERTICAL_TO_SAMPLE_WORKER[verticalId] ?? DEMO_FALLBACK_WORKER;
  return {
    workerId: chosen.id,
    workerName: chosen.name,
    pending: false,
  };
}

export async function firstHire(req: FirstHireRequest): Promise<FirstHireResult> {
  // Demo-mode short-circuit: the first-hire endpoint requires a verified
  // principal AND a real tenant id. Neither exists in demo mode. Skip
  // the gateway + the post-hire readback verify (no worker row will be
  // there to verify) and return a canonical sample-worker UUID so the
  // wizard redirects cleanly to /workers/<id> and renders a real C2
  // card via the sample-roster fallback in `web/lib/api/workers.ts`.
  if (isDemoMode()) {
    return buildDemoFirstHireResult(req.verticalId);
  }
  const raw = await apiPost<unknown, FirstHirePayload>(
    "/api/v1/onboarding/first-hire",
    { description: req.description, vertical_id: req.verticalId },
  );
  if (!isFirstHireResponse(raw)) {
    throw new ApiError(502, "first-hire endpoint returned an unexpected shape");
  }
  try {
    await verifyWorkerReadable(raw.worker_id);
  } catch (err) {
    if (err instanceof ApiError && err.status === 404) {
      // Defence-in-depth: a 404 here historically meant a gateway "ghost
      // worker" (write/read tenant mismatch). The handler-side fix lives
      // in `routes_onboarding.rs::first_hire_response`, but if the SPA
      // still hits 404 (e.g. a future regression of that same class), try
      // resolving the freshly hired teammate by name against the team
      // roster before surfacing `FIRST_HIRE_VERIFY_FAILED_MESSAGE`.
      const fallbackName = raw.worker_name;
      if (fallbackName) {
        const hit = await findWorkerByName(fallbackName);
        if (hit) {
          return {
            workerId: hit.id,
            workerName: hit.name,
            pending: raw.pending ?? false,
          };
        }
      }
      await emitFirstHireVerifyFailed(raw.worker_id, req.tenantId, req.verticalId);
      throw new ApiError(404, FIRST_HIRE_VERIFY_FAILED_MESSAGE, "first_hire_verify_failed");
    }
    throw err;
  }
  return {
    workerId: raw.worker_id,
    workerName: raw.worker_name ?? "your first worker",
    pending: raw.pending ?? false,
  };
}

export function useFirstHire() {
  return useMutation<FirstHireResult, Error, FirstHireRequest>({
    mutationFn: firstHire,
  });
}
