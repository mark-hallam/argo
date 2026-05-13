/**
 * Evidence-bundle export API client. Mirrors the
 * `audit-detail.ts` 404→mock-fallback contract:
 *
 *  - POST `/api/v1/audit/export/preview` → `{ count, sizeEstimate }`
 *  - POST `/api/v1/audit/export` → `{ kind: "immediate" | "queued", ... }`
 *  - POST `/api/v1/audit` → audit-the-audit row (fire-and-forget;
 *    failures are logged, not surfaced to the caller).
 *
 * In strict-production mode (NODE_ENV=production AND
 * NEXT_PUBLIC_ENV=production) the mock fallback is suppressed — a
 * 404 or network error surfaces to the caller so SREs see real
 * gateway breakage rather than mocks that pretend to work.
 *
 * The auth-bypass dev shortcut (NEXT_PUBLIC_GATEWAY_AUTH_BYPASS=1)
 * short-circuits to the mock, matching the rest of the audit feed.
 */
import { apiPost } from "@/lib/api/client";
import { ApiError } from "@/lib/api/types";
import {
  mockGenerate,
  mockPreview,
  type ExportParams,
  type ExportPreview,
  type ExportResult,
} from "@/lib/sample-export";

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

/** Cheap row-count + size estimate for the dialog's preview block. */
export async function previewExport(
  params: ExportParams,
): Promise<ExportPreview> {
  if (isAuthBypassed()) return mockPreview(params);
  try {
    return await apiPost<ExportPreview, ExportParams>(
      "/api/v1/audit/export/preview",
      params,
    );
  } catch (err) {
    if (!isStrictProduction() && isMissingOrUnreachable(err)) {
      return mockPreview(params);
    }
    throw err;
  }
}

/** Kick off bundle generation. Returns immediate-download or queued
 *  envelope depending on bundle size; downloading is the caller's job. */
export async function generateExport(
  params: ExportParams,
): Promise<ExportResult> {
  if (isAuthBypassed()) return withAuditEmission(params, mockGenerate(params));
  try {
    const result = await apiPost<ExportResult, ExportParams>(
      "/api/v1/audit/export",
      params,
    );
    void emitAuditRow(params, result);
    return result;
  } catch (err) {
    if (!isStrictProduction() && isMissingOrUnreachable(err)) {
      return withAuditEmission(params, mockGenerate(params));
    }
    throw err;
  }
}

interface AuditEmissionPayload {
  action: "export_generated";
  metadata: {
    format: ExportParams["format"];
    scope: ExportParams["scope"];
    sizeEstimate: number;
    kind: ExportResult["kind"];
  };
  tenant: string;
}

async function emitAuditRow(
  params: ExportParams,
  result: ExportResult,
): Promise<void> {
  const payload: AuditEmissionPayload = {
    action: "export_generated",
    metadata: {
      format: params.format,
      scope: params.scope,
      sizeEstimate: result.sizeEstimate,
      kind: result.kind,
    },
    tenant: params.tenantId,
  };
  try {
    if (isAuthBypassed()) {
      recordedMockAudit.push(payload);
      return;
    }
    await apiPost<void, AuditEmissionPayload>("/api/v1/audit", payload);
  } catch (err) {
    if (typeof console !== "undefined") {
      console.warn("audit emission failed", err);
    }
  }
}

function withAuditEmission(
  params: ExportParams,
  result: ExportResult,
): ExportResult {
  void emitAuditRow(params, result);
  return result;
}

/** Test-only window into mock-audit emissions. Production code never
 *  reads this; the array is reset by tests via `resetMockAuditLog`. */
export const recordedMockAudit: AuditEmissionPayload[] = [];

export function resetMockAuditLog(): void {
  recordedMockAudit.length = 0;
}
