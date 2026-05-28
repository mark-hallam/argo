"use client";

import { useMutation } from "@tanstack/react-query";
import { apiGet, apiPost, ApiError } from "@/lib/api/client";
import { findWorkerByName } from "@/lib/api/team";
import { appendDemoWorker } from "@/lib/api/demo-roster-store";
import type { WorkerOverview } from "@/lib/api/worker-overview";
import { uuidV7 } from "@/lib/hire/idempotency-key";
import { isDemoMode } from "@/lib/skycot/demo-mode";
import type { Worker } from "@/lib/types/worker";

const VERIFY_ATTEMPTS = 3;
const VERIFY_BACKOFF_MS = 500;

export const FIRST_HIRE_VERIFY_FAILED_MESSAGE =
  "Hiring succeeded — your teammate is provisioning. Refresh in a few seconds; if they don't appear, the Help link in the top right has next steps.";

export interface FirstHireRequest {
  name: string;
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
  name: string;
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
 * Vertical ids the pick-vertical demo grid filters against. F10
 * (#2647) restricted the grid to verticals with a matching sample
 * persona so the camera never landed on a "Sales SDR → Mariah's
 * marketplace-lister charter" vocabulary mismatch. The
 * VERTICAL_TO_SAMPLE_WORKER map that originally derived this set is
 * gone (the demo short-circuit now synthesises a fresh worker from
 * the user's typed name + description), so the set is now a flat
 * literal. Loosening the F10 filter is out of scope for this brief.
 */
export const DEMO_SUPPORTED_VERTICAL_IDS: ReadonlySet<string> = new Set([
  "marketplace-seller",
  "recruiter",
  "customer-support",
  "marketing-intern",
  "project-coordination",
]);

function buildDemoFirstHireOverview(description: string): WorkerOverview {
  // Empty-baseline KPIs: a brand-new hire has no track record yet.
  // Borrowing another worker's numbers would be theater the camera
  // sees on /workers/<id>/overview.
  return {
    charter: description,
    duties: [],
    kpis: [
      { id: "k-actions", label: "actions taken today", value: "0",
        series: [0, 0, 0, 0, 0, 0, 0], deltaPct: 0, trend: "flat", tone: "muted" },
      { id: "k-approvals", label: "approvals awaiting you", value: "0",
        series: [0, 0, 0, 0, 0, 0, 0], deltaPct: 0, trend: "flat", tone: "muted" },
      { id: "k-shadow", label: "shadow-mode hours", value: "0h",
        series: [0, 0, 0, 0, 0, 0, 0], deltaPct: 0, trend: "flat", tone: "muted" },
      { id: "k-onboarding", label: "onboarding progress", value: "Just hired",
        series: [0, 0, 0, 0, 0, 0, 0], deltaPct: 0, trend: "flat", tone: "muted" },
    ],
    recentActivity: [],
  };
}

function buildDemoFirstHireWorker(
  workerId: string,
  name: string,
  description: string,
  verticalId: string,
): Worker {
  return {
    id: workerId,
    name,
    role: { tagline: description, vertical: verticalId },
    status: "idle",
    lastActiveAt: new Date().toISOString(),
  };
}

function buildDemoFirstHireResult(req: FirstHireRequest): FirstHireResult {
  // Per brief `fix-demo-first-hire-discards-prompt-and-redirects-to-
  // sample-worker`: the prior `VERTICAL_TO_SAMPLE_WORKER` map silently
  // discarded the user's typed name + description and routed every
  // vertical to a fixed Mariah/Priya/Jamal/Nori/Peta. The camera saw
  // the wrong name on /workers/<id> and the roster never grew.
  // Synthesize a real-shape UUIDv7 + append the hire to the demo
  // roster so /team + /workers + /workers/<id>/overview all reflect
  // the user's actual input.
  const workerId = uuidV7();
  appendDemoWorker({
    worker: buildDemoFirstHireWorker(workerId, req.name, req.description, req.verticalId),
    overview: buildDemoFirstHireOverview(req.description),
  });
  return { workerId, workerName: req.name, pending: false };
}

export async function firstHire(req: FirstHireRequest): Promise<FirstHireResult> {
  // Demo-mode short-circuit: the first-hire endpoint requires a verified
  // principal AND a real tenant id. Neither exists in demo mode. Skip
  // the gateway + the post-hire readback verify and synthesize a
  // session-local hire keyed off the user's typed name + description;
  // `appendDemoWorker` makes the new worker resolvable from /team and
  // /workers/<id>/overview without a live gateway.
  if (isDemoMode()) {
    return buildDemoFirstHireResult(req);
  }
  const raw = await apiPost<unknown, FirstHirePayload>(
    "/api/v1/onboarding/first-hire",
    { name: req.name, description: req.description, vertical_id: req.verticalId },
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
