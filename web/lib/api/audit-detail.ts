/**
 * Audit drill-in API client. Mirrors the `audit.ts` 404→mock-fallback
 * contract; the gateway endpoint `GET /api/v1/audit/<actionId>`
 * arrives via `mission-gateway-audit-detail-endpoint`.
 *
 * In strict-production mode (NODE_ENV=production AND
 * NEXT_PUBLIC_ENV=production) the mock fallback is suppressed — a
 * 404 or network error surfaces to the caller so SREs see real
 * gateway breakage rather than mocks that pretend to work.
 *
 * The auth-bypass dev shortcut (NEXT_PUBLIC_GATEWAY_AUTH_BYPASS=1)
 * short-circuits to the mock, matching the rest of the audit feed.
 */
import { apiGet } from "@/lib/api/client";
import { ApiError } from "@/lib/api/types";
import { findSampleAuditDetail } from "@/lib/sample-audit-detail";
import type { AuditDetail } from "@/lib/types/audit-detail";

export class AuditDetailNotFoundError extends Error {
  actionId: string;
  constructor(actionId: string) {
    super(`Audit action not found: ${actionId}`);
    this.name = "AuditDetailNotFoundError";
    this.actionId = actionId;
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

function mockDetail(actionId: string): AuditDetail {
  const detail = findSampleAuditDetail(actionId);
  if (!detail) throw new AuditDetailNotFoundError(actionId);
  return detail;
}

/**
 * Fetch a single audit-action detail. Throws `AuditDetailNotFoundError`
 * when the action id is unknown (either gateway-404 or mock-miss);
 * surfaces network / 5xx errors to the caller.
 */
export async function fetchAuditDetail(actionId: string): Promise<AuditDetail> {
  if (!actionId) throw new AuditDetailNotFoundError(actionId);

  if (isAuthBypassed()) {
    return mockDetail(actionId);
  }

  const path = `/api/v1/audit/${encodeURIComponent(actionId)}`;
  try {
    return await apiGet<AuditDetail>(path);
  } catch (err) {
    if (err instanceof ApiError && err.status === 404 && isStrictProduction()) {
      throw new AuditDetailNotFoundError(actionId);
    }
    if (!isStrictProduction() && isMissingOrUnreachable(err)) {
      return mockDetail(actionId);
    }
    throw err;
  }
}
