"use client";

import { Loader2 } from "lucide-react";
import {
  formatBytes,
  SIZE_100MB,
  SIZE_50MB,
  type ExportPreview,
} from "@/lib/sample-export";

/**
 * Read-only preview block beneath the dialog form. Surfaces action
 * count + estimated bundle size + the size-path the user is about
 * to take (immediate, confirm, queued).
 */
export interface ExportPreviewStatsProps {
  preview: ExportPreview | undefined;
  isLoading: boolean;
  isError: boolean;
}

type SizePath = "immediate" | "confirm" | "queued";

function sizePathFor(bytes: number): SizePath {
  if (bytes > SIZE_100MB) return "queued";
  if (bytes > SIZE_50MB) return "confirm";
  return "immediate";
}

const PATH_COPY: Record<SizePath, string> = {
  immediate: "Downloads immediately.",
  confirm: "Will ask to confirm before downloading.",
  queued: "Will be queued — we'll email you when ready.",
};

const PATH_TONE: Record<SizePath, string> = {
  immediate: "text-[var(--muted-foreground)]",
  confirm: "text-[var(--warning)]",
  queued: "text-[var(--warning)]",
};

export function ExportPreviewStats({
  preview,
  isLoading,
  isError,
}: ExportPreviewStatsProps) {
  if (isError) {
    return (
      <div
        data-testid="export-preview-stats-error"
        className="rounded-[var(--radius-sm)] border border-dashed border-[var(--destructive)]/40 bg-[var(--destructive)]/5 px-3 py-2 text-xs text-[var(--foreground)]"
      >
        Couldn&apos;t estimate bundle size.
      </div>
    );
  }
  if (!preview || isLoading) {
    return (
      <div
        data-testid="export-preview-stats-loading"
        className="flex items-center gap-2 rounded-[var(--radius-sm)] border border-[var(--border)] bg-[var(--card)]/60 px-3 py-2 text-xs text-[var(--muted-foreground)]"
      >
        <Loader2 className="size-3 animate-spin" aria-hidden />
        Estimating bundle size…
      </div>
    );
  }
  const path = sizePathFor(preview.sizeEstimate);
  return (
    <div
      data-testid="export-preview-stats"
      data-path={path}
      className="rounded-[var(--radius-sm)] border border-[var(--border)] bg-[var(--card)]/60 px-3 py-2 text-xs"
    >
      <dl className="grid grid-cols-2 gap-y-1">
        <dt className="text-[var(--muted-foreground)]">Actions</dt>
        <dd
          data-testid="export-preview-action-count"
          className="text-right font-medium text-[var(--foreground)]"
        >
          {preview.count.toLocaleString()}
        </dd>
        <dt className="text-[var(--muted-foreground)]">Estimated size</dt>
        <dd
          data-testid="export-preview-size-estimate"
          className="text-right font-medium text-[var(--foreground)]"
        >
          {formatBytes(preview.sizeEstimate)}
        </dd>
      </dl>
      <p
        data-testid="export-preview-path"
        className={`mt-2 ${PATH_TONE[path]}`}
      >
        {PATH_COPY[path]}
      </p>
    </div>
  );
}

export { sizePathFor };
