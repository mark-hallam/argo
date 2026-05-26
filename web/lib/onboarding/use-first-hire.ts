"use client";

import { useMutation } from "@tanstack/react-query";
import { apiGet, apiPost, ApiError } from "@/lib/api/client";

const VERIFY_ATTEMPTS = 3;
const VERIFY_BACKOFF_MS = 500;

export const FIRST_HIRE_VERIFY_FAILED_MESSAGE =
  "Hiring succeeded but your teammate isn't visible yet. Try refreshing or contact support.";

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

export async function firstHire(req: FirstHireRequest): Promise<FirstHireResult> {
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
