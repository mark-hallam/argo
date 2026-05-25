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
 * The auth-bypass dev shortcut (NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=1)
 * short-circuits to the mock, matching the rest of the audit feed.
 */
import { apiPost } from "@/lib/api/client";
import { ApiError } from "@/lib/api/types";
import {
  mockGenerate,
  mockPreview,
  type ExportFormat,
  type ExportParams,
  type ExportPreview,
  type ExportResult,
} from "@/lib/sample-export";

/**
 * Wire format the gateway's `POST /api/v1/audit/export` accepts.
 * Mirrors `BundleFormat` in `crates/argo-audit/src/export_bundle.rs`.
 */
type GatewayBundleFormat = "json" | "csv" | "tar";

/**
 * Map the SPA's compliance-shaped `ExportFormat` to the gateway's
 * `BundleFormat`. All four compliance bundles ride the `tar` envelope
 * (`audit.ndjson` + `audit.csv` + `chain_metadata.json` + `verify.sh`);
 * the compliance label stays SPA-side as cosmetic metadata for filename
 * and UI copy. Closes `web-audit-export-format-strings-fix` — the
 * gateway's `parse_format` rejects `soc2-type-ii`/`gdpr-article-15`/
 * `hipaa-disclosure-log`/`generic` with a 400 today.
 */
export function complianceFormatToBundleFormat(
  format: ExportFormat,
): GatewayBundleFormat {
  switch (format) {
    case "soc2-type-ii":
    case "gdpr-article-15":
    case "hipaa-disclosure-log":
    case "generic":
      return "tar";
  }
}

interface GatewayExportRequest {
  scope: ExportParams["scope"];
  tenantId: ExportParams["tenantId"];
  format: GatewayBundleFormat;
}

function toGatewayRequest(params: ExportParams): GatewayExportRequest {
  return {
    scope: params.scope,
    tenantId: params.tenantId,
    format: complianceFormatToBundleFormat(params.format),
  };
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
    process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS === "1"
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
    return await apiPost<ExportPreview, GatewayExportRequest>(
      "/api/v1/audit/export/preview",
      toGatewayRequest(params),
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
    const result = await apiPost<ExportResult, GatewayExportRequest>(
      "/api/v1/audit/export",
      toGatewayRequest(params),
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
