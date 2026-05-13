"use client";

import * as DialogPrimitive from "@radix-ui/react-dialog";
import { Download, X } from "lucide-react";
import { useCallback, useMemo, useState } from "react";
import { Button } from "@/components/ui/Button";
import { cn } from "@/lib/utils";
import { useExportGenerate, useExportPreview } from "@/lib/hooks/use-audit-export";
import {
  DEFAULT_EXPORT_FORMAT,
  formatBytes,
  SIZE_100MB,
  SIZE_50MB,
  type ExportFormat,
  type ExportParams,
  type ExportResult,
  type ExportScope,
} from "@/lib/sample-export";
import { ExportFormatSelector } from "@/components/skycot/audit/ExportFormatSelector";
import {
  ExportScopeSelector,
  PRIMARY_SCOPE_KINDS,
} from "@/components/skycot/audit/ExportScopeSelector";
import { ExportPreviewStats } from "@/components/skycot/audit/ExportPreviewStats";

/**
 * `[ ⤓ Export ]` dialog for the audit timeline. Wires the existing
 * compliance-export infrastructure (`mission-compliance-soc2-audit-
 * trail` MERGED + `mission-compliance-gdpr-data-export` MERGED + the
 * HIPAA + Generic generators in flight) into a single dialog that
 * the audit page (E5-a) and action detail (E5-c) — and later the
 * replay surface (E5-d) — can mount.
 *
 * Three size-path branches are baked into the on-Export handler:
 *  - ≤ 50MB    → immediate download via `<a download>` + toast
 *  - 50-100MB  → confirm prompt → immediate download + toast
 *  - > 100MB   → queued mode: dialog body replaced with a
 *                "we'll email you" message + Done button.
 *
 * The trigger button + audit-the-audit row are owned by this
 * component; the caller passes initial scope (which can lock to
 * single-action / replay mode) and tenant id.
 */

export type ExporterMode = "bulk" | "single-action" | "replay";

export interface EvidenceBundleExporterProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  tenantId: string;
  tenantLabel: string;
  initialScope: ExportScope;
  initialFormat?: ExportFormat;
  mode?: ExporterMode;
  /** Toast callback — caller renders the actual toast surface. */
  onToast: (message: string) => void;
}

type PendingConfirm = { params: ExportParams; sizeEstimate: number } | null;
type QueuedState = { eta: string; sizeEstimate: number } | null;

export function EvidenceBundleExporter({
  open,
  onOpenChange,
  tenantId,
  tenantLabel,
  initialScope,
  initialFormat = DEFAULT_EXPORT_FORMAT,
  mode = "bulk",
  onToast,
}: EvidenceBundleExporterProps) {
  const [scope, setScope] = useState<ExportScope>(initialScope);
  const [format, setFormat] = useState<ExportFormat>(initialFormat);
  const [pendingConfirm, setPendingConfirm] = useState<PendingConfirm>(null);
  const [queued, setQueued] = useState<QueuedState>(null);

  const isLockedScope =
    initialScope.kind === "single-action" || initialScope.kind === "replay";

  const params = useMemo<ExportParams>(
    () => ({ scope, format, tenantId }),
    [scope, format, tenantId],
  );
  const previewQuery = useExportPreview(params, open && !queued);
  const generate = useExportGenerate();

  const reset = useCallback(() => {
    setScope(initialScope);
    setFormat(initialFormat);
    setPendingConfirm(null);
    setQueued(null);
    generate.reset();
  }, [generate, initialFormat, initialScope]);

  const handleOpenChange = useCallback(
    (next: boolean) => {
      if (!next) reset();
      onOpenChange(next);
    },
    [onOpenChange, reset],
  );

  const triggerExport = useCallback(
    (finalParams: ExportParams) => {
      generate.mutate(finalParams, {
        onSuccess: (result) => handleResult(result),
        onError: () => onToast("Export failed. Try again."),
      });
    },
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [generate, onToast],
  );

  const handleResult = useCallback(
    (result: ExportResult) => {
      if (result.kind === "queued") {
        setQueued({ eta: result.eta, sizeEstimate: result.sizeEstimate });
        onToast("Export queued. We'll email you when it's ready.");
        return;
      }
      triggerDownload(result.url, result.fileName);
      onToast(`Export ready — ${formatBytes(result.sizeEstimate)}.`);
      onOpenChange(false);
      reset();
    },
    [onOpenChange, onToast, reset],
  );

  const onExportClick = useCallback(() => {
    if (!previewQuery.data) return;
    const sizeEstimate = previewQuery.data.sizeEstimate;
    if (sizeEstimate > SIZE_50MB && sizeEstimate <= SIZE_100MB) {
      setPendingConfirm({ params, sizeEstimate });
      return;
    }
    triggerExport(params);
  }, [params, previewQuery.data, triggerExport]);

  const confirmAndExport = useCallback(() => {
    if (!pendingConfirm) return;
    const next = pendingConfirm.params;
    setPendingConfirm(null);
    triggerExport(next);
  }, [pendingConfirm, triggerExport]);

  const canSubmit =
    !generate.isPending &&
    previewQuery.data !== undefined &&
    previewQuery.data.count > 0;

  return (
    <DialogPrimitive.Root open={open} onOpenChange={handleOpenChange}>
      <DialogPrimitive.Portal>
        <DialogPrimitive.Overlay
          className="fixed inset-0 z-40 bg-black/40 backdrop-blur-sm"
          data-testid="export-dialog-overlay"
        />
        <DialogPrimitive.Content
          aria-modal="true"
          data-testid="export-dialog"
          data-mode={mode}
          className={cn(
            "fixed left-1/2 top-1/2 z-50 w-[92vw] max-w-lg -translate-x-1/2 -translate-y-1/2",
            "rounded-[var(--radius-lg)] border border-[var(--border)] bg-[var(--background)] p-6 shadow-lg",
            "focus:outline-none",
          )}
        >
          <DialogHeader />

          {queued ? (
            <QueuedBody queued={queued} onDone={() => handleOpenChange(false)} />
          ) : pendingConfirm ? (
            <ConfirmBody
              sizeEstimate={pendingConfirm.sizeEstimate}
              onCancel={() => setPendingConfirm(null)}
              onConfirm={confirmAndExport}
            />
          ) : (
            <FormBody
              scope={scope}
              format={format}
              tenantLabel={tenantLabel}
              isLockedScope={isLockedScope}
              previewQuery={previewQuery}
              canSubmit={canSubmit}
              isExporting={generate.isPending}
              onScopeChange={setScope}
              onFormatChange={setFormat}
              onCancel={() => handleOpenChange(false)}
              onExport={onExportClick}
            />
          )}
        </DialogPrimitive.Content>
      </DialogPrimitive.Portal>
    </DialogPrimitive.Root>
  );
}

function DialogHeader() {
  return (
    <div className="flex items-start justify-between gap-4">
      <div>
        <DialogPrimitive.Title className="text-base font-semibold text-[var(--foreground)]">
          Export evidence bundle
        </DialogPrimitive.Title>
        <DialogPrimitive.Description className="mt-1 text-xs text-[var(--muted-foreground)]">
          Signed, redaction-aware audit export with a verifiable hash chain.
        </DialogPrimitive.Description>
      </div>
      <DialogPrimitive.Close
        aria-label="Close"
        data-testid="export-dialog-close"
        className="rounded-[var(--radius-sm)] p-1 text-[var(--muted-foreground)] hover:bg-[var(--accent)]/40 hover:text-[var(--foreground)]"
      >
        <X aria-hidden className="size-4" />
      </DialogPrimitive.Close>
    </div>
  );
}

interface FormBodyProps {
  scope: ExportScope;
  format: ExportFormat;
  tenantLabel: string;
  isLockedScope: boolean;
  previewQuery: ReturnType<typeof useExportPreview>;
  canSubmit: boolean;
  isExporting: boolean;
  onScopeChange: (next: ExportScope) => void;
  onFormatChange: (next: ExportFormat) => void;
  onCancel: () => void;
  onExport: () => void;
}

function FormBody({
  scope,
  format,
  tenantLabel,
  isLockedScope,
  previewQuery,
  canSubmit,
  isExporting,
  onScopeChange,
  onFormatChange,
  onCancel,
  onExport,
}: FormBodyProps) {
  return (
    <div className="mt-5 space-y-4">
      <ExportScopeSelector
        scope={scope}
        onChange={onScopeChange}
        disabled={isLockedScope || isExporting}
      />
      <ExportFormatSelector
        value={format}
        onChange={onFormatChange}
        disabled={isExporting}
      />
      <TenantSummary tenantLabel={tenantLabel} />
      <ExportPreviewStats
        preview={previewQuery.data}
        isLoading={previewQuery.isLoading}
        isError={previewQuery.isError}
      />
      <div className="mt-2 flex items-center justify-end gap-2">
        <Button
          variant="ghost"
          data-testid="export-dialog-cancel"
          onClick={onCancel}
        >
          Cancel
        </Button>
        <Button
          variant="primary"
          data-testid="export-dialog-submit"
          onClick={onExport}
          disabled={!canSubmit}
          loading={isExporting}
        >
          <Download aria-hidden className="size-4" />
          Export
        </Button>
      </div>
    </div>
  );
}

function TenantSummary({ tenantLabel }: { tenantLabel: string }) {
  return (
    <div
      data-testid="export-dialog-tenant"
      className="flex items-center justify-between rounded-[var(--radius-sm)] border border-[var(--border)] bg-[var(--card)]/40 px-3 py-2 text-xs"
    >
      <span className="font-medium uppercase tracking-wide text-[var(--muted-foreground)]">
        Tenant
      </span>
      <span className="font-medium text-[var(--foreground)]">{tenantLabel}</span>
    </div>
  );
}

function ConfirmBody({
  sizeEstimate,
  onCancel,
  onConfirm,
}: {
  sizeEstimate: number;
  onCancel: () => void;
  onConfirm: () => void;
}) {
  return (
    <div className="mt-5 space-y-4" data-testid="export-dialog-confirm">
      <p className="text-sm text-[var(--foreground)]">
        This export is ~{formatBytes(sizeEstimate)}. Continue?
      </p>
      <div className="flex justify-end gap-2">
        <Button variant="ghost" onClick={onCancel} data-testid="export-confirm-cancel">
          Cancel
        </Button>
        <Button
          variant="primary"
          onClick={onConfirm}
          data-testid="export-confirm-continue"
        >
          Continue
        </Button>
      </div>
    </div>
  );
}

function QueuedBody({
  queued,
  onDone,
}: {
  queued: NonNullable<QueuedState>;
  onDone: () => void;
}) {
  const eta = queuedEtaLabel(queued.eta);
  return (
    <div className="mt-5 space-y-4" data-testid="export-dialog-queued">
      <p className="text-sm text-[var(--foreground)]">
        Your export is being prepared. We&apos;ll email you when it&apos;s ready (~{eta}).
      </p>
      <p className="text-xs text-[var(--muted-foreground)]">
        Estimated size: {formatBytes(queued.sizeEstimate)}.
      </p>
      <div className="flex justify-end">
        <Button variant="primary" onClick={onDone} data-testid="export-dialog-done">
          Done
        </Button>
      </div>
    </div>
  );
}

function queuedEtaLabel(iso: string): string {
  const t = Date.parse(iso);
  if (!Number.isFinite(t)) return "a few minutes";
  const minutes = Math.max(1, Math.round((t - Date.now()) / 60_000));
  return `${minutes} min`;
}

function triggerDownload(url: string, fileName: string): void {
  if (typeof document === "undefined") return;
  const a = document.createElement("a");
  a.href = url;
  a.download = fileName;
  a.rel = "noopener";
  a.setAttribute("data-testid", "export-download-link");
  document.body.appendChild(a);
  a.click();
  a.remove();
}

export { PRIMARY_SCOPE_KINDS };
