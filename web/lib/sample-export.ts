/**
 * Mock evidence-bundle exporter for `/audit` and `/audit/action/<id>`.
 *
 * The real `previewExport` + `generateExport` endpoints live in the
 * gateway behind the existing `mission-compliance-*` rust briefs.
 * This module returns deterministic preview stats + fake download
 * URLs so dev / Storybook / Playwright runs work offline. Bundle
 * generation is a server concern; the UI only consumes the result
 * envelope.
 *
 * Size thresholds line up with the brief acceptance criteria:
 *  - ≤ 50MB → immediate download
 *  - 50-100MB → confirm-then-immediate
 *  - > 100MB → queued (email + push)
 *
 * Mocked size scaling is intentional: row counts scale to push the
 * mock through every size path so Playwright / unit tests can
 * exercise all three branches without server cooperation.
 */

export type ExportFormat =
  | "soc2-type-ii"
  | "gdpr-article-15"
  | "hipaa-disclosure-log"
  | "generic";

export const EXPORT_FORMATS: readonly ExportFormat[] = [
  "soc2-type-ii",
  "gdpr-article-15",
  "hipaa-disclosure-log",
  "generic",
] as const;

export const EXPORT_FORMAT_LABELS: Record<ExportFormat, string> = {
  "soc2-type-ii": "SOC2 Type II",
  "gdpr-article-15": "GDPR Article 15",
  "hipaa-disclosure-log": "HIPAA disclosure log",
  generic: "Generic JSON + PDF",
};

export const EXPORT_FORMAT_DESCRIPTIONS: Record<ExportFormat, string> = {
  "soc2-type-ii":
    "Generates a SOC2 Type II evidence bundle (JSON + PDF + hash chain) suitable for attestation sampling.",
  "gdpr-article-15":
    "Generates a per-user GDPR Article 15 access export (redaction-aware).",
  "hipaa-disclosure-log":
    "Generates a HIPAA disclosure log for sampled PHI access.",
  generic:
    "Generates a documented JSON + PDF pair with verifiable hash chain.",
};

export const DEFAULT_EXPORT_FORMAT: ExportFormat = "soc2-type-ii";

export type ExportScope =
  | { kind: "filtered" }
  | { kind: "range"; from: string; to: string }
  | { kind: "sampled"; size: number }
  | { kind: "single-action"; actionId: string; timestamp: string }
  | { kind: "replay"; replayId: string };

export const SIZE_50MB = 50 * 1024 * 1024;
export const SIZE_100MB = 100 * 1024 * 1024;
const APPROX_BYTES_PER_ROW = 4_000;

export interface ExportParams {
  scope: ExportScope;
  format: ExportFormat;
  tenantId: string;
}

export interface ExportPreview {
  count: number;
  sizeEstimate: number;
}

export interface ImmediateExportResult {
  kind: "immediate";
  url: string;
  fileName: string;
  sizeEstimate: number;
}

export interface QueuedExportResult {
  kind: "queued";
  eta: string;
  sizeEstimate: number;
}

export type ExportResult = ImmediateExportResult | QueuedExportResult;

/** Deterministic row count for a scope. Mock estimates only. */
export function estimateRowCount(scope: ExportScope): number {
  switch (scope.kind) {
    case "filtered":
      return 2_400;
    case "range":
      return rangeRowCount(scope.from, scope.to);
    case "sampled":
      return Math.max(1, Math.floor(scope.size));
    case "single-action":
      return 1;
    case "replay":
      return 1;
  }
}

function rangeRowCount(fromIso: string, toIso: string): number {
  const from = Date.parse(fromIso);
  const to = Date.parse(toIso);
  if (!Number.isFinite(from) || !Number.isFinite(to) || to <= from) {
    return 100;
  }
  const days = Math.max(1, Math.round((to - from) / 86_400_000));
  return Math.min(40_000, days * 1_200);
}

export function mockPreview(params: ExportParams): ExportPreview {
  const count = estimateRowCount(params.scope);
  return { count, sizeEstimate: count * APPROX_BYTES_PER_ROW };
}

export function mockGenerate(params: ExportParams): ExportResult {
  const { count, sizeEstimate } = mockPreview(params);
  if (sizeEstimate > SIZE_100MB) {
    return {
      kind: "queued",
      eta: new Date(Date.now() + 5 * 60_000).toISOString(),
      sizeEstimate,
    };
  }
  const slug = params.format;
  const tag = scopeTag(params.scope);
  return {
    kind: "immediate",
    url: `/mock-exports/${slug}/${tag}-${count}.zip`,
    fileName: `${slug}-${tag}.zip`,
    sizeEstimate,
  };
}

function scopeTag(scope: ExportScope): string {
  switch (scope.kind) {
    case "filtered":
      return "filtered";
    case "range":
      return "range";
    case "sampled":
      return `sample-${Math.max(1, Math.floor(scope.size))}`;
    case "single-action":
      return `action-${scope.actionId}`;
    case "replay":
      return `replay-${scope.replayId}`;
  }
}

/** Format a byte count as `12.4 MB`. Used by preview-stats + confirm. */
export function formatBytes(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes <= 0) return "0 B";
  const units = ["B", "KB", "MB", "GB"];
  let value = bytes;
  let i = 0;
  while (value >= 1024 && i < units.length - 1) {
    value /= 1024;
    i += 1;
  }
  const fixed = value >= 100 ? 0 : value >= 10 ? 1 : 2;
  return `${value.toFixed(fixed)} ${units[i]}`;
}

/** Build a `single-action` scope pre-narrowed to ±1 minute of the
 *  source row's timestamp (per acceptance criteria). */
export function singleActionScope(
  actionId: string,
  timestamp: string,
): ExportScope {
  return { kind: "single-action", actionId, timestamp };
}
