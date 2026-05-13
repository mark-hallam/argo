/**
 * Elevated-view request API client.
 *
 * Wraps `POST /api/v1/audit/elevated-view-request` — the sibling
 * backend brief `mission-gateway-elevated-view-request-endpoint`
 * implements the server side. v1 behavior here:
 *
 *  - In bypass / dev mode (NEXT_PUBLIC_GATEWAY_AUTH_BYPASS=1) the
 *    request is swallowed locally with a console.info; the client
 *    immediately issues a mock grant.
 *  - In a regular env, we attempt the POST. If the gateway returns
 *    404 (endpoint not deployed yet), we degrade gracefully and
 *    still issue the client-side grant — matching the rest of the
 *    audit stack's 404→mock-fallback contract.
 *  - 5xx / network errors surface to the caller.
 *
 * Audit-the-audit: every request is logged via the gateway when
 * available (the gateway emits the audit row on receipt). In v1
 * bypass / 404-fallback paths, a `console.info` substitutes for the
 * audit emission — operators still see the request in the browser
 * console.
 */

import { apiPost } from "@/lib/api/client";
import { ApiError } from "@/lib/api/types";
import type { GrantDuration } from "@/lib/hooks/use-elevated-view-grant";
import type { RbacPermission } from "@/lib/redaction/types";

export const ELEVATED_VIEW_REQUEST_PATH = "/api/v1/audit/elevated-view-request";

export interface ElevatedViewRequest {
  surface: string;
  categories: RbacPermission[];
  reason: string;
  duration: GrantDuration;
}

export interface ElevatedViewRequestResponse {
  ok: true;
  /** Server-issued request id when the endpoint is live; otherwise undefined. */
  requestId?: string;
  /** Operator-approval workflow status — v1.5+. v1 always returns `auto`. */
  status?: "auto" | "queued" | "approved" | "denied";
}

function isAuthBypassed(): boolean {
  return (
    process.env.NODE_ENV !== "production" &&
    process.env.NEXT_PUBLIC_GATEWAY_AUTH_BYPASS === "1"
  );
}

/**
 * Submit an elevated-view request to the gateway. Returns the
 * response (or a mock `{ ok: true }` if the endpoint isn't live).
 * Throws `ApiError` for unexpected 4xx / 5xx — callers must distinguish
 * "endpoint missing" from "request rejected".
 */
export async function submitElevatedViewRequest(
  body: ElevatedViewRequest,
): Promise<ElevatedViewRequestResponse> {
  if (isAuthBypassed()) {
    if (typeof console !== "undefined") {
      console.info("[skycot/audit] elevated-view-request (bypass)", body);
    }
    return { ok: true, status: "auto" };
  }
  try {
    return await apiPost<ElevatedViewRequestResponse, ElevatedViewRequest>(
      ELEVATED_VIEW_REQUEST_PATH,
      body,
    );
  } catch (err) {
    if (err instanceof ApiError && err.status === 404) {
      if (typeof console !== "undefined") {
        console.info(
          "[skycot/audit] elevated-view-request (gateway 404 — client-side grant)",
          body,
        );
      }
      return { ok: true, status: "auto" };
    }
    throw err;
  }
}
