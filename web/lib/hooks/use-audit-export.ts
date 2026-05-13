"use client";

import { useEffect, useState } from "react";
import { useMutation, useQuery, type UseQueryResult } from "@tanstack/react-query";
import { generateExport, previewExport } from "@/lib/api/audit-export";
import type {
  ExportParams,
  ExportPreview,
  ExportResult,
} from "@/lib/sample-export";

const PREVIEW_DEBOUNCE_MS = 300;

/** Stable cache key for a preview request. Hashing the scope as JSON
 *  is fine — params are tiny + serialisable. */
export function exportPreviewQueryKey(params: ExportParams) {
  return [
    "audit-export-preview",
    params.tenantId,
    params.format,
    JSON.stringify(params.scope),
  ] as const;
}

/** Debounced preview-stats hook. Returns the latest preview for the
 *  params after a 300ms quiet period. */
export function useExportPreview(
  params: ExportParams,
  enabled = true,
): UseQueryResult<ExportPreview, Error> {
  const [debounced, setDebounced] = useState(params);
  useEffect(() => {
    const handle = window.setTimeout(() => setDebounced(params), PREVIEW_DEBOUNCE_MS);
    return () => window.clearTimeout(handle);
  }, [params]);

  return useQuery<ExportPreview, Error>({
    queryKey: exportPreviewQueryKey(debounced),
    queryFn: () => previewExport(debounced),
    enabled,
    staleTime: 30_000,
    retry: 0,
  });
}

/** Generate-bundle mutation. Caller wires the on-success branch to
 *  the dialog's size-path UI (immediate download vs queued copy). */
export function useExportGenerate() {
  return useMutation<ExportResult, Error, ExportParams>({
    mutationFn: (params) => generateExport(params),
  });
}
