"use client";

import { useMutation } from "@tanstack/react-query";
import { apiPost, ApiError } from "@/lib/api/client";

export interface FirstHireRequest {
  description: string;
  verticalId: string;
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

function isFirstHireResponse(value: unknown): value is FirstHireResponse {
  if (!value || typeof value !== "object") return false;
  const v = value as Record<string, unknown>;
  return typeof v.worker_id === "string" && v.worker_id.length > 0;
}

export async function firstHire(req: FirstHireRequest): Promise<FirstHireResult> {
  const raw = await apiPost<unknown, FirstHirePayload>(
    "/api/v1/onboarding/first-hire",
    { description: req.description, vertical_id: req.verticalId },
  );
  if (!isFirstHireResponse(raw)) {
    throw new ApiError(502, "first-hire endpoint returned an unexpected shape");
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
