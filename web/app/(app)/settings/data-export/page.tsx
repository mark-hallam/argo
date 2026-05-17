"use client";

import { useCallback, useState } from "react";
import { useExportRequest } from "@/lib/data-export/use-export-request";
import { apiGet, ApiError } from "@/lib/api/client";
import type { ExportDownloadResponse } from "@/lib/data-export/use-export-request";

export default function DataExportPage() {
  const xport = useExportRequest();
  const [downloading, setDownloading] = useState(false);
  const [downloadError, setDownloadError] = useState<string | null>(null);

  const onDownload = useCallback(async () => {
    if (!xport.downloadUrl) return;
    setDownloading(true);
    setDownloadError(null);
    try {
      const resp = await apiGet<ExportDownloadResponse>(xport.downloadUrl);
      const bytes = Uint8Array.from(atob(resp.body_b64), (c) => c.charCodeAt(0));
      const blob = new Blob([bytes], { type: resp.content_type });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = resp.filename;
      document.body.appendChild(a);
      a.click();
      a.remove();
      URL.revokeObjectURL(url);
    } catch (e) {
      const msg = e instanceof ApiError ? e.message : e instanceof Error ? e.message : "download failed";
      setDownloadError(msg);
    } finally {
      setDownloading(false);
    }
  }, [xport.downloadUrl]);

  return (
    <div className="mx-auto max-w-2xl px-4 sm:px-6 lg:px-8 py-8 space-y-6">
      <header className="space-y-1">
        <h1 className="text-xl font-semibold tracking-tight text-[var(--foreground)]">
          Download my data
        </h1>
        <p className="text-sm text-[var(--muted-foreground)]">
          Export everything we hold about your account — your profile, the
          tenants you belong to, audit rows attributed to you, and outbound
          messages from tenants you own. Per GDPR Article 15 / CCPA right-to-know,
          you can request one export every 24 hours.
        </p>
      </header>

      <section
        data-testid="data-export-panel"
        className="rounded-[var(--radius-lg)] border border-[var(--border)] bg-[var(--muted)]/30 p-5 space-y-4"
      >
        <ul className="text-xs text-[var(--muted-foreground)] space-y-1 list-disc pl-4">
          <li><code>user_account.json</code> — your profile row</li>
          <li><code>tenants.json</code> — workspaces you belong to + your role</li>
          <li><code>workers.jsonl</code> — workers in tenants you own</li>
          <li><code>audit/chain.jsonl</code> — audit rows attributed to you</li>
          <li><code>messages/outbound.jsonl</code> — outbound messages from tenants you own</li>
          <li><code>manifest.json</code> — version, scope, and known limitations</li>
        </ul>

        {xport.status === "idle" ? (
          <button
            type="button"
            onClick={() => void xport.request()}
            data-testid="data-export-request"
            className="rounded-[var(--radius-md)] bg-[var(--primary)] px-4 py-2 text-sm font-medium text-[var(--primary-foreground)] hover:opacity-90 focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-[var(--ring)]"
          >
            Request export
          </button>
        ) : null}

        {xport.status === "requesting" || xport.status === "polling" ? (
          <p data-testid="data-export-pending" className="text-sm text-[var(--muted-foreground)]">
            Assembling your data… this usually takes a few seconds.
          </p>
        ) : null}

        {xport.status === "ready" && xport.downloadUrl ? (
          <div className="space-y-2">
            <p className="text-sm text-[var(--foreground)]">
              Your export is ready. The download link expires in 7 days.
            </p>
            <button
              type="button"
              onClick={() => void onDownload()}
              disabled={downloading}
              data-testid="data-export-download"
              className="rounded-[var(--radius-md)] bg-[var(--primary)] px-4 py-2 text-sm font-medium text-[var(--primary-foreground)] hover:opacity-90 disabled:opacity-50 focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-[var(--ring)]"
            >
              {downloading ? "Downloading…" : "Download .tar.gz"}
            </button>
            {downloadError ? (
              <p role="alert" data-testid="data-export-download-error" className="text-sm text-[var(--destructive)]">
                {downloadError}
              </p>
            ) : null}
          </div>
        ) : null}

        {xport.status === "rate_limited" ? (
          <p data-testid="data-export-rate-limited" role="alert" className="text-sm text-[var(--destructive)]">
            You already requested an export in the last 24 hours. Try again later.
          </p>
        ) : null}

        {xport.status === "error" ? (
          <p data-testid="data-export-error" role="alert" className="text-sm text-[var(--destructive)]">
            Export failed: {xport.error ?? "unknown error"}
          </p>
        ) : null}
      </section>
    </div>
  );
}
